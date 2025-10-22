use super::{AddMusicLocation, CustomList, CustomListInfo, ListData, MusicInfo};
use crate::server::socket::ListSyncActionHandler;
use crate::utils::now_ms;
use std::collections::{HashMap, HashSet};

#[allow(unused)]
impl ListSyncActionHandler for ListData {
    async fn on_list_data_overwrite(&mut self, data: ListData) {
        *self = data;
    }

    async fn on_list_create(&mut self, position: i64, infos: Vec<CustomListInfo>) {
        let position = usize::try_from(position).unwrap_or(0);
        if infos.len() == 1 {
            let info = infos.into_iter().next().unwrap();
            if self.custom_lists.iter().any(|list| list.id() == info.id) {
                return;
            }

            let new_list = CustomList { info, list: vec![] };

            if position >= self.custom_lists.len() {
                self.custom_lists.push(new_list);
            } else {
                self.custom_lists.insert(position, new_list);
            }
            return;
        }

        let existed_ids = self
            .custom_lists
            .iter()
            .map(CustomList::id)
            .collect::<HashSet<_>>();
        let mut info = infos
            .into_iter()
            .filter(|list_info| !existed_ids.contains(&list_info.id))
            .map(|list_info| CustomList {
                info: list_info,
                list: vec![],
            })
            .collect::<Vec<_>>();
        if position >= self.custom_lists.len() {
            self.custom_lists.append(&mut info);
        } else {
            self.custom_lists.splice(position..position, info);
        }
    }

    async fn on_list_remove(&mut self, ids: Vec<u64>) {
        if let [id] = ids.as_slice() {
            if let Some(pos) = self.custom_lists.iter().position(|list| list.id() == *id) {
                self.custom_lists.remove(pos);
            }
        } else {
            let to_remove: HashSet<u64> = ids.into_iter().collect();
            self.custom_lists
                .retain(|custom_list| !to_remove.contains(&custom_list.id()))
        }
    }

    async fn on_list_update(&mut self, infos: Vec<CustomListInfo>) {
        if infos.len() == 1 {
            let info = infos.into_iter().next().unwrap();
            if let Some(pos) = self
                .custom_lists
                .iter()
                .position(|list| list.id() == info.id)
            {
                self.custom_lists[pos].info = info;
            }
        } else {
            let mut map: HashMap<_, _> = infos.into_iter().map(|list| (list.id, list)).collect();
            for list in self.custom_lists.iter_mut() {
                if let Some(info) = map.remove(&list.id()) {
                    list.info = info;
                    if map.is_empty() {
                        return;
                    }
                }
            }
        }
    }

    async fn on_list_update_position(&mut self, ids: Vec<u64>, position: usize) {
        let now = now_ms();
        todo!()
    }

    async fn on_list_music_add(
        &mut self,
        id: u64,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    ) {
        todo!()
    }

    async fn on_list_music_move(
        &mut self,
        from_id: String,
        to_id: String,
        musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    ) {
        todo!()
    }

    async fn on_list_music_remove(&mut self, list_id: u64, ids: Vec<String>) {
        todo!()
    }

    async fn on_list_music_update(&mut self, id: String, music: MusicInfo) {
        todo!()
    }

    async fn on_list_music_update_position(
        &mut self,
        list_id: u64,
        position: usize,
        ids: Vec<String>,
    ) {
        todo!()
    }

    async fn on_list_music_overwrite(&mut self, list_id: u64, musics: Vec<MusicInfo>) {
        todo!()
    }

    async fn on_list_music_clear(&mut self, arg: Vec<String>) {
        todo!()
    }
}
