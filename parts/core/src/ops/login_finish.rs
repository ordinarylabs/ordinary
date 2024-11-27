use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use saferlmdb::ReadTransaction;

const MAX_USERNAME_LEN: u8 = 255;

/// username_len.username.client_finish
/// payload
pub fn req(
    username: &[u8],
    password: &[u8],
    client_state: &[u8],
    server_message: &[u8],
) -> Result<(Bytes, Vec<u8>), Box<dyn std::error::Error>> {
    let (client_finish, session_key) =
        cbwaw::login::client_finish(password, client_state, server_message)?;

    let username_len = username.len();
    if username_len > MAX_USERNAME_LEN as usize {
        return Err("username is too long".into());
    }

    let mut buf = BytesMut::with_capacity(1 + username_len + client_finish.len());

    buf.put_u8(username_len as u8);
    buf.put(&username[..]);
    buf.put(&client_finish[..]);

    Ok((buf.into(), session_key))
}

/// (username, client_finish)
pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let username_len = bytes[0];
    if username_len > bytes.len() as u8 - 2 {
        return Err("invalid format".into());
    }

    let username = bytes[1..(username_len as usize) + 1].to_vec();
    let client_finish = bytes[username_len as usize + 1..].to_vec();

    let txn = ReadTransaction::new(core.env.clone())?;
    let access = txn.access();

    let user_uuid_password_file: &[u8] = access.get(&core.auth_db.clone(), &username)?;

    let user_uuid: [u8; 16] = user_uuid_password_file[0..16].try_into()?;
    drop(access);

    let mut auth_state = core.auth_state.lock();

    if let Some(server_start) = auth_state.get(&user_uuid) {
        let refresh_token = cbwaw::login::server_finish(&user_uuid, &client_finish, &server_start)?;
        auth_state.remove(&user_uuid);
        drop(auth_state);

        return Ok(refresh_token);
    }

    Ok(Bytes::new())
}
