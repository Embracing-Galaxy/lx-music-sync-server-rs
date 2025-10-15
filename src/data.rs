pub(crate) mod config;
pub(crate) mod list;
pub(crate) mod user;

use crate::utils::{crypto::rand_16bytes_as_base64, load_or_create};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

pub(crate) const SERVER_ID_PREFIX: &str = "OjppZDo6";
pub(crate) static SERVER_INFO: LazyLock<ServerInfo> =
    LazyLock::new(|| load_or_create(Path::new("server_info.json")));

pub(crate) type ClientId = String;
pub(crate) type Username = &'static str;

#[derive(Serialize, Deserialize)]
pub(crate) struct ServerInfo {
    pub(crate) server_id: String,
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            server_id: rand_16bytes_as_base64(),
        }
    }
}
