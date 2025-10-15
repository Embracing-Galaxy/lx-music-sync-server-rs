use crate::data::Username;
use crate::server::SERVER_CONTEXT;
use crate::utils::load_or_create;
use crate::{
    data::config::AddMusicLocation,
    data::{list::ListData, list::ListDataManager, ClientId},
    utils::{
        crypto::{rand_16bytes_as_base64, MD5},
        filter_file_name,
    },
};
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub(crate) fn new(
        user_name: Username,
        devices_infos: DevicesInfos,
        devices_file_path: Arc<Path>,
    ) -> Self {
        let user_data = UserData::new(user_name, devices_infos, devices_file_path);
        let path = &user_data.user_path;
        Self {
            list: RwLock::new(ListDataManager::new(&path.join("list"))),
            dislike: RwLock::new(ListDataManager::new(&path.join("dislike"))),
            user_data,
        }
    }

    pub(crate) async fn get_client_device_info(&self, id: &str) -> Option<Arc<DeviceInfo>> {
        self.user_data.devices_infos.clients.load().get(id).cloned()
    }

    pub(crate) async fn update_device_name(&self, id: &ClientId, device_name: &str) {
        let guard = self.user_data.devices_infos.clients.load();
        let device_info = guard.get(id).unwrap();
        if device_info.device_name.load().as_str() != device_name {
            // assert only one socket would visit here
            device_info
                .device_name
                .store(Arc::new(device_name.to_string()));
            self.user_data.write_devices_infos();
        }
    }

    pub(crate) fn insert_device_info(&self, device_info: DeviceInfo) {
        SERVER_CONTEXT
            .update_device_username_map(device_info.client_id.clone(), self.user_data.username);
        self.user_data.devices_infos.insert_device(device_info);
        self.user_data.write_devices_infos();
    }

    pub(crate) async fn merge_list(
        &self,
        client_id: &ClientId,
        client: &ListData,
        snapshot: &ListData,
        add_location: &AddMusicLocation,
    ) -> Vec<u8> {
        let merged_bytes = self
            .list
            .write()
            .await
            .merge(client_id, client, snapshot, add_location);
        self.list.write().await.create_snapshot();
        merged_bytes
    }

    pub(crate) async fn overwrite_list(&self, client_id: &ClientId, data: ListData) {
        self.list.write().await.overwrite(client_id, data);
    }

    /// Get the snapshot of the last sync of the given client
    pub(crate) async fn get_snapshot(&self, client_id: &ClientId) -> Option<ListData> {
        self.list.read().await.get_snapshot(&client_id)
    }

    pub(crate) async fn get_current_list_info_key(&self) -> MD5 {
        self.list.write().await.get_info_key()
    }

    pub(crate) async fn get_snapshot_key(&self, client_id: &ClientId) -> Option<MD5> {
        self.list.read().await.get_snapshot_key(client_id).cloned()
    }

    pub(crate) async fn update_snapshot_key(&self, client_id: &ClientId, key: MD5) {
        self.list.write().await.update_snapshot_key(client_id, key);
    }
}

struct UserData {
    username: Username,
    user_path: Box<Path>,
    devices_file_path: Arc<Path>,
    devices_infos: Arc<DevicesInfos>,
}

impl UserData {
    pub(crate) fn new(
        username: Username,
        device_infos: DevicesInfos,
        devices_file_path: Arc<Path>,
    ) -> Self {
        let dir = USERS_PATH.join(filter_file_name(username));
        Self {
            username,
            devices_infos: Arc::new(device_infos),
            user_path: dir.into_boxed_path(),
            devices_file_path,
        }
    }

    /// Write `devices_infos` to file without await
    fn write_devices_infos(&self) {
        let path = self.devices_file_path.clone();
        let devices_infos = self.devices_infos.clone();
        tokio::spawn(tokio::fs::write(path, devices_infos.serialize()));
    }
}

pub(crate) struct DevicesInfos {
    clients: ArcSwap<HashMap<ClientId, Arc<DeviceInfo>>>,
}

impl DevicesInfos {
    pub(crate) fn load(path: impl AsRef<Path>) -> Self {
        let deserialized: Vec<DeviceInfo> = load_or_create(path.as_ref());

        Self {
            clients: ArcSwap::from_pointee(
                deserialized
                    .into_iter()
                    .map(|info| (info.client_id.clone(), Arc::new(info)))
                    .collect(),
            ),
        }
    }

    pub(crate) fn register_each_device(
        &self,
        device_user_map: &mut HashMap<ClientId, Username>,
        username: Username,
    ) {
        for client_id in self.clients.load().keys() {
            device_user_map.insert(client_id.clone(), username);
        }
    }

    /// Use RCU (Read-Copy-Update) mode
    fn insert_device(&self, device_info: DeviceInfo) {
        let mut new = (**self.clients.load()).clone();
        new.insert(device_info.client_id.clone(), Arc::new(device_info));
        self.clients.store(Arc::new(new));
    }

    fn serialize(&self) -> Vec<u8> {
        let guard = self.clients.load();
        let infos: Vec<_> = guard.values().map(AsRef::as_ref).collect();
        serde_json::to_vec(&infos).unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DeviceInfo {
    pub(crate) client_id: ClientId,
    pub(crate) key: String,
    device_name: ArcSwap<String>,
    pub(crate) is_mobile: bool,
    last_connect_date: Option<usize>,
}

impl DeviceInfo {
    pub(crate) fn new(device_name: String, is_mobile: bool) -> Self {
        Self {
            client_id: rand_16bytes_as_base64(),
            key: rand_16bytes_as_base64(),
            device_name: ArcSwap::from_pointee(device_name),
            is_mobile,
            last_connect_date: Some(0),
        }
    }

    pub(crate) fn get_device_name(&self) -> Arc<String> {
        self.device_name.load().clone()
    }
}
