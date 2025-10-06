use crate::auth::{auth_by_code, auth_by_key};
use crate::data::{SERVER_ID_PREFIX, SERVER_INFO};
use crate::utils::ungzip_base64;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_ws::AggregatedMessage;
use serde::Deserialize;
use server::SERVER_CONTEXT;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::interval;
mod auth;
mod data;
mod server;
mod utils;

#[get("/")]
async fn root() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/id")]
async fn server_id() -> HttpResponse {
    HttpResponse::Ok().body(format!("{}{}", SERVER_ID_PREFIX, SERVER_INFO.server_id))
}

#[allow(unreachable_code)]
#[post("/au")]
async fn auth_code(req: HttpRequest) -> HttpResponse {
    let Ok(ip) = SERVER_CONTEXT.get_ip(&req).await else {
        return HttpResponse::Forbidden().body("Blocked IP");
    };
    let headers = req.headers();
    let Some(msg) = headers.get("m") else {
        SERVER_CONTEXT.record_auth_failed_ip(&ip).await;
        return HttpResponse::Unauthorized().body("Auth failed");
    };
    let msg = msg.to_str().unwrap();
    let response = if let Some(userid) = headers.get("i") {
        let client_id = userid.to_str().unwrap().to_owned();
        let user_space = SERVER_CONTEXT.get_client_user_space(&client_id);
        auth_by_key(msg, &client_id , user_space).await
    } else {
        auth_by_code(msg).await
    };

    match response {
        Err(err_msg) => HttpResponse::Forbidden().body(err_msg),
        Ok(response) => HttpResponse::Ok().body(response.to_string()),
    }
}

#[get("/hello")]
async fn hello() -> HttpResponse {
    HttpResponse::Ok().body("Hello~::^-^::~v4~")
}

#[derive(Deserialize)]
struct SocketInfo {
    i: String,
    #[allow(unused)]
    t: String,
}

#[get("/socket")]
async fn websocket(
    req: HttpRequest,
    query: web::Query<SocketInfo>,
    stream: web::Payload,
) -> actix_web::Result<HttpResponse> {
    let client_id = &query.i;
    let user_space = SERVER_CONTEXT.get_client_user_space(client_id);
    let Some(user_space) = user_space else {
        return Ok(HttpResponse::NotFound().body("The user does not exist"));
    };

    let Some(device_info) = user_space.get_client_device_info(client_id).await else {
        return Ok(HttpResponse::BadRequest().body("missing ?i=clientId"));
    };

    let (res, session, stream) = actix_ws::handle(&req, stream)?;
    // 如果池中已存在，踢掉旧连接
    let socket_context = SERVER_CONTEXT.register_socket(&session, &device_info).await;
    let mut stream = stream.aggregate_continuations();

    // TODO status

    // ping & check pong timeout
    let mut session_cloned = session.clone();
    let is_mobile = device_info.is_mobile;
    actix_web::rt::spawn(async move {
        let mut pong_timeout = interval(Duration::from_secs(30));
        let get_pong = AtomicBool::new(true);
        loop {
            pong_timeout.tick().await;
            if !get_pong.swap(false, Ordering::Relaxed) {
                session_cloned.close(None).await.unwrap();
                break;
            }

            if is_mobile {
                session_cloned.text("ping").await.unwrap();
            }
            session_cloned.ping(b"").await.unwrap();
        }
    });
    socket_context.sync_once();
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = stream.recv().await {
            match msg {
                AggregatedMessage::Text(text) => {
                    let sync_data = if text[..3].eq("cg_") {
                        ungzip_base64(&text[3..])
                    } else {
                        text.as_bytes().to_vec()
                    };
                    socket_context.on_response_string(&*sync_data).await;
                }
                AggregatedMessage::Pong(_) => {
                    socket_context.got_pong();
                }
                AggregatedMessage::Close(reason) => {
                    session.close(reason).await.unwrap();
                    break;
                }
                _ => {}
            }
        }
    });
    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SERVER_CONTEXT.start_daemon();

    HttpServer::new(|| {
        App::new()
            .service(root)
            .service(hello)
            .service(server_id)
            .service(auth_code)
            .service(websocket)
            .default_service(web::to(|| HttpResponse::Unauthorized()))
    })
    .bind(("127.0.0.1", 9527))?
    .run()
    .await
}
