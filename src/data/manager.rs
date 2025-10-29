use crate::data::{config::AddMusicLocation, ClientId, SnapshotInfo, SnapshotKey};
use crate::server::socket::handler::JsonReqHandler;
use crate::utils::crypto::{md5_to_hex, to_md5};
use crate::utils::load_or_create;
use log::debug;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug)]
pub(crate) enum DataType {
    LIST,
    DISLIKE,
}

pub(crate) struct DataManager<DATA: Data> {
    path: Box<Path>,
    info_path: &'static Path,
    snapshot_info: RwLock<SnapshotInfo>,
    current_data: RwLock<DATA>,
}

impl<DATA: Data> DataManager<DATA> {
    pub(super) fn new(path: &Path) -> Self {
        let info_path = path.join("snapshotInfo.json").leak();
        let snapshot_info: SnapshotInfo = load_or_create(info_path);
        let current_list_data = match snapshot_info.latest_key {
            None => DATA::default(),
            Some(key) => Self::get_snapshot_from_key(&path, key).unwrap_or_default(),
        };
        Self {
            current_data: RwLock::new(current_list_data),
            path: path.into(),
            info_path,
            snapshot_info: RwLock::new(snapshot_info),
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

    pub(crate) async fn get_info_key(&self) -> SnapshotKey {
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
            .cloned()
    }

    async fn create_snapshot(&self, bytes: Vec<u8>) -> SnapshotKey {
        let key = to_md5(&bytes);
        if let Some(latest_key) = self.snapshot_info.read().await.latest_key
            && latest_key == key
        {
            return key;
        }

        if self.snapshot_info.write().await.try_insert_key(key) {
            // async write snapshot content
            let path = self.path.join(md5_to_hex(key));
            tokio::spawn(tokio::fs::write(path, bytes));
        };
        self.write_snapshot_info().await;
        key
    }

    async fn save_snapshot(&self) -> SnapshotKey {
        let bytes = serde_json::to_vec(&*self.current_data.read().await).unwrap();
        self.create_snapshot(bytes).await
    }

    pub(crate) async fn merge(
        &self,
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

    pub(crate) async fn overwrite_from_client(&self, client_id: &ClientId, data: DATA) {
        if data.is_empty() {
            return;
        }
        let bytes = serde_json::to_vec(&data).unwrap();
        self.update_snapshot_key(client_id, to_md5(&bytes)).await;
        *self.current_data.write().await = data;
        self.create_snapshot(bytes).await;
    }

    pub(crate) async fn on_sync(&self, action: serde_json::Value) -> SnapshotKey {
        debug!("get action: {action}");
        self.current_data.write().await.on(action);
        self.save_snapshot().await
    }

    pub(crate) async fn update_snapshot_key(&self, client_id: &ClientId, key: SnapshotKey) {
        self.snapshot_info.write().await.update(client_id, key);
        self.write_snapshot_info().await;
    }

    async fn write_snapshot_info(&self) {
        // TODO throttle
        let info = serde_json::to_vec_pretty(&*self.snapshot_info.read().await).unwrap();
        tokio::spawn(tokio::fs::write(self.info_path, info));
    }
}

pub(crate) trait Data: Default + Serialize + DeserializeOwned + JsonReqHandler {
    fn is_empty(&self) -> bool;
    fn merge(&mut self, client: &Self, snapshot: &Self, add_location: &AddMusicLocation);
}
