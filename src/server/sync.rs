use crate::data::config::AddMusicLocation;
use crate::data::user::UserSpace;
use crate::server::socket::SocketContext;

pub(super) async fn sync_once(
    socket: &SocketContext,
    user_space: &'static UserSpace,
    add_location: &AddMusicLocation,
) {
    list::sync_once(socket, user_space, add_location).await;
    dislike::sync_once(socket, user_space, add_location).await;
}

#[macro_use]
mod template;

mod list {
    sync_once_for!(list);
}
mod dislike {
    sync_once_for!(dislike);
}
