use crate::data::DataType;
use crate::{
    Broadcaster, ServerState, Subscriber,
    data::{ClientId, SnapshotKey, Username, config::CONFIG, user::DeviceInfo},
    server::SERVER_CONTEXT,
    utils::{gzip_base64, ungzip_base64},
};
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use dto::{EnabledFeatures, Req, Resp};
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use log::{info, trace};
use serde_json::json;
use std::fmt::Formatter;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinSet;
use tokio::time::interval;

mod dto;

#[derive(Clone)]
pub(super) struct SocketContext {
    sender: Arc<Mutex<Sender>>,
    list_ready: Arc<AtomicBool>,
    dislike_ready: Arc<AtomicBool>,
    callbacks: Arc<DashMap<String, oneshot::Sender<Resp>>>,
    broadcaster: Broadcaster,
    pub(super) client_id: ClientId,
    pub(crate) username: Username,
}

type Sender = SplitSink<WebSocket, Message>;
type Receiver = SplitStream<WebSocket>;

impl SocketContext {
    pub(crate) fn new(
        sender: Sender,
        broadcaster: Broadcaster,
        device_info: &DeviceInfo,
        username: Username,
    ) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
            list_ready: Default::default(),
            dislike_ready: Default::default(),
            callbacks: Default::default(), // TODO A more efficient CallbackManager (make id usize & use vec)
            broadcaster,
            client_id: device_info.client_id.clone(),
            username,
        }
    }

    #[inline]
    pub(crate) fn list_ready(&self) {
        self.list_ready.store(true, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn dislike_ready(&self) {
        self.dislike_ready.store(true, Ordering::Relaxed);
    }

    /// Send a request, returns: the callback
    ///
    /// # Arguments
    ///
    /// * `name`: the request name
    /// * `data`: the exact data to send
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// let callback = socket_context.request("get", vec![json!("data")]).await;
    /// let resp = callback.await.unwrap();
    /// ```
    pub(crate) async fn request(
        &self,
        name: &str,
        data: Vec<serde_json::Value>,
    ) -> oneshot::Receiver<Resp> {
        trace!("request: {}", name);
        let (key, req) = Req::new(name, data);
        let text = req.to_json();
        let message = Self::zip_req(text);
        {
            let mut lock = self.sender.lock().await;
            lock.send(message).await.expect("Failed to send message");
        }
        let (tx, rx) = oneshot::channel();
        self.callbacks.insert(key, tx);
        rx
    }

    pub(crate) async fn post(&self, name: &str, data: Vec<serde_json::Value>) {
        trace!("request without callback: {}", name);
        let (_, req) = Req::new(name, data);
        let text = req.to_json();
        let message = Self::zip_req(text);
        let mut lock = self.sender.lock().await;
        lock.send(message).await.expect("Failed to send message");
    }

    fn zip_req(req: String) -> Message {
        if req.len() > 1024 {
            Message::Text(format!("cg_{}", gzip_base64(req)).into())
        } else {
            Message::Text(req.into())
        }
    }

    pub(crate) fn on_message_string(&self, resp: &[u8]) {
        let json = match serde_json::from_slice::<Resp>(resp) {
            Ok(json) => json,
            Err(_) => todo!("handle req"),
        };
        if let Some((_, tx)) = self.callbacks.remove(json.get_name())
            && !tx.is_closed()
        {
            tx.send(json).unwrap();
        }
    }

    pub(crate) async fn broadcast_sync_result(
        &self,
        data_type: DataType,
        data: serde_json::Value,
        key: SnapshotKey,
    ) {
        self.broadcaster
            .send((
                self.client_id.clone(),
                match data_type {
                    DataType::LIST => "onListSyncAction",
                    DataType::DISLIKE => "onDislikeSyncAction",
                },
                data,
                data_type,
                key,
            ))
            .expect("Failed to broadcast sync result");
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
                trace!(
                    "{:?} received data:{}",
                    context,
                    String::from_utf8_lossy(&data)
                );
                context.on_message_string(&data);
            }
            Message::Pong(_) => {
                got_pong.store(true, Ordering::Relaxed);
            }
            _ => trace!("Skipped message: {:?}", msg),
        }
    }
}

async fn handle_broadcast(context: SocketContext, mut rx: Subscriber) {
    while let Ok((client_id, name, data, data_type, key)) = rx.recv().await {
        if context.client_id == client_id {
            continue;
        }

        context.post(name, vec![data]).await;

        // already checked in main
        let user_space = SERVER_CONTEXT.get_user_space(context.username).unwrap();
        match data_type {
            DataType::LIST => {
                user_space
                    .list
                    .update_snapshot_key(&context.client_id, key)
                    .await
            }
            DataType::DISLIKE => {
                user_space
                    .dislike
                    .update_snapshot_key(&context.client_id, key)
                    .await
            }
        }
    }
}

async fn sync_once(context: SocketContext) {
    let callback = context
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
        .await;
    let resp = callback.await.unwrap();
    let enabled_features = resp.get_data::<EnabledFeatures>().unwrap();
    assert_eq!(enabled_features, EnabledFeatures::DEFAULT);

    // already checked in main
    let user_space = SERVER_CONTEXT.get_user_space(context.username).unwrap();
    let add_location = &CONFIG
        .user_configs
        .get(context.username)
        .unwrap()
        .add_music_location;
    super::sync::sync_once(&context, user_space, add_location).await;
    context.post("finished", vec![]).await;
}

pub(crate) async fn handle_socket(
    socket: WebSocket,
    username: Username,
    device_info: Arc<DeviceInfo>,
    (connections, broadcaster): ServerState,
) {
    let (close_tx, close_rx) = oneshot::channel(); // channel to send close signal

    if let Some(old_close_tx) = connections.insert(device_info.client_id.clone(), close_tx) {
        let _ = old_close_tx.send(());
        info!("Duplicate connection found! Closed old connection.");
    }

    let device_name = device_info.get_device_name();
    info!("User {username} on device {device_name} connected.");

    let (sender, receiver) = socket.split();
    let broadcast_rx = broadcaster.subscribe();
    let socket_context = SocketContext::new(sender, broadcaster, &device_info, username);

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
    tasks.spawn(handle_broadcast(context, broadcast_rx));

    tasks.spawn(sync_once(socket_context));
    tokio::select! {
        _ = message_handle => {},
        _ = close_rx => {},
    }
    tasks.shutdown().await;
    info!("Closing connection of device {device_name}.");
    connections.remove(&device_info.client_id);
}
