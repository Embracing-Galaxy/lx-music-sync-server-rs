use super::*;
use crate::data::dislike::{DislikeData, Name, Singer};

#[derive(Deserialize)]
#[serde(tag = "action", content = "data")]
pub(crate) enum DislikeSyncAction {
    #[serde(rename = "dislike_data_overwrite")]
    Overwrite(DislikeData),
    #[serde(rename = "dislike_music_add")]
    Add(Vec<(Name, Singer)>),
    #[serde(rename = "dislike_music_clear")]
    Clear,
}

impl JsonReqHandler for DislikeData {
    async fn on(&mut self, e: serde_json::Value) {
        match serde_json::from_value(e).unwrap() {
            DislikeSyncAction::Overwrite(data) => self.on_overwrite(data).await,
            DislikeSyncAction::Add(data) => self.on_add(data).await,
            DislikeSyncAction::Clear => self.on_clear().await,
        }
    }
}

pub(crate) trait DislikeSyncActionHandler {
    async fn on_overwrite(&mut self, data: DislikeData);
    async fn on_add(&mut self, arg: Vec<(Name, Singer)>);
    async fn on_clear(&mut self);
}
