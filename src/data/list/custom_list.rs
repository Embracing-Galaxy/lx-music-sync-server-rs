
use crate::data::config::AddMusicLocation;
use crate::data::list::de_list_id;
use crate::data::list::music::MusicSource;
use crate::data::list::{combine_without_duplication, MusicInfo};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct CustomList {
    #[serde(flatten)]
    pub(super) info: CustomListInfo,
    pub(super) list: Vec<MusicInfo>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct CustomListInfo {
    #[serde(deserialize_with = "de_list_id")]
    pub(super) id: u64,
    name: String,
    source: Option<MusicSource>, // TODO Usually None, its role is not yet clear
    source_list_id: Option<String>,
    pub(super) location_update_time: Option<u128>,
}

impl CustomList {
    #[inline]
    pub(super) fn id(&self) -> u64 {
        self.info.id
    }

    #[inline]
    pub(super) fn merge(&self, client_list: &Self, add_location: &AddMusicLocation) -> Self {
        CustomList {
            info: self.info.clone(),
            list: combine_without_duplication(&self.list, &client_list.list, add_location),
        }
    }

    #[inline]
    pub(super) fn merge_with_snapshot(
        &self,
        client_list: &Self,
        snapshot_list: &Self,
        add_location: &AddMusicLocation,
    ) -> Self {
        CustomList {
            info: self.info.merge(&client_list.info, &snapshot_list.info),
            list: combine_without_duplication(&self.list, &client_list.list, add_location),
        }
    }
}

impl CustomListInfo {
    #[inline]
    fn merge(&self, client_list: &Self, snapshot_list: &Self) -> Self {
        fn select_data<'a, T: PartialEq + Clone>(
            current: &'a T,
            client: &'a T,
            snapshot: &T,
        ) -> T {
            if current == snapshot { client } else { current }.clone()
        }
        Self {
            id: self.id,
            name: select_data(&self.name, &client_list.name, &snapshot_list.name),
            source: select_data(&self.source, &client_list.source, &snapshot_list.source),
            source_list_id: select_data(
                &self.source_list_id,
                &client_list.source_list_id,
                &snapshot_list.source_list_id,
            ),
            location_update_time: self.location_update_time,
        }
    }
}