use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicInfo {
    id: String, // only id is used in sync
    name: String,
    singer: String,
    source: MusicSource,
    interval: serde_json::Value,
    meta: serde_json::Value,
}

impl MusicInfo {
    pub(super) fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum MusicSource {
    KW,
    KG,
    TX,
    WY,
    MG,
    LOCAL,
}
