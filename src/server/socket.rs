use crate::server::SERVER_CONTEXT;
use crate::{
    data::ClientId,
    server::dto::{EnabledFeatures, Req, Resp},
    server::sync::sync_list_once,
    utils::gzip_base64,
};
use actix_ws::Session;
use serde_json::json;
use std::fmt::Formatter;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering}, Arc,
        LazyLock,
    },
};
use tokio::sync::{oneshot, Mutex, Semaphore};

static LOCK: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

#[derive(Clone)]
pub(crate) struct SocketContext {
    pub(super) session: Session,
    got_pong: Arc<AtomicBool>,
    list_ready: Arc<AtomicBool>,
    handlers: Arc<Mutex<HashMap<String, oneshot::Sender<Resp>>>>,
    pub(super) client_id: ClientId,
    pub(crate) username: &'static str,
}

impl SocketContext {
    pub(crate) fn new(session: Session, client_id: ClientId, username: &'static str) -> Self {
        Self {
            session,
            got_pong: Arc::new(AtomicBool::new(true)),
            list_ready: Default::default(),
            handlers: Default::default(),
            client_id,
            username,
        }
    }

    pub(crate) fn got_pong(&self) {
        self.got_pong.store(true, Ordering::Relaxed);
    }

    pub(crate) fn list_ready(&self) {
        self.list_ready.store(true, Ordering::Relaxed);
    }

    pub(crate) async fn request(
        &mut self,
        name: &str,
        data: Option<serde_json::Value>,
    ) -> Result<oneshot::Receiver<Resp>, actix_ws::Closed> {
        let text = Req::new(name, data).to_json();
        if text.len() > 1024 {
            self.session
                .text(format!("cg_{}", gzip_base64(text)))
                .await?;
        } else {
            self.session.text(text).await?;
        }
        let (tx, rx) = oneshot::channel();
        self.handlers.lock().await.insert(name.to_string(), tx);
        Ok(rx)
    }

    pub(crate) async fn on_response_string(&self, resp: &[u8]) {
        let json = serde_json::from_slice::<Resp>(resp).unwrap();
        let Some(tx) = self.handlers.lock().await.remove(json.get_name()) else {
            return;
        };
        tx.send(json).unwrap();
    }

    pub(crate) fn sync_once(&self) {
        let mut socket = self.clone();
        actix_web::rt::spawn(async move {
            let enabled_features = socket.get_enabled_features().await;
            let _ = LOCK.acquire().await;
            sync_list_once(&mut socket, enabled_features).await;
        });
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

    // TODO optimize RwLock here
    pub(crate) async fn broadcast_sync_result(&mut self) {
        let guard = SERVER_CONTEXT.sockets.read().await;
        async_scoped::TokioScope::scope_and_block(|scope| {
            guard
                .values()
                .filter(|socket| {
                    socket.username != self.username && socket.list_ready.load(Ordering::Relaxed)
                })
                .for_each(|socket| scope.spawn(socket.on_list_sync_action()));
        });
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
