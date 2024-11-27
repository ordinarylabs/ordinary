use bytes::{BufMut, Bytes, BytesMut};

const MAX_USERNAME_LEN: u8 = 255;

/// username_len.username.client_start
/// (client_state, payload)
pub fn new(
    username: &[u8],
    password: &[u8],
) -> Result<(Vec<u8>, Bytes), Box<dyn std::error::Error>> {
    let (client_state, client_start) = cbwaw::registration::client_start(password)?;

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
pub fn process(bytes: Bytes) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let username_len = bytes[0];
    if username_len > bytes.len() as u8 - 2 {
        return Err("invalid format".into());
    }

    let username = bytes[1..(username_len as usize) + 1].to_vec();
    let client_start = bytes[username_len as usize + 1..].to_vec();

    Ok((username, client_start))
}
