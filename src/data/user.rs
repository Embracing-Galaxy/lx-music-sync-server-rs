use crate::{
    data::config::AddMusicLocation,
    data::{list::ListData, list::ListDataManager, ClientId},
    utils::{
        crypto::{rand_16bytes_as_base64, MD5},
        de_rwlock, filter_file_name, ser_rwlock,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;

pub(crate) static USERS_PATH: LazyLock<&Path> = LazyLock::new(|| Path::new("users"));

pub(crate) struct UserSpace {
    user_data: UserData,
    list: RwLock<ListDataManager>,
    #[allow(unused)]
    dislike: RwLock<ListDataManager>,
}

impl UserSpace {
    pub(crate) fn new(user_name: String) -> Self {
        let user_data = UserData::new(user_name);
        let path = &user_data.user_path;
        Self {
            list: RwLock::new(ListDataManager::new(&path.join("list"))),
            dislike: RwLock::new(ListDataManager::new(&path.join("dislike"))),
            user_data,
        }
    }

    pub(crate) async fn get_client_device_info(&self, id: &str) -> Option<Arc<DeviceInfo>> {
        self.user_data
            .devices_infos
            .clients
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub(crate) async fn update_device_name(&self, id: &ClientId, device_name: &str) {
        let clients = &self.user_data.devices_infos.clients.read().await;
        let device_info = clients.get(id).unwrap();
        if *device_info.device_name.read().await != device_name {
            *device_info.device_name.write().await = device_name.to_string();
            self.user_data.write_devices_infos();
        }
    }

    pub(crate) async fn insert_device_info(&self, device_info: DeviceInfo) {
        self.user_data
            .devices_infos
            .clients
            .write()
            .await
            .insert(device_info.client_id.clone(), Arc::new(device_info));
        self.user_data.write_devices_infos();
    }

    pub(crate) async fn merge_list(
        &self,
        client_id: &ClientId,
        client: &ListData,
        snapshot: &ListData,
        add_location: &AddMusicLocation,
    ) -> Vec<u8> {
        self.list
            .write()
            .await
            .merge(client_id, client, snapshot, add_location)
    }

    pub(crate) async fn overwrite_list(&self, client_id: &ClientId, data: ListData) {
        self.list.write().await.overwrite(client_id, data);
    }

    /// Get the snapshot of the last sync of the given client
    pub(crate) async fn get_snapshot(&self, client_id: &ClientId) -> Option<ListData> {
        self.list
            .read()
            .await
            .get_snapshot(&self.user_data.user_path, &client_id)
    }

    pub(crate) async fn get_current_list_info_key(&self) -> MD5 {
        self.list.read().await.get_info_key()
    }

    pub(crate) async fn get_snapshot_key(&self, client_id: &ClientId) -> Option<MD5> {
        self.list.read().await.get_snapshot_key(client_id).cloned()
    }

    pub(crate) async fn update_snapshot_key(&self, client_id: &ClientId, key: MD5) {
        self.list.write().await.update_snapshot_key(client_id, key);
    }
}

struct UserData {
    user_path: Box<Path>,
    devices_file_path: Arc<Path>,
    devices_infos: Arc<DevicesInfos>,
}

impl UserData {
    fn new(username: String) -> Self {
        let dir = USERS_PATH.join(filter_file_name(&username));
        let devices_file_path = dir.join("devices.json");
        Self {
            devices_infos: Arc::new(DevicesInfos::load(&devices_file_path)),
            user_path: dir.into_boxed_path(),
            devices_file_path: Arc::from(devices_file_path),
        }
    }

    /// Write `devices_infos` to file without await
    fn write_devices_infos(&self) {
        async_scoped::TokioScope::scope_and_block(|scope| {
            scope.spawn(async {
                let data = self.devices_infos.serialize().await;
                tokio::fs::write(&self.devices_file_path, data)
                    .await
                    .unwrap();
            })
        });
    }
}

pub(crate) struct DevicesInfos {
    pub(crate) username: String,
    clients: RwLock<HashMap<ClientId, Arc<DeviceInfo>>>,
}

impl DevicesInfos {
    pub(crate) fn load(path: impl AsRef<Path>) -> Self {
        let deserialized: Helper = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

        Self {
            username: deserialized.username,
            clients: RwLock::new(
                deserialized
                    .clients
                    .into_iter()
                    .map(|(k, v)| (k, Arc::new(v)))
                    .collect(),
            ),
        }
    }

    pub(crate) async fn register_each_device(
        &self,
        device_user_map: &mut HashMap<ClientId, String>,
    ) {
        for client_id in self.clients.read().await.keys() {
            device_user_map.insert(client_id.clone(), self.username.clone());
        }
    }

    async fn serialize(&self) -> Vec<u8> {
        let guard = self.clients.read().await;
        let infos: Vec<_> = guard.values().map(AsRef::as_ref).collect();
        serde_json::to_vec(&infos).unwrap()
    }
}

#[derive(Deserialize)]
struct Helper {
    username: String,
    clients: HashMap<ClientId, DeviceInfo>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct DeviceInfo {
    pub(crate) client_id: ClientId,
    pub(crate) key: String,
    #[serde(deserialize_with = "de_rwlock", serialize_with = "ser_rwlock")]
    pub(crate) device_name: RwLock<String>,
    pub(crate) is_mobile: bool,
    last_connect_date: Option<usize>,
}

impl DeviceInfo {
    pub(crate) fn new(device_name: String, is_mobile: bool) -> Self {
        Self {
            client_id: rand_16bytes_as_base64(),
            key: rand_16bytes_as_base64(),
            device_name: RwLock::new(device_name),
            is_mobile,
            last_connect_date: Some(0),
        }
    }
}
