use super::*;
use crate::data::config::AddMusicLocation;
use crate::data::list::{de_list_id, CustomListInfo, ListData, MusicInfo};
use serde::Deserializer;

#[derive(Deserialize)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
pub(crate) enum ListSyncAction {
    ListDataOverwrite(ListData),
    ListCreate {
        position: i64,
        #[serde(rename = "listInfos")]
        infos: Vec<CustomListInfo>,
    },
    ListRemove(Vec<u64>),
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

pub(crate) trait ListSyncActionHandler {
    async fn on(&mut self, action: ListSyncAction) {
        match (action) {
            ListSyncAction::ListDataOverwrite(arg) => self.on_list_data_overwrite(arg).await,
            ListSyncAction::ListCreate { position, infos } => {
                self.on_list_create(position, infos).await
            }
            ListSyncAction::ListRemove(arg) => self.on_list_remove(arg).await,
            ListSyncAction::ListUpdate(arg) => self.on_list_update(arg).await,
            ListSyncAction::ListUpdatePosition { ids, position } => {
                self.on_list_update_position(ids, position).await
            }
            ListSyncAction::ListMusicAdd {
                id,
                musics,
                add_type,
            } => self.on_list_music_add(id, musics, add_type).await,
            ListSyncAction::ListMusicMove {
                from_id,
                to_id,
                musics,
                add_type,
            } => {
                self.on_list_music_move(from_id, to_id, musics, add_type)
                    .await
            }
            ListSyncAction::ListMusicRemove { list_id, ids } => {
                self.on_list_music_remove(list_id, ids).await
            }
            ListSyncAction::ListMusicUpdate { id, music } => {
                self.on_list_music_update(id, music).await
            }
            ListSyncAction::ListMusicUpdatePosition {
                list_id,
                position,
                ids,
            } => {
                self.on_list_music_update_position(list_id, position, ids)
                    .await
            }
            ListSyncAction::ListMusicOverwrite { list_id, musics } => {
                self.on_list_music_overwrite(list_id, musics).await
            }
            ListSyncAction::ListMusicClear(arg) => self.on_list_music_clear(arg).await,
        }
    }
    async fn on_list_data_overwrite(&mut self, arg: ListData);
    async fn on_list_create(&mut self, position: i64, infos: Vec<CustomListInfo>);
    async fn on_list_remove(&mut self, arg: Vec<u64>);
    async fn on_list_update(&mut self, arg: Vec<CustomListInfo>);
    async fn on_list_update_position(&mut self, ids: Vec<u64>, position: usize);
    async fn on_list_music_add(
        &mut self,
        id: u64,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    );
    async fn on_list_music_move(
        &mut self,
        from_id: String, // FIXME
        to_id: String,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    );
    async fn on_list_music_remove(&mut self, list_id: u64, ids: Vec<String>);
    async fn on_list_music_update(&mut self, id: String, music: MusicInfo);
    async fn on_list_music_update_position(
        &mut self,
        list_id: u64,
        position: usize,
        ids: Vec<String>,
    );
    async fn on_list_music_overwrite(&mut self, list_id: u64, musics: Vec<MusicInfo>);
    async fn on_list_music_clear(&mut self, arg: Vec<String>);
}

pub(in crate::server::socket) fn on_list_sync(ready: &AtomicBool, user_space: &UserSpace) {
    if !ready.load(Ordering::Relaxed) {
        return;
    }
    todo!()
}
