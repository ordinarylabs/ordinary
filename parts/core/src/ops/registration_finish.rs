use crate::{Core, MAX_USERNAME_LEN};
use bytes::{BufMut, Bytes, BytesMut};
use saferlmdb::{put, WriteTransaction};
use uuid::Uuid;

/// username_len.username.client_finish
/// payload
pub fn req(
    username: &[u8],
    password: &[u8],
    client_state: &[u8],
    server_message: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let client_finish = cbwaw::registration::client_finish(password, client_state, server_message)?;

    let username_len = username.len();
    if username_len > MAX_USERNAME_LEN as usize {
        return Err("username is too long".into());
    }

    let mut buf = BytesMut::with_capacity(1 + username_len + client_finish.len());

    buf.put_u8(username_len as u8);
    buf.put(&username[..]);
    buf.put(&client_finish[..]);

    Ok(buf.into())
}

/// (username, client_finish)
pub fn handle(core: &Core, payload: Bytes) -> Result<(), Box<dyn std::error::Error>> {
    let username_len = payload[0];
    if username_len > payload.len() as u8 - 2 {
        return Err("invalid format".into());
    }

    let username = payload[1..(username_len as usize) + 1].to_vec();
    let client_finish = payload[username_len as usize + 1..].to_vec();

    let mut password_file = cbwaw::registration::server_finish(&client_finish)?;

    let uuid = Uuid::new_v4();
    let user_uuid = uuid.as_bytes();

    password_file.extend_from_slice(user_uuid);

    let txn = WriteTransaction::new(core.env.clone())?;

    {
        let mut access = txn.access();

        access.put(
            &core.auth_db,
            &username,
            &password_file,
            put::Flags::empty(),
        )?;

        access.put(
            &core.user_db,
            user_uuid,
            &username, // TODO: figure out what the user type should have
            put::Flags::empty(),
        )?;
    }

    txn.commit()?;

    Ok(())
}
