use crate::data::config::CONFIG;
use crate::data::user::{DevicesInfos, UserSpace};
use crate::data::{ClientId, Username};
use crate::utils::RwCounter;
use axum::http::HeaderMap;
use std::net::SocketAddr;
use std::path::Path;
use std::{collections::HashMap, sync::LazyLock, time::Duration};
use std::sync::Arc;
use arc_swap::ArcSwap;
use tokio::time::interval;

mod dto;
pub(crate) mod socket;
mod sync;

pub(crate) static SERVER_CONTEXT: LazyLock<ServerContext> = LazyLock::new(|| ServerContext::new());

pub(crate) struct ServerContext {
    auth_failed_ips: RwCounter<String>,
    device_username_map: ArcSwap<HashMap<ClientId, Username>>,
    user_space_map: HashMap<Username, UserSpace>,
}

pub(crate) struct BlockedIPError;

impl ServerContext {
    fn new() -> Self {
        let mut device_user_map = HashMap::new();
        let mut user_space_map = HashMap::new();
        for username in CONFIG.user_configs.keys() {
            let path = Path::new(username).join("devices.json");
            let infos = DevicesInfos::load(&path);
            infos.register_each_device(&mut device_user_map, &username);
            user_space_map.insert(
                username.as_str(),
                UserSpace::new(username, infos, path.into()),
            );
        }
        Self {
            auth_failed_ips: RwCounter::new(),
            device_username_map: ArcSwap::from_pointee(device_user_map),
            user_space_map,
        }
    }

    pub(crate) fn update_device_username_map(&self, client_id: ClientId, username: Username) {
        let mut new_map = (**self.device_username_map.load()).clone();
        new_map.insert(client_id, username);
        self.device_username_map.store(Arc::new(new_map));
    }

    pub(crate) async fn get_ip(
        &self,
        headers: &HeaderMap,
        addr: SocketAddr,
    ) -> Result<String, BlockedIPError> {
        let ip = if CONFIG.enable_proxy
            && let Some(real_ip) = headers.get("x-real-ip")
        {
            real_ip.to_str().unwrap().to_string()
        } else {
            addr.ip().to_string()
        };

        if self.auth_failed_ips.count(&ip).await < 20 {
            Ok(ip)
        } else {
            Err(BlockedIPError)
        }
    }

    /// get the corresponding username
    pub(crate) fn get_username(&self, client_id: &ClientId) -> Option<&str> {
        self.device_username_map.load().get(client_id).cloned()
    }

    pub(crate) fn get_user_space(&self, user_name: &str) -> Option<&UserSpace> {
        self.user_space_map.get(user_name)
    }

    pub(crate) fn get_client_user_space(&self, client_id: &ClientId) -> Option<&UserSpace> {
        self.user_space_map
            .get(self.device_username_map.load().get(client_id)?)
    }

    pub(crate) async fn record_auth_failed_ip(&self, ip: &String) {
        self.auth_failed_ips.increase(ip.clone()).await;
    }

    pub(crate) fn start_daemon(&'static self) {
        tokio::spawn(async move {
            let mut clean_expired_ip_record = interval(Duration::from_secs(3600));
            loop {
                clean_expired_ip_record.tick().await;
                self.auth_failed_ips.cleanup().await;
            }
        });
    }
}
