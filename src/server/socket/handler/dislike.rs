use super::*;
use crate::data::dislike::DislikeData;
type Name = String;
type Singer = String;

#[derive(Deserialize, enum_handler::EnumHandler)]
#[serde(tag = "action", content = "data")]
#[enum_handler(is_async = true, no_async_trait_macro = true)]
enum DislikeSyncAction {
    #[serde(rename = "dislike_data_overwrite")]
    Overwrite(DislikeData),
    #[serde(rename = "dislike_music_add")]
    Add(Vec<(Name, Singer)>),
    #[serde(rename = "dislike_music_clear")]
    Clear,
}

struct DislikeSyncHandler {
    user_space: &'static UserSpace,
}

impl DislikeSyncActionHandler for DislikeSyncHandler {
    async fn on_overwrite(&self, data: DislikeData) {
        self.user_space.dislike.overwrite(data).await;
    }

    async fn on_add(&self, data: Vec<(Name, Singer)>) {
        let new_rules: Vec<String> = data
            .into_iter()
            .map(format_to_rule)
            .collect();
        self.user_space.dislike.append(&new_rules.join("\n"));
    }

    async fn on_clear(&self) {
        self.user_space.dislike.clear().await;
    }
}

fn format_to_rule(name_and_singer: (Name, Singer)) -> DislikeData {
    let (name, singer) = name_and_singer;
    if singer.is_empty() {
        name
    } else {
        format!("{name}@{singer}")
    }
}

pub(in crate::server::socket) fn on_dislike_sync(ready: &AtomicBool, user_space: &UserSpace) {
    if !ready.load(Ordering::Relaxed) {
        return;
    }
    todo!()
}
