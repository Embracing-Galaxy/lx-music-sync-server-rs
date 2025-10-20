use crate::data::config::AddMusicLocation;
use crate::data::manager::Data;
use std::collections::HashSet;

pub(crate) type DislikeData = String;

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
