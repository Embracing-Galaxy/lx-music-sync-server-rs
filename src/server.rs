use crate::data::config::CONFIG;
use crate::data::user::{DeviceInfo, DevicesInfos, UserSpace};
use crate::data::ClientId;
use crate::utils::RwCounter;
use actix_ws::Session;
use socket::SocketContext;
use std::path::Path;
use std::{collections::HashMap, sync::LazyLock, time::Duration};
use tokio::{sync::RwLock, time::interval};

mod dto;
pub(crate) mod socket;
pub(crate) mod sync;

pub(crate) static SERVER_CONTEXT: LazyLock<ServerContext> = LazyLock::new(|| ServerContext::new());

pub(crate) struct ServerContext {
    sockets: RwLock<HashMap<ClientId, SocketContext>>,
    auth_failed_ips: RwCounter<String>,
    device_user_map: HashMap<ClientId, &'static str>,
    user_space_map: HashMap<&'static str, UserSpace>,
}

pub(crate) struct BlockedIPError;

impl ServerContext {
    fn new() -> Self {
        let mut device_user_map = HashMap::new();
        let mut user_space_map = HashMap::new();
        for username in CONFIG.user_configs.keys() {
            let info = DevicesInfos::load(Path::new(username).join("devices.json"));
            info.register_each_device(&mut device_user_map, &username);
            user_space_map.insert(username.as_str(), UserSpace::new(username.clone()));
        }
        Self {
            sockets: Default::default(),
            auth_failed_ips: RwCounter::new(),
            device_user_map,
            user_space_map,
        }
    }

    pub(crate) async fn get_ip(
        &self,
        req: &actix_web::HttpRequest,
    ) -> Result<String, BlockedIPError> {
        let ip = if CONFIG.enable_proxy
            && let Some(real_ip) = req.headers().get("x-real-ip")
        {
            Some(real_ip.to_str().unwrap().to_string())
        } else if let Some(addr) = req.peer_addr() {
            Some(addr.ip().to_string())
        } else {
            None
        };

        if let Some(ip) = ip
            && self.auth_failed_ips.count(&ip).await < 20
        {
            Ok(ip)
        } else {
            Err(BlockedIPError)
        }
    }

    /// get the corresponding username
    pub(crate) fn get_username(&self, client_id: &ClientId) -> Option<&&str> {
        self.device_user_map.get(client_id)
    }

    pub(crate) fn get_user_space(&self, user_name: &str) -> Option<&UserSpace> {
        self.user_space_map.get(user_name)
    }

    pub(crate) fn get_client_user_space(&self, client_id: &ClientId) -> Option<&UserSpace> {
        self.user_space_map
            .get(self.device_user_map.get(client_id)?)
    }

    /// Would close old socket
    pub(crate) async fn register_socket<'a>(
        &self,
        session: &Session,
        device_info: &DeviceInfo,
    ) -> SocketContext {
        // TODO too much clone here
        let client_id = device_info.client_id.clone();
        let username = *self.device_user_map.get(&client_id).unwrap(); // already checked
        let socket_context = SocketContext::new(session.clone(), client_id.clone(), username);
        if let Some(old) = self
            .sockets
            .write()
            .await
            .insert(client_id, socket_context.clone())
        {
            old.session
                .close(Some(actix_ws::CloseCode::Normal.into()))
                .await
                .unwrap();
        }
        socket_context
    }

    pub(crate) async fn record_auth_failed_ip(&self, ip: &String) {
        self.auth_failed_ips.increase(ip.clone()).await;
    }

    pub(crate) fn start_daemon(&'static self) {
        actix_web::rt::spawn(async move {
            let mut clean_expired_ip_record = interval(Duration::from_secs(3600));
            loop {
                clean_expired_ip_record.tick().await;
                self.auth_failed_ips.cleanup().await;
            }
        });
    }
}
