use super::*;
use crate::data::config::AddMusicLocation;
use crate::data::list::{de_list_id, CustomListInfo, ListData, MusicInfo};
use serde::Deserializer;

#[derive(Deserialize, enum_handler::EnumHandler)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
enum ListSyncAction {
    ListDataOverwrite(ListData),
    ListCreate {
        position: usize,
        #[serde(rename = "listInfos")]
        infos: Vec<CustomListInfo>,
    },
    ListRemove(Vec<String>),
    ListUpdate(Vec<CustomListInfo>),
    ListUpdatePosition {
        #[serde(deserialize_with = "de_list_ids")]
        ids: Vec<u64>,
        position: usize,
    },

    ListMusicAdd {
        #[serde(deserialize_with = "de_list_id")]
        id: u64,
        #[serde(rename = "musicInfos")]
        musics: Vec<MusicInfo>,
        #[serde(rename = "addMusicLocationType")]
        add_type: AddMusicLocation,
    },
    ListMusicMove {
        #[serde(rename = "fromId")]
        from_id: String, // FIXME
        #[serde(rename = "toId")]
        to_id: String,
        #[serde(rename = "musicInfos")]
        musics: Vec<MusicInfo>,
        #[serde(rename = "addMusicLocationType")]
        add_type: AddMusicLocation,
    },
    ListMusicRemove {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        ids: Vec<String>,
    },
    ListMusicUpdate {
        id: String,
        #[serde(rename = "musicInfo")]
        music: MusicInfo,
    },
    ListMusicUpdatePosition {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        position: usize,
        ids: Vec<String>,
    },
    ListMusicOverwrite {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        #[serde(rename = "musicInfos")]
        musics: Vec<MusicInfo>,
    },
    ListMusicClear(Vec<String>), // FIXME
}

fn de_list_ids<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u64>, D::Error> {
    let raw_vec: Vec<String> = Vec::deserialize(de)?;
    const PREFIX: usize = "userlist_".len();
    raw_vec
        .into_iter()
        .map(|s| s[PREFIX..].parse().map_err(serde::de::Error::custom))
        .collect()
}

struct ListSyncHandler {
    user_space: &'static UserSpace,
}

impl ListSyncActionHandler for ListSyncHandler {
    fn on_list_data_overwrite(&self, data: ListData) -> () {
        self.user_space.list.overwrite(data);
    }

    fn on_list_create(&self, position: usize, infos: Vec<CustomListInfo>) -> () {
        todo!()
    }

    fn on_list_remove(&self, arg: Vec<String>) -> () {
        todo!()
    }

    fn on_list_update(&self, arg: Vec<CustomListInfo>) -> () {
        todo!()
    }

    fn on_list_update_position(&self, ids: Vec<u64>, position: usize) -> () {
        todo!()
    }

    fn on_list_music_add(&self, id: u64, musics: Vec<MusicInfo>, add_type: AddMusicLocation) -> () {
        todo!()
    }

    fn on_list_music_move(
        &self,
        from_id: String,
        to_id: String,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    ) -> () {
        todo!()
    }

    fn on_list_music_remove(&self, list_id: u64, ids: Vec<String>) -> () {
        todo!()
    }

    fn on_list_music_update(&self, id: String, music: MusicInfo) -> () {
        todo!()
    }

    fn on_list_music_update_position(&self, list_id: u64, position: usize, ids: Vec<String>) -> () {
        todo!()
    }

    fn on_list_music_overwrite(&self, list_id: u64, musics: Vec<MusicInfo>) -> () {
        todo!()
    }

    fn on_list_music_clear(&self, arg: Vec<String>) -> () {
        todo!()
    }
}

pub(in crate::server::socket) fn on_list_sync(ready: &AtomicBool, user_space: &UserSpace) {
    if !ready.load(Ordering::Relaxed) {
        return;
    }
    todo!()
}
