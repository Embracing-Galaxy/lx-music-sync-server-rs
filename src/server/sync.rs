use crate::data::config::CONFIG;
use crate::data::{list::ListData, user::UserSpace};
use crate::server::{dto::EnabledFeatures, socket::SocketContext, SERVER_CONTEXT};
use crate::utils::crypto::MD5;

#[allow(unreachable_code, unused_variables)]
pub(super) async fn sync_list_once(socket: &mut SocketContext, enabled_features: EnabledFeatures) {
    assert_eq!(enabled_features, EnabledFeatures::DEFAULT);
    let username = &socket.username;
    // already checked in main
    let user_space = SERVER_CONTEXT.get_user_space(username).unwrap();
    let add_location = &CONFIG
        .user_configs
        .get(username)
        .unwrap()
        .add_music_location;

    let client_data = get_client_list_data(socket).await;

    if let Some(snapshot) = user_space.get_snapshot(&socket.client_id).await {
        if !list_latest(socket, &user_space).await {
            let new_list_data = user_space
                .merge_list(&socket.client_id, &client_data, &snapshot, add_location)
                .await;
            set_client_list(socket, &new_list_data).await;
        }
    } else if !client_data.is_empty() {
        user_space
            .overwrite_list(&socket.client_id, client_data)
            .await;
    }

    socket.broadcast_sync_result().await;
    finished_sync(socket).await;
}

/// Used to prompt the client to start performing incremental sync
/// after manual sync is completed
async fn finished_sync(socket: &mut SocketContext) {
    socket.request("list_sync_finished", None).await.unwrap();
    socket.list_ready();
}

async fn get_client_list_data(socket: &mut SocketContext) -> ListData {
    let receiver = socket
        .request("list_sync_get_list_data", None)
        .await
        .unwrap();
    let resp = receiver.await.unwrap();
    resp.get_data().unwrap()
}

async fn get_client_list_md5(socket: &mut SocketContext) -> MD5 {
    let receiver = socket.request("list_sync_get_md5", None).await.unwrap();
    let resp = receiver.await.unwrap();
    let hex_str = resp.get_data::<String>().unwrap();

    debug_assert!(hex_str.len() == 32);
    let bytes = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .expect("hex decode error");
    bytes.try_into().unwrap()
}

async fn set_client_list(socket: &mut SocketContext, list_data: &Vec<u8>) {
    let data: serde_json::Value = serde_json::from_slice(&list_data).unwrap();
    socket
        .request("list_sync_set_list_data", Some(data))
        .await
        .unwrap();
}

#[allow(unreachable_code, unused_variables)]
async fn list_latest(socket: &mut SocketContext, user_space: &UserSpace) -> bool {
    let client_md5 = get_client_list_md5(socket).await;
    let snapshot_key = user_space
        .get_snapshot_key(unimplemented!("client id"))
        .await;
    let current_key = user_space.get_current_list_info_key().await;

    let latest = client_md5 == current_key;
    if latest
        && let Some(snapshot_key) = snapshot_key
        && snapshot_key != current_key
    {
        user_space.update_snapshot_key(&socket.client_id, current_key).await;
    }
    latest
}
