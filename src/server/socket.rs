use crate::data::user::DeviceInfo;
use crate::utils::ungzip_base64;
use crate::{
    data::ClientId,
    server::dto::{EnabledFeatures, Req, Resp},
    server::sync::sync_list_once,
    utils::gzip_base64,
    ConnectionMap,
};
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use log::{debug, info, warn};
use serde_json::json;
use std::fmt::Formatter;
use std::sync::{
    atomic::{AtomicBool, Ordering}, Arc,
    LazyLock,
};
use std::time::Duration;
use tokio::sync::{oneshot, Semaphore};
use tokio::time::interval;
use crate::data::Username;

static LOCK: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

pub(crate) struct SocketContext {
    socket: WebSocket,
    handlers: Arc<DashMap<String, oneshot::Sender<Resp>>>,
    pub(super) client_id: ClientId,
    is_mobile: bool,
    pub(crate) username: Username,
}

impl SocketContext {
    pub(crate) fn new(socket: WebSocket, device_info: &DeviceInfo, username: Username) -> Self {
        Self {
            socket,
            handlers: Default::default(),
            client_id: device_info.client_id.clone(),
            is_mobile: device_info.is_mobile,
            username,
        }
    }

    async fn ping(&mut self) {
        let ping_msg = if self.is_mobile {
            Message::text("ping")
        } else {
            Message::Ping(Default::default())
        };
        self.socket
            .send(ping_msg)
            .await
            .expect("Couldn't send ping");
    }

    pub(crate) async fn request(
        &mut self,
        name: &str,
        data: Option<serde_json::Value>,
    ) -> Result<oneshot::Receiver<Resp>, axum::Error> {
        let text = Req::new(name, data).to_json();
        if text.len() > 1024 {
            self.socket
                .send(Message::Text(format!("cg_{}", gzip_base64(text)).into()))
                .await?;
        } else {
            self.socket.send(Message::Text(text.into())).await?;
        }
        let (tx, rx) = oneshot::channel();
        self.handlers.insert(name.to_string(), tx);
        Ok(rx)
    }

    pub(crate) fn on_response_string(&self, resp: &[u8]) {
        let json = serde_json::from_slice::<Resp>(resp).unwrap();
        let Some((_, tx)) = self.handlers.remove(json.get_name()) else {
            return;
        };
        tx.send(json).unwrap();
    }

    async fn sync_once(&mut self) {
        let enabled_features = self.get_enabled_features().await;
        let _ = LOCK.acquire().await;
        sync_list_once(self, enabled_features).await;
    }

    async fn get_enabled_features(&mut self) -> EnabledFeatures {
        let receiver = self
            .request(
                "getEnabledFeatures",
                Some(json!({
                    "list": 1,
                    "dislike": 1
                })),
            )
            .await
            .unwrap();
        let resp = receiver.await.unwrap();
        resp.get_data::<EnabledFeatures>().unwrap()
    }

    pub(crate) async fn broadcast_sync_result(&mut self) {
        todo!("rewrite")
    }

    async fn on_list_sync_action(&self) {
        todo!("broadcast sync action & update snapshot key");
    }
}

impl std::fmt::Debug for SocketContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket")
            .field("client_id", &self.client_id)
            .finish()
    }
}

pub(crate) async fn handle_socket(
    socket: WebSocket,
    username: Username,
    device_info: Arc<DeviceInfo>,
    connections: ConnectionMap,
) {
    let (close_tx, close_rx) = oneshot::channel(); // channel to send close signal

    if let Some(old_close_tx) = connections.insert(device_info.client_id.clone(), close_tx) {
        let _ = old_close_tx.send(());
        warn!("Duplicate connection found! Closed old connection");
    }

    let device_name = device_info.get_device_name();
    info!("User {username} on device {device_name} connected");

    let socket_context: SocketContext = SocketContext::new(socket, &device_info, username);
    tokio::select! {
        _ = handle_websocket_messages(socket_context, &device_name) => {},
        _ = close_rx => {
            // Received a shutdown signal, exit directly.
        },
    }

    connections.remove(&device_info.client_id);
}

async fn handle_websocket_messages(mut context: SocketContext, device_name: &str) {
    context.sync_once().await;

    let mut pong_timeout = interval(Duration::from_secs(30));
    let got_pong = AtomicBool::new(true);
    loop {
        tokio::select! {
            _ = pong_timeout.tick() => {
                if !got_pong.swap(false, Ordering::Relaxed) {
                    warn!("Closing connection of device {device_name}: Client didn't pong within 30s.");
                    break;
                }
                context.ping().await;
            }
            Some(Ok(msg)) = context.socket.recv() => {
                match msg {
                    Message::Text(text) => {
                        let data = if text[..3].eq("cg_") {
                            ungzip_base64(&text[3..])
                        } else {
                            text.as_bytes().to_vec()
                        };
                        debug!("{:?} received data:{}", context, String::from_utf8_lossy(&data));
                        context.on_response_string(&data);
                        todo!("handle req");
                    }
                    Message::Pong(_) => {
                        got_pong.store(true, Ordering::Relaxed);
                    }
                    Message::Close(reason) => {
                        info!("Closing connection of device {device_name}, for reason: {:?}", reason);
                        break;
                    }
                    _ => warn!("Unsupported message: {:?}", msg),
                }
            }
        }
    }
}
