#![allow(warnings)]
mod meta;

use meta::{MusicMetaKg, MusicMetaLocal, MusicMetaMg, MusicMetaOnline, MusicMetaTx};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) enum MusicInfo {
    Local(MusicInfoLocal),
    Online(MusicInfoOnline),
}

impl MusicInfo {
    pub(crate) fn get_id(&self) -> &str {
        match self {
            Self::Local(info) => &info.id,
            Self::Online(info) => info.get_id(),
        }
    }
}

type MusicInfoLocal = MusicInfoBase<MusicMetaLocal>;
type MusicInfoOnlineCommon = MusicInfoBase<MusicMetaOnline>;
type MusicInfoKg = MusicInfoBase<MusicMetaKg>;
type MusicInfoTx = MusicInfoBase<MusicMetaTx>;
type MusicInfoMg = MusicInfoBase<MusicMetaMg>;

#[derive(Clone, Deserialize, Serialize)]
pub struct MusicInfoBase<M> {
    pub id: String,
    pub name: String,
    pub singer: String,
    pub source: MusicSource,
    pub interval: Option<String>,
    pub meta: M,
}

impl<M> MusicInfoBase<M> {
    fn get_id(&self) -> &String {
        &self.id
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub(super) enum MusicSource {
    KW,
    KG,
    TX,
    WY,
    MG,
    LOCAL,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum MusicInfoOnline {
    Common(MusicInfoOnlineCommon),
    Kg(MusicInfoKg),
    Tx(MusicInfoTx),
    Mg(MusicInfoMg),
}

impl MusicInfoOnline {
    fn get_id(&self) -> &String {
        match self {
            MusicInfoOnline::Common(info) => &info.id,
            MusicInfoOnline::Kg(info) => &info.id,
            MusicInfoOnline::Tx(info) => &info.id,
            MusicInfoOnline::Mg(info) => &info.id,
        }
    }
}
