pub(crate) use dislike::DislikeSyncActionHandler;
pub(crate) use list::{ListSyncActionHandler, MusicUpdateInfo};
use serde::Deserialize;

mod dislike;
mod list;

pub(crate) trait JsonReqHandler {
    fn on(&mut self, e: serde_json::Value);
}
