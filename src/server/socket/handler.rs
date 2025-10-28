pub(crate) use dislike::DislikeSyncActionHandler;
pub(crate) use list::{ListSyncActionHandler, MusicUpdateInfo};
use serde::Deserialize;

mod dislike;
mod list;

pub(crate) trait JsonReqHandler {
    async fn on(&mut self, e: serde_json::Value);
}
