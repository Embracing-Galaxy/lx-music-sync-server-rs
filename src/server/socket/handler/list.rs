use super::*;
use crate::data::config::AddMusicLocation;
use crate::data::list::{de_list_id, CustomListInfo, ListData, MusicInfo};
use serde::Deserializer;
use std::collections::HashSet;
use ListSyncAction::*;

#[derive(Deserialize)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
pub(crate) enum ListSyncAction {
    ListDataOverwrite(ListData),
    ListCreate {
        position: i64,
        #[serde(rename = "listInfos")]
        infos: Vec<CustomListInfo>,
    },
    ListRemove {
        #[serde(deserialize_with = "de_list_ids")]
        to_remove: HashSet<u64>,
    },
    ListUpdate(Vec<CustomListInfo>),
    ListUpdatePosition {
        #[serde(deserialize_with = "de_list_ids")]
        ids: HashSet<u64>,
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
        #[serde(rename = "fromId", deserialize_with = "de_list_id")]
        src_list_id: u64,
        #[serde(rename = "toId", deserialize_with = "de_list_id")]
        dst_list_id: u64,
        #[serde(rename = "musicInfos")]
        musics: Vec<MusicInfo>,
        #[serde(rename = "addMusicLocationType")]
        add_type: AddMusicLocation,
    },
    ListMusicRemove {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        ids: HashSet<String>,
    },
    ListMusicUpdate(Vec<MusicUpdateInfo>),
    ListMusicUpdatePosition {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        position: usize,
        ids: HashSet<String>,
    },
    ListMusicOverwrite {
        #[serde(rename = "listId", deserialize_with = "de_list_id")]
        list_id: u64,
        #[serde(rename = "musicInfos")]
        musics: Vec<MusicInfo>,
    },
    ListMusicClear {
        #[serde(deserialize_with = "de_list_ids")]
        list_ids: HashSet<u64>,
    },
}

#[derive(Deserialize)]
pub(crate) struct MusicUpdateInfo {
    #[serde(deserialize_with = "de_list_id")]
    pub(crate) id: u64,
    #[serde(rename = "musicInfo")]
    pub(crate) music: MusicInfo,
}

fn de_list_ids<'de, D: Deserializer<'de>>(de: D) -> Result<HashSet<u64>, D::Error> {
    let raw_vec: HashSet<String> = HashSet::deserialize(de)?;
    const PREFIX: usize = "userlist_".len();
    raw_vec
        .into_iter()
        .map(|s| s[PREFIX..].parse().map_err(serde::de::Error::custom))
        .collect()
}

impl JsonReqHandler for ListData {
    fn on(&mut self, action: serde_json::Value) {
        match serde_json::from_value(action).unwrap() {
            ListDataOverwrite(arg) => self.list_data_overwrite(arg),
            ListCreate { position, infos } => self.list_create(position, infos),
            ListRemove { to_remove } => self.list_remove(to_remove),
            ListUpdate(arg) => self.list_update(arg),
            ListUpdatePosition { ids, position } => self.list_sort(ids, position),
            ListMusicAdd {
                id,
                musics,
                add_type,
            } => self.music_add(id, musics, add_type),
            ListMusicMove {
                src_list_id: src_id,
                dst_list_id: dst_id,
                musics,
                add_type,
            } => self.music_move(src_id, dst_id, musics, add_type),
            ListMusicRemove { list_id, ids } => self.music_remove(list_id, ids),

            ListMusicUpdate(data) => self.music_update(data),
            ListMusicUpdatePosition {
                list_id,
                position,
                ids,
            } => self.music_sort(list_id, position, ids),
            ListMusicOverwrite { list_id, musics } => self.music_overwrite(list_id, musics),
            ListMusicClear { list_ids } => self.list_clear(list_ids),
        }
    }
}

pub(crate) trait ListSyncActionHandler {
    fn list_data_overwrite(&mut self, arg: ListData);
    fn list_create(&mut self, position: i64, infos: Vec<CustomListInfo>);
    fn list_remove(&mut self, to_remove: HashSet<u64>);
    fn list_update(&mut self, arg: Vec<CustomListInfo>);
    fn list_sort(&mut self, ids: HashSet<u64>, position: usize);
    fn list_clear(&mut self, list_ids: HashSet<u64>);
    fn music_add(&mut self, list_id: u64, musics: Vec<MusicInfo>, add_type: AddMusicLocation);
    fn music_move(
        &mut self,
        src_list_id: u64,
        dst_list_id: u64,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    );
    fn music_remove(&mut self, list_id: u64, ids: HashSet<String>);
    fn music_update(&mut self, data: Vec<MusicUpdateInfo>);
    fn music_sort(&mut self, list_id: u64, position: usize, ids: HashSet<String>);
    fn music_overwrite(&mut self, list_id: u64, musics: Vec<MusicInfo>);
}
