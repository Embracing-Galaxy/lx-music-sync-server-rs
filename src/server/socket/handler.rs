#![allow(unused)]

use crate::data::manager::Data;
use crate::data::user::UserSpace;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
pub(super) use {dislike::on_dislike_sync, list::on_list_sync};

mod dislike;
mod list;
