use crate::{Core, MAX_USERNAME_LEN};
use bytes::{BufMut, Bytes, BytesMut};
use saferlmdb::ReadTransaction;

/// username_len.username.client_start
/// (client_state, payload)
pub fn req(
    username: &[u8],
    password: &[u8],
) -> Result<(Vec<u8>, Bytes), Box<dyn std::error::Error>> {
    let (client_state, client_start) = cbwaw::login::client_start(password)?;

    let username_len = username.len();
    if username_len > MAX_USERNAME_LEN as usize {
        return Err("username is too long".into());
    }

    let mut buf = BytesMut::with_capacity(1 + username_len + client_start.len());

    buf.put_u8(username_len as u8);
    buf.put(&username[..]);
    buf.put(&client_start[..]);

    Ok((client_state, buf.into()))
}

/// (username, client_start)
pub fn handle(core: &Core, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let username_len = payload[0];
    if username_len > payload.len() as u8 - 2 {
        return Err("invalid format".into());
    }

    let username = payload[1..(username_len as usize) + 1].to_vec();
    let client_start = payload[username_len as usize + 1..].to_vec();

    let txn = ReadTransaction::new(core.env.clone())?;
    let access = txn.access();

    let user_uuid_password_file: &[u8] = access.get(&core.auth_db.clone(), &username)?;

    let (state, message) = cbwaw::login::server_start(
        &core.opaque,
        &username,
        &user_uuid_password_file[16..],
        &client_start,
    )?;

    let user_uuid: [u8; 16] = user_uuid_password_file[0..16].try_into()?;
    drop(access);

    let mut auth_state = core.auth_state.lock();
    auth_state.insert(user_uuid, state);
    drop(auth_state);

    Ok(message)
}
