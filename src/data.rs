pub(crate) mod config;
pub(crate) mod dislike;
pub(crate) mod list;
pub(crate) mod manager;
pub(crate) mod user;

use crate::utils::crypto::{rand_16bytes_as_base64, MD5};
use crate::utils::{load_or_create, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::{path::Path, sync::LazyLock};
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

pub(crate) type SnapshotKey = MD5;

#[derive(Clone, Default, Deserialize, Serialize)]
struct SnapshotInfo {
    latest_key: Option<SnapshotKey>,
    time: u128,
    saved_keys: HashSet<SnapshotKey>,
    clients: HashMap<ClientId, SnapshotKey>,
}

impl SnapshotInfo {
    /// Update `time` & `latest_key` anyhow, and then returns whether the key was newly inserted.
    ///
    /// # Arguments
    ///
    /// * `key`: the new snapshot key
    fn try_insert_key(&mut self, key: SnapshotKey) -> bool {
        self.time = now_ms();
        self.latest_key = Some(key);
        self.saved_keys.insert(key)
    }

    fn update(&mut self, client_id: &ClientId, key: SnapshotKey) {
        self.clients.insert(client_id.clone(), key);
    }
}
