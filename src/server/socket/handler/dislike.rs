use super::*;
type Name = String;
type Singer = String;

#[derive(Deserialize, enum_handler::EnumHandler)]
#[serde(tag = "action", content = "data")]
enum DislikeSyncAction {
    #[serde(rename = "dislike_data_overwrite")]
    Overwrite(String),
    #[serde(rename = "dislike_music_add")]
    Add(Vec<(Name, Singer)>),
    #[serde(rename = "dislike_music_clear")]
    Clear,
}

struct DislikeSyncHandler;

impl DislikeSyncActionHandler for DislikeSyncHandler {
    fn on_overwrite(&self, data: String) -> () {
        todo!()
    }

    fn on_add(&self, data: Vec<(Name, Singer)>) -> () {
        todo!()
    }

    fn on_clear(&self) -> () {
        todo!()
    }
}

pub(in crate::server::socket) fn on_dislike_sync(ready: &AtomicBool, user_space: &UserSpace) {
    if !ready.load(Ordering::Relaxed) {
        return;
    }
    todo!()
}
