use crate::utils::load_or_create;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

pub(crate) static CONFIG: LazyLock<Config> =
    LazyLock::new(|| load_or_create(Path::new("config.json")));

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) server_name: String,
    pub(crate) enable_proxy: bool,
    pub(crate) user_configs: HashMap<String, UserConfig>, // username -> user_config
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct UserConfig {
    pub(crate) password: String,
    pub(crate) max_snapshot_count: u64,
    pub(crate) add_music_location: AddMusicLocation,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) enum AddMusicLocation {
    #[default]
    TOP,
    BOTTOM,
}
