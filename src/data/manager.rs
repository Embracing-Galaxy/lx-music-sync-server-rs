use crate::data::{ClientId, config::AddMusicLocation};
use crate::server::socket::handler::JsonReqHandler;
use crate::utils::crypto::{MD5, md5_to_hex, to_md5};
use crate::utils::load_or_create;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

#[derive(Clone, Copy, Debug)]
pub(crate) enum DataType {
    LIST,
    DISLIKE,
}

pub(crate) struct DataManager<DATA: Data + Send + Sync> {
    path: Box<Path>,
    info_path: &'static Path,
    snapshot_info: RwLock<SnapshotInfo>,
    current_data: RwLock<DATA>,
    info_last_save: Mutex<Instant>,
}

const DEBOUNCE: Duration = Duration::from_secs(30);
const MIN_INTERVAL: Duration = Duration::from_secs(120);

impl<DATA: Data + Send + Sync> DataManager<DATA> {
    pub(super) fn new(path: &Path) -> Self {
        let info_path = path.join("snapshotInfo.json").leak();
        let snapshot_info: SnapshotInfo = load_or_create(info_path);
        let current_list_data = match snapshot_info.latest_key {
            None => DATA::default(),
            Some(key) => Self::get_snapshot_from_key(path, key).unwrap_or_default(),
        };
        Self {
            current_data: RwLock::new(current_list_data),
            path: path.into(),
            info_path,
            snapshot_info: RwLock::new(snapshot_info),
            info_last_save: Mutex::new(Instant::now() - MIN_INTERVAL),
        }
    }

    /// Get the snapshot of the last sync of the given client
    pub(crate) async fn get_snapshot(&self, client_id: &ClientId) -> Option<DATA> {
        let key = self.get_snapshot_key(client_id).await?;
        Self::get_snapshot_from_key(&self.path, key)
    }

    fn get_snapshot_from_key(user_path: &Path, key: SnapshotKey) -> Option<DATA> {
        let bytes = std::fs::read(user_path.join(md5_to_hex(key))).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub(crate) async fn get_info_key(&'static self) -> SnapshotKey {
        match self.snapshot_info.read().await.latest_key {
            None => self.save_snapshot().await, // latest_key would update inside the `save_snapshot`
            Some(latest) => latest,
        }
    }

    pub(crate) async fn get_snapshot_key(&self, client_id: &ClientId) -> Option<SnapshotKey> {
        self.snapshot_info
            .read()
            .await
            .clients
            .get(client_id)
            .copied()
    }

    async fn create_snapshot(&'static self, bytes: Vec<u8>) -> SnapshotKey {
        let key = to_md5(&bytes);
        if self.snapshot_info.read().await.latest_key == Some(key) {
            return key;
        }
        self.snapshot_info.write().await.latest_key = Some(key);
        // async write snapshot content
        let path = self.path.join(md5_to_hex(key));
        tokio::spawn(tokio::fs::write(path, bytes));

        self.write_snapshot_info().await;
        key
    }

    async fn save_snapshot(&'static self) -> SnapshotKey {
        let bytes = serde_json::to_vec(&*self.current_data.read().await).unwrap();
        self.create_snapshot(bytes).await
    }

    pub(crate) async fn merge(
        &'static self,
        client_id: &ClientId,
        client: &DATA,
        snapshot: &DATA,
        add_location: &AddMusicLocation,
    ) -> (Vec<u8>, SnapshotKey) {
        self.current_data
            .write()
            .await
            .merge(client, snapshot, add_location);
        let bytes = serde_json::to_vec(&*self.current_data.read().await).unwrap();
        self.update_snapshot_key(client_id, to_md5(&bytes)).await;
        (bytes.clone(), self.create_snapshot(bytes).await)
    }

    pub(crate) async fn overwrite_from_client(&'static self, client_id: &ClientId, data: DATA) {
        if data.is_empty() {
            return;
        }
        let bytes = serde_json::to_vec(&data).unwrap();
        self.update_snapshot_key(client_id, to_md5(&bytes)).await;
        *self.current_data.write().await = data;
        self.create_snapshot(bytes).await;
    }

    pub(crate) async fn on_sync(&'static self, action: serde_json::Value) -> SnapshotKey {
        self.current_data.write().await.on(action);
        self.save_snapshot().await
    }

    pub(crate) async fn update_snapshot_key(&'static self, client_id: &ClientId, key: SnapshotKey) {
        let clients = &mut self.snapshot_info.write().await.clients;
        if let Some(old_key) = clients.insert(client_id.clone(), key)
            && !clients.values().any(|key| *key == old_key)
        {
            tokio::spawn(tokio::fs::remove_file(self.path.join(md5_to_hex(old_key))));
        }
        self.write_snapshot_info().await;
    }

    async fn write_snapshot_info(&'static self) {
        let last_save = self.info_last_save.lock().await;
        let now = Instant::now();
        let duration = now.duration_since(*last_save);
        if duration < MIN_INTERVAL {
            return;
        }
        drop(last_save);
        tokio::spawn(self.write_snapshot_info_debounce());
    }

    async fn write_snapshot_info_debounce(&'static self) {
        sleep(DEBOUNCE).await;
        let mut last_save = self.info_last_save.lock().await;
        let info = serde_json::to_vec_pretty(&*self.snapshot_info.read().await).unwrap();
        tokio::fs::write(self.info_path, info).await.unwrap();
        *last_save = Instant::now();
    }
}

pub(crate) trait Data: Default + Serialize + DeserializeOwned + JsonReqHandler {
    fn is_empty(&self) -> bool;
    fn merge(&mut self, client: &Self, snapshot: &Self, add_location: &AddMusicLocation);
}

pub(crate) type SnapshotKey = MD5;

#[derive(Default, Deserialize, Serialize)]
struct SnapshotInfo {
    latest_key: Option<SnapshotKey>,
    clients: HashMap<ClientId, SnapshotKey>,
}

impl SnapshotInfo {}
