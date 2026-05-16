use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct MusicInfo {
    id: String, // only id is used in list
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

    pub(super) fn take_id(&self) -> String {
        self.id.clone()
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
    Local,
}
