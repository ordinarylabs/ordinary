use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use saferlmdb::ReadTransaction;

/// refresh_token.len() + group_uuid.len() + action.len()
const PAYLOAD_SIZE: usize = 57 + 16 + 1;

/// refresh_token.group.action
pub fn req(
    refresh_token: &[u8],
    group: &[u8; 16],
    action: u8,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(PAYLOAD_SIZE);

    buf.put(refresh_token);
    buf.put(&group[..]);
    buf.put_u8(action);

    Ok(buf.into())
}

/// access token
pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    if bytes.len() != 57 + 16 + 1 {
        return Err("invalid format".into());
    }

    let user_uuid = cbwaw::token::verify_without_group(0, &bytes[..57])?;
    let group_uuid: [u8; 16] = bytes[57..57 + 16].try_into()?;
    let action = bytes[57 + 16];

    let mut check = Vec::with_capacity(32);

    check.extend_from_slice(&user_uuid[..]);
    check.extend_from_slice(&group_uuid[..]);
    check.push(action);

    let txn = ReadTransaction::new(core.env.clone())?;
    let access = txn.access();

    access.get::<[u8], [u8]>(&core.group_db.clone(), &check)?;

    cbwaw::token::gen_with_group(action, &user_uuid, &group_uuid)
}
