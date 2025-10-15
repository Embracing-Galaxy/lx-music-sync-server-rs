use crate::data::user::DeviceInfo;
use crate::data::Username;
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
use futures_util::stream::SplitStream;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, info};
use serde_json::json;
use std::fmt::Formatter;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinSet;
use tokio::time::interval;

// static LOCK: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));
#[derive(Clone)]
pub(super) struct SocketContext {
    sender: Arc<Mutex<Sender>>,
    list_ready: Arc<AtomicBool>,
    handlers: Arc<DashMap<String, oneshot::Sender<Resp>>>,
    pub(super) client_id: ClientId,
    pub(crate) username: Username,
}

type Sender = SplitSink<WebSocket, Message>;
type Receiver = SplitStream<WebSocket>;

impl SocketContext {
    pub(crate) fn new(sender: Sender, device_info: &DeviceInfo, username: Username) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
            list_ready: Default::default(),
            handlers: Default::default(),
            client_id: device_info.client_id.clone(),
            username,
        }
    }

    pub(crate) fn list_ready(&self) {
        self.list_ready.store(true, Ordering::Relaxed);
    }

    pub(crate) async fn request(
        &self,
        name: &str,
        data: Vec<serde_json::Value>,
    ) -> Result<oneshot::Receiver<Resp>, axum::Error> {
        debug!("request: {}", name);
        let (key, req) = Req::new(name, data);
        let text = req.to_json();
        if text.len() > 1024 {
            self.sender
                .lock()
                .await
                .send(Message::Text(format!("cg_{}", gzip_base64(text)).into()))
                .await?;
        } else {
            self.sender
                .lock()
                .await
                .send(Message::Text(text.into()))
                .await?;
        }
        let (tx, rx) = oneshot::channel();
        self.handlers.insert(key, tx);
        Ok(rx)
    }

    pub(crate) fn on_message_string(&self, resp: &[u8]) {
        let json = match serde_json::from_slice::<Resp>(resp) {
            Ok(json) => json,
            Err(_) => todo!("handle req"),
        };
        if let Some((_, tx)) = self.handlers.remove(json.get_name()) {
            tx.send(json).unwrap();
        }
    }

    pub(crate) async fn broadcast_sync_result(&self) {
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

async fn heartbeat(
    context: SocketContext,
    got_pong: Arc<AtomicBool>,
    is_mobile: bool,
    device_name: Arc<String>,
) {
    let mut pong_timeout = interval(Duration::from_secs(30));
    loop {
        pong_timeout.tick().await;
        if !got_pong.swap(false, Ordering::Relaxed) {
            info!("Closing connection of device {device_name}: Client didn't pong within 30s.");
            break;
        }

        let ping_msg = if is_mobile {
            Message::text("ping")
        } else {
            Message::Ping(Default::default())
        };
        let mut lock = context.sender.lock().await;
        lock.send(ping_msg).await.expect("Couldn't send ping");
    }
}

async fn on_message(context: SocketContext, mut receiver: Receiver, got_pong: Arc<AtomicBool>) {
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let data = if text[..3].eq("cg_") {
                    ungzip_base64(&text[3..])
                } else {
                    text.as_bytes().to_vec()
                };
                debug!(
                    "{:?} received data:{}",
                    context,
                    String::from_utf8_lossy(&data)
                );
                context.on_message_string(&data);
            }
            Message::Pong(_) => {
                got_pong.store(true, Ordering::Relaxed);
            }
            _ => debug!("Skipped message: {:?}", msg),
        }
    }
}

async fn sync_once(context: SocketContext) {
    let receiver = context
        .request(
            "getEnabledFeatures",
            vec![
                json!("server"),
                json!({
                    "list": 1,
                    "dislike": 1
                }),
            ],
        )
        .await
        .unwrap();
    let resp = receiver.await.unwrap();
    let enabled_features = resp.get_data::<EnabledFeatures>().unwrap();
    // let _ = LOCK.acquire().await;
    sync_list_once(&context, enabled_features).await;
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
        info!("Duplicate connection found! Closed old connection");
    }

    let device_name = device_info.get_device_name();
    info!("User {username} on device {device_name} connected");

    let (sender, receiver) = socket.split();
    let socket_context = SocketContext::new(sender, &device_info, username);

    let flag = Arc::new(AtomicBool::new(true));
    let context = socket_context.clone();
    let got_pong = flag.clone();
    let device_name_cloned = device_name.clone();
    let is_mobile = device_info.is_mobile;
    let mut tasks = JoinSet::new();
    tasks.spawn(heartbeat(context, got_pong, is_mobile, device_name_cloned));

    let context = socket_context.clone();
    let got_pong = flag.clone();
    let message_handle = tokio::spawn(on_message(context, receiver, got_pong));

    let context = socket_context.clone();
    tasks.spawn(sync_once(context));
    tokio::select! {
        _ = message_handle => {},
        _ = close_rx => {},
    }
    tasks.shutdown().await;
    info!("Closing connection of device {device_name}.");
    connections.remove(&device_info.client_id);
}
