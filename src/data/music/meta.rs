use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaBase {
    #[serde(rename = "songId")]
    pub(crate) song_id: String,
    #[serde(rename = "albumName")]
    pub(crate) album_name: String,
    #[serde(rename = "picUrl")]
    pub(crate) pic_url: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaLocal {
    #[serde(flatten)]
    pub(super) base: MusicMetaBase,
    pub(super) file_path: String,
    pub(super) ext: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaOnline;
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaKg;
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaTx;
#[derive(Clone, Deserialize, Serialize)]
pub(super) struct MusicMetaMg;
