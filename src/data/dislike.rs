use crate::data::config::AddMusicLocation;
use crate::data::manager::Data;
use crate::server::socket::handler::DislikeSyncActionHandler;
use std::collections::HashSet;

pub(crate) type DislikeData = String;
pub(crate) type Name = String;
pub(crate) type Singer = String;

impl Data for DislikeData {
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn merge(&mut self, client: &Self, snapshot: &Self, _: &AddMusicLocation) {
        let current = self.lines().collect::<HashSet<_>>();
        let client = client.lines().collect::<HashSet<_>>();
        let snapshot = snapshot.lines().collect::<HashSet<_>>();

        // assert that the format is correct
        *self = current
            .union(&client)
            .filter(|&rule| {
                (current.contains(rule) && client.contains(rule)) || !snapshot.contains(rule)
            })
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl DislikeSyncActionHandler for DislikeData {
    fn on_overwrite(&mut self, data: DislikeData) {
        *self = data;
    }

    fn on_add(&mut self, data: Vec<(Name, Singer)>) {
        let new_rules: Vec<String> = data.into_iter().map(format_to_rule).collect();
        self.push_str(&new_rules.join("\n"));
    }

    fn on_clear(&mut self) {
        *self = Self::default();
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
