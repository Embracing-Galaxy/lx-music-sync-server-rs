use crate::auth::{auth_by_code, auth_by_key};
use crate::data::{ClientId, DataType, SnapshotKey, SERVER_ID_PREFIX, SERVER_INFO};
use crate::server::{socket::handle_socket, SERVER_CONTEXT};
use axum::extract::State;
use axum::{
    extract::{ConnectInfo, Query, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use dashmap::DashMap;
use log::info;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};

mod auth;
mod data;
mod server;
mod utils;

async fn server_id() -> String {
    format!("{}{}", SERVER_ID_PREFIX, SERVER_INFO.server_id)
}

async fn auth_code(headers: HeaderMap, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Response {
    let Ok(ip) = SERVER_CONTEXT.get_ip(&headers, addr).await else {
        return (StatusCode::FORBIDDEN, "Blocked IP").into_response();
    };
    let Some(msg) = headers.get("m") else {
        SERVER_CONTEXT.record_auth_failed_ip(&ip).await;
        return (StatusCode::UNAUTHORIZED, "Auth failed").into_response();
    };
    let msg = msg.to_str().unwrap();
    let response = if let Some(userid) = headers.get("i") {
        let client_id = userid.to_str().unwrap().to_owned();
        let user_space = SERVER_CONTEXT.get_client_user_space(&client_id);
        auth_by_key(msg, &client_id, user_space).await
    } else {
        auth_by_code(msg).await
    };

    match response {
        Err(err_msg) => (StatusCode::FORBIDDEN, err_msg).into_response(),
        Ok(response) => response.into_response(),
    }
}

async fn hello() -> &'static str {
    "Hello~::^-^::~v4~"
}

#[derive(Deserialize)]
struct SocketInfo {
    i: ClientId,
    #[allow(unused)]
    t: String,
}
type ConnectionMap = Arc<DashMap<ClientId, oneshot::Sender<()>>>;
type BroadcastMsg = (
    ClientId,
    &'static str, // Req name
    serde_json::Value,
    DataType,
    SnapshotKey,
);
type Broadcaster = broadcast::Sender<BroadcastMsg>;
type Subscriber = broadcast::Receiver<BroadcastMsg>;
type ServerState = (ConnectionMap, Broadcaster);

/// Websocket handshake entrypoint
async fn websocket(
    ws: WebSocketUpgrade,
    Query(query): Query<SocketInfo>,
    State(socket_state): State<ServerState>,
) -> Response {
    let client_id = &query.i;
    let (username, user_space) = if let Some(username) = SERVER_CONTEXT.get_username(client_id)
        && let Some(user_space) = SERVER_CONTEXT.get_user_space(username)
    {
        (username, user_space)
    } else {
        return (StatusCode::NOT_FOUND, "The user does not exist").into_response();
    };

    let Some(device_info) = user_space.get_client_device_info(client_id).await else {
        return (StatusCode::BAD_REQUEST, "missing ?i=clientId").into_response();
    };
    ws.on_upgrade(|socket| handle_socket(socket, username, device_info, socket_state))
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let log_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
    SERVER_CONTEXT.start_daemon();
    let connections = ConnectionMap::default();
    let (tx, _) = broadcast::channel(16);

    // build our application with a route
    let app = Router::new()
        .route("/id", get(server_id))
        .route("/ah", get(auth_code))
        .route("/hello", get(hello))
        .route("/socket", get(websocket))
        .fallback(fallback)
        .with_state((connections, tx))
        .into_make_service_with_connect_info::<SocketAddr>();

    let addr = SocketAddr::from(([127, 0, 0, 1], 9527));
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
