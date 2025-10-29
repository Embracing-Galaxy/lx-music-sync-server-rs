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
    fn on(&mut self, e: serde_json::Value) {
        match serde_json::from_value(e).unwrap() {
            DislikeSyncAction::Overwrite(data) => self.on_overwrite(data),
            DislikeSyncAction::Add(data) => self.on_add(data),
            DislikeSyncAction::Clear => self.on_clear(),
        }
    }
}

pub(crate) trait DislikeSyncActionHandler {
    fn on_overwrite(&mut self, data: DislikeData);
    fn on_add(&mut self, arg: Vec<(Name, Singer)>);
    fn on_clear(&mut self);
}
