macro_rules! sync_once_for {
    ($name:ident) => {
        use crate::data::{config::AddMusicLocation, manager::{DataType, SnapshotKey}, user::UserSpace};
        use crate::server::socket::SocketContext;
        use crate::utils::crypto::{hex_to_md5, MD5};
        use paste::paste;

        pub(super) async fn sync_once(
            socket: &SocketContext,
            user_space: &'static UserSpace,
            add_location: &AddMusicLocation,
        ) {
            let callback = socket.request(concat!(stringify!($name), "_sync_get_list_data"), vec![]).await;
            let resp = callback.await.unwrap();
            let client_data = resp.get_data().unwrap();

            if let Some(snapshot) = user_space.$name.get_snapshot(&socket.client_id).await {
                if !latest(socket, &user_space).await {
                    let (new_data, new_key) = user_space
                    .$name
                    .merge(&socket.client_id, &client_data, &snapshot, add_location)
                    .await;

                    let new_data: serde_json::Value = serde_json::from_slice(&new_data).unwrap();
                    set_client_data(socket, &new_data).await;
                    broadcast(socket, new_data, new_key).await;
                }
            } else {
                user_space.$name.overwrite_from_client(&socket.client_id, client_data).await;
            }

            // Used to prompt the client to start performing incremental sync
            // after manual sync is completed
            socket.post(concat!(stringify!($name), "_sync_finished"), vec![]).await;
            paste! {
                socket.[<$name _ready>]();
            }
        }

        #[inline]
        async fn set_client_data(socket: &SocketContext, data: &serde_json::Value) {
            socket.post(concat!(stringify!($name), "_sync_set_list_data"), vec![data.clone()]).await;
        }

        const TYPE: DataType = paste!(DataType::[<$name:upper>]);
        #[inline]
        async fn broadcast(socket: &SocketContext, data: serde_json::Value, key: SnapshotKey) {
            let broadcast_data = serde_json::json!({
                "action": concat!(stringify!($name), "_data_overwrite"),
                "data": data
            });
            socket.broadcast(TYPE, broadcast_data, key);
        }

        #[inline]
        async fn get_client_md5(socket: &SocketContext) -> MD5 {
            let callback = socket.request(concat!(stringify!($name), "_sync_get_md5"), vec![]).await;
            let resp = callback.await.unwrap();
            let hex_str = resp.get_data::<String>().unwrap();
            hex_to_md5(&hex_str)
        }

        #[inline]
        async fn latest(socket: &SocketContext, user_space: &'static UserSpace) -> bool {
            let client_md5 = get_client_md5(socket).await;
            let client_id = &socket.client_id;
            let snapshot_key = user_space.$name.get_snapshot_key(client_id).await;
            let current_key = user_space.$name.get_info_key().await;

            let latest = client_md5 == current_key;
            if latest && let Some(snapshot_key) = snapshot_key && snapshot_key != current_key{
                user_space.$name.update_snapshot_key(client_id, current_key).await;
            }
            latest
        }
    }
}