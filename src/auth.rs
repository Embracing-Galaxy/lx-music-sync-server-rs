use crate::data::config::CONFIG;
use crate::data::user::UserSpace;
use crate::data::{user::DeviceInfo, ClientId};
use crate::server::SERVER_CONTEXT;
use crate::utils::crypto::{
    aes_decrypt_with_base64, aes_encrypt_with_base64, md5_to_hex, rsa_encrypt_with_base64, to_md5,
};
use base64::prelude::{Engine, BASE64_STANDARD};
use log::info;

const AUTH_HEAD: &str = "lx-music auth::";
const AUTH_HEAD_LENGTH: usize = AUTH_HEAD.len();
const INCORRECT_FORMAT: &str = "Incorrect auth format";
const NO_SUCH_USER: &str = "No such user";

pub(crate) async fn auth_by_key(
    encrypt_msg: &str,
    client_id: &ClientId,
    user_space: Option<&UserSpace>,
) -> Result<String, &'static str> {
    let (user_space, device_info) = if let Some(user_space) = user_space
        && let Some(device_info) = user_space.get_client_device_info(client_id).await
    {
        (user_space, device_info)
    } else {
        return Err(NO_SUCH_USER);
    };
    let key = &device_info.key;
    let text = aes_decrypt_with_base64(encrypt_msg, key);
    if !text.starts_with(AUTH_HEAD) {
        return Err(INCORRECT_FORMAT);
    }

    let device_name = &text[AUTH_HEAD_LENGTH..];
    user_space.update_device_name(client_id, device_name).await;
    Ok(aes_encrypt_with_base64("Hello~::^-^::~v4~", key))
}

pub(crate) async fn auth_by_code(encrypt_msg: &str) -> Result<String, &'static str> {
    for (username, user_config) in CONFIG.user_configs.iter() {
        let hex_key = md5_to_hex(&to_md5(&user_config.password), 16);
        let key = BASE64_STANDARD.encode(hex_key.as_bytes());
        let text = aes_decrypt_with_base64(encrypt_msg, &key);
        if !text.starts_with(AUTH_HEAD) {
            continue;
        }
        let mut lines = text.lines();
        let key_body = lines.nth(1).unwrap_or(""); // Line 1 (index 1)
        let public_key = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            key_body
        );

        let device_name = lines.next().unwrap_or("Unknown"); // Line 2 (index 2)
        let is_mobile = lines.next().is_some_and(|s| s == "lx_music_mobile"); // Line 3 (index 3)
        let device_info = DeviceInfo::new(device_name.to_string(), is_mobile);
        let user_space = SERVER_CONTEXT.get_user_space(username).unwrap();
        let result = rsa_encrypt_with_base64(
            &format!(
                r#"{{"clientId":"{}","key":"{}","serverName":"{}"}}"#,
                device_info.client_id, device_info.key, CONFIG.server_name
            ),
            &public_key,
        );
        info!("Device {:?} first connected", device_info);
        user_space.insert_device_info(device_info);
        return Ok(result);
    }
    Err(NO_SUCH_USER)
}
