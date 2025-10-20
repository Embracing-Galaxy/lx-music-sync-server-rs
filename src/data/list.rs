use crate::data::{config::AddMusicLocation, manager::Data};
pub(crate) use music::MusicInfo;
use music::MusicSource;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct ListData {
    #[serde(rename = "defaultList")]
    default: Vec<MusicInfo>,
    #[serde(rename = "loveList")]
    love: Vec<MusicInfo>,
    #[serde(rename = "userList")]
    custom_lists: Vec<CustomList>,
}

impl ListData {
    #[inline]
    fn build_custom_map(&self) -> HashMap<u64, &CustomList> {
        self.custom_lists
            .iter()
            .map(|list| (list.id(), list))
            .collect()
    }
}

impl Data for ListData {
    #[inline]
    fn is_empty(&self) -> bool {
        self.default.is_empty() && self.love.is_empty() && self.custom_lists.is_empty()
    }

    /// It may be a bit ugly here,
    /// but it's not a hot spot because it's only called once when the socket is connected.
    fn merge(&mut self, client: &Self, snapshot: &Self, add_location: &AddMusicLocation) {
        let current_custom_map = self.build_custom_map();
        let client_custom_map = client.build_custom_map();
        let snapshot_custom_map = snapshot.build_custom_map();
        let deleted_ids: HashSet<u64> = snapshot
            .custom_lists
            .iter()
            .map(CustomList::id)
            .filter(|id| {
                !current_custom_map.contains_key(id) || !client_custom_map.contains_key(id)
            })
            .collect();

        let mut custom_lists = Vec::new();

        for current_list in self.custom_lists.iter() {
            if deleted_ids.contains(&current_list.id()) {
                continue;
            }

            let merged_list = match client_custom_map.get(&current_list.id()) {
                Some(&client_list) => match snapshot_custom_map.get(&current_list.id()) {
                    None => current_list.merge(client_list, add_location),
                    Some(&snapshot_list) => {
                        current_list.merge_with_snapshot(client_list, snapshot_list, add_location)
                    }
                },
                None => current_list.clone(),
            };
            custom_lists.push(merged_list)
        }

        for (index, client_list) in client.custom_lists.iter().enumerate() {
            if deleted_ids.contains(&client_list.id()) {
                continue;
            }
            let client_update_time = client_list.info.location_update_time.unwrap_or(0);

            match custom_lists
                .iter()
                .position(|list| list.id() == client_list.id())
            {
                None => {
                    if client_update_time > 0 {
                        custom_lists.insert(index, client_list.clone());
                    } else {
                        custom_lists.push(client_list.clone());
                    }
                }
                Some(current_list_index) => {
                    let current_update_time = custom_lists[current_list_index]
                        .info
                        .location_update_time
                        .unwrap_or(0);
                    if current_update_time >= client_update_time {
                        continue;
                    }
                    // TODO optimize!
                    let mut current_list = custom_lists.remove(current_list_index);
                    current_list.info.location_update_time = Some(current_update_time);
                    custom_lists.insert(index, current_list);
                }
            }
        }

        self.default = merge_vec(
            &self.default,
            &client.default,
            &snapshot.default,
            add_location,
        );
        self.love = merge_vec(&self.love, &client.love, &snapshot.love, add_location);
        self.custom_lists = custom_lists;
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct CustomList {
    #[serde(flatten)]
    info: CustomListInfo,
    list: Vec<MusicInfo>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct CustomListInfo {
    #[serde(deserialize_with = "de_list_id")]
    id: u64,
    name: String,
    source: Option<MusicSource>, // TODO Usually None, its role is not yet clear
    source_list_id: Option<String>,
    location_update_time: Option<u128>,
}

/// deserialize "userlist_<some number>" to u64
pub(crate) fn de_list_id<'de, D: Deserializer<'de>>(de: D) -> Result<u64, D::Error> {
    let raw_str = String::deserialize(de)?;
    const PREFIX_LEN: usize = "userlist_".len();
    raw_str[PREFIX_LEN..]
        .parse()
        .map_err(serde::de::Error::custom)
}

impl CustomList {
    #[inline]
    fn id(&self) -> u64 {
        self.info.id
    }

    #[inline]
    fn merge(&self, client_list: &Self, add_location: &AddMusicLocation) -> Self {
        CustomList {
            info: self.info.clone(),
            list: combine_without_duplication(&self.list, &client_list.list, add_location),
        }
    }

    #[inline]
    fn merge_with_snapshot(
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
        fn select_data<'a, T: PartialEq + Clone>(current: &'a T, client: &'a T, snapshot: &T) -> T {
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

fn merge_vec(
    current: &Vec<MusicInfo>,
    client: &Vec<MusicInfo>,
    snapshot: &Vec<MusicInfo>,
    add_location: &AddMusicLocation,
) -> Vec<MusicInfo> {
    debug_assert!(!current.is_empty());
    debug_assert!(!client.is_empty());

    let current_ids: HashSet<&str> = current.iter().map(MusicInfo::get_id).collect();
    let client_ids: HashSet<&str> = client.iter().map(MusicInfo::get_id).collect();
    let deleted_ids: HashSet<&str> = snapshot
        .iter()
        .map(MusicInfo::get_id)
        .filter(|&id| !current_ids.contains(id) || !client_ids.contains(id))
        .collect();

    let new = client
        .iter()
        .filter(|info| {
            let id = &info.get_id();
            // Remove the deleted and duplicated infos
            !deleted_ids.contains(id) && !current_ids.contains(id)
        })
        .cloned();
    let current = current
        .iter()
        .filter(|info| !deleted_ids.contains(info.get_id()))
        .cloned();

    match add_location {
        AddMusicLocation::TOP => new.chain(current).collect(),
        AddMusicLocation::BOTTOM => current.chain(new).collect(),
    }
}

fn combine_without_duplication(
    a: &Vec<MusicInfo>,
    b: &Vec<MusicInfo>,
    add_location: &AddMusicLocation,
) -> Vec<MusicInfo> {
    let base = a.iter().cloned();
    let seen: HashSet<&str> = a.iter().map(MusicInfo::get_id).collect();
    let new = b
        .iter()
        .filter(|&info| !seen.contains(info.get_id()))
        .cloned();

    match add_location {
        AddMusicLocation::TOP => new.chain(base).collect(),
        AddMusicLocation::BOTTOM => base.chain(new).collect(),
    }
}

mod music {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize)]
    pub(crate) struct MusicInfo {
        id: String, // only id is used in list
        name: String,
        singer: String,
        source: MusicSource,
        interval: serde_json::Value,
        meta: serde_json::Value,
    }

    impl MusicInfo {
        pub(super) fn get_id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub(super) enum MusicSource {
        KW,
        KG,
        TX,
        WY,
        MG,
        LOCAL,
    }
}

#[cfg(test)]
mod tests {
    use super::de_list_id;
    use serde::de::value::{Error, StrDeserializer};

    #[test]
    fn deserialize_custom_list_id() {
        let id = "userlist_12345";
        let de = StrDeserializer::<Error>::new(id);
        let id: u64 = de_list_id(de).expect("should parse");
        assert_eq!(id, 12345);
    }
}
