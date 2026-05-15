use crate::data::{ClientId, Username, dislike::DislikeData, list::ListData, manager::DataManager};
use crate::server::SERVER_CONTEXT;
use crate::utils::{crypto::rand_16bytes_as_base64, load_or_create};
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};

pub(crate) static USERS_PATH: LazyLock<&Path> = LazyLock::new(|| Path::new("users"));

pub(crate) struct UserSpace {
    user_data: UserData,
    pub(crate) list: DataManager<ListData>,
    #[allow(unused)]
    pub(crate) dislike: DataManager<DislikeData>,
}

impl UserSpace {
    pub(crate) fn new(
        user_name: Username,
        user_path: Box<Path>,
        devices_infos: &'static DevicesInfos,
        devices_file_path: &'static Path,
    ) -> Self {
        let user_data = UserData::new(user_name, user_path, devices_infos, devices_file_path);
        let path = &user_data.user_path;
        Self {
            list: DataManager::new(&path.join("list")),
            dislike: DataManager::new(&path.join("dislike")),
            user_data,
        }
    }

    pub(crate) fn get_client_device_info(&self, id: &str) -> Option<Arc<DeviceInfo>> {
        self.user_data.devices_infos.clients.load().get(id).cloned()
    }

    pub(crate) fn update_device_name(&self, id: &ClientId, device_name: &str) {
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
}

struct UserData {
    username: Username,
    user_path: Box<Path>,
    devices_file_path: &'static Path,
    devices_infos: &'static DevicesInfos,
}

impl UserData {
    fn new(
        username: Username,
        user_path: Box<Path>,
        devices_infos: &'static DevicesInfos,
        devices_file_path: &'static Path,
    ) -> Self {
        Self {
            username,
            user_path,
            devices_file_path,
            devices_infos,
        }
    }

    /// Write `devices_infos` to file without await
    fn write_devices_infos(&self) {
        tokio::spawn(tokio::fs::write(
            self.devices_file_path,
            self.devices_infos.serialize(),
        ));
    }
}

pub(crate) struct DevicesInfos {
    clients: ArcSwap<HashMap<ClientId, Arc<DeviceInfo>>>,
}

impl DevicesInfos {
    pub(crate) fn load(path: &Path) -> &'static Self {
        let deserialized: Vec<DeviceInfo> = load_or_create(path);

        let result = Self {
            clients: ArcSwap::from_pointee(
                deserialized
                    .into_iter()
                    .map(|info| (info.client_id.clone(), Arc::new(info)))
                    .collect(),
            ),
        };
        Box::leak(Box::new(result))
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
