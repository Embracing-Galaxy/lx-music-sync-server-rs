use super::{AddMusicLocation, CustomList, CustomListInfo, ListData, MusicInfo};
use crate::server::socket::ListSyncActionHandler;
use crate::utils::now_ms;
use std::collections::{HashMap, HashSet};

impl ListData {
    fn get_music_list(&mut self, id: u64) -> Option<&mut Vec<MusicInfo>> {
        match id {
            0 => Some(&mut self.default),
            1 => Some(&mut self.love),
            _ => self
                .custom_lists
                .iter_mut()
                .find(|list| list.id() == id)
                .map(|custom_list| &mut custom_list.list),
        }
    }
}

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

    async fn on_list_remove(&mut self, to_remove: HashSet<u64>) {
        if to_remove.len() == 1 {
            let id = to_remove.into_iter().next().unwrap();
            if let Some(pos) = self.custom_lists.iter().position(|list| list.id() == id) {
                self.custom_lists.remove(pos);
            }
        } else {
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

    async fn on_list_update_position(&mut self, ids: HashSet<u64>, position: usize) {
        let (mut to_keep, mut to_move): (Vec<_>, Vec<_>) = self
            .custom_lists
            .drain(..)
            .partition(|item| !ids.contains(&item.id()));

        let now = Some(now_ms());
        for custom_list in to_move.iter_mut() {
            custom_list.info.location_update_time = now;
        }

        if position < self.custom_lists.len() {
            to_keep.splice(position..position, to_move);
        } else {
            to_keep.extend(to_move);
        }
        self.custom_lists = to_keep;
    }

    async fn on_list_music_add(
        &mut self,
        list_id: u64,
        mut musics: Vec<MusicInfo>,
        add_type: AddMusicLocation,
    ) {
        let Some(target_list) = self.get_music_list(list_id) else {
            return;
        };
        if add_type == AddMusicLocation::TOP {
            std::mem::swap(target_list, &mut musics);
        }
        target_list.extend(musics);
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

    async fn on_list_music_remove(&mut self, list_id: u64, ids: HashSet<String>) {
        let Some(target_list) = self.get_music_list(list_id) else {
            return;
        };
        target_list.retain(|info| !ids.contains(info.get_id()));
    }

    async fn on_list_music_update(&mut self, id: String, music: MusicInfo) {
        todo!()
    }

    async fn on_list_music_update_position(
        &mut self,
        list_id: u64,
        position: usize,
        ids: HashSet<String>,
    ) {
        let Some(target_list) = self.get_music_list(list_id) else {
            return;
        };

        let (mut to_keep, to_move): (Vec<_>, Vec<_>) = target_list
            .drain(..)
            .partition(|info| !ids.contains(info.get_id()));

        if position < target_list.len() {
            to_keep.splice(position..position, to_move);
        } else {
            to_keep.extend(to_move);
        }
        *target_list = to_keep;
    }

    async fn on_list_music_overwrite(&mut self, list_id: u64, musics: Vec<MusicInfo>) {
        if let Some(target_list) = self.get_music_list(list_id) {
            *target_list = musics;
        }
    }

    async fn on_list_music_clear(&mut self, list_ids: HashSet<u64>) {
        for list_id in list_ids {
            if let Some(target_list) = self.get_music_list(list_id) {
                *target_list = Vec::new();
            }
        }
    }
}
