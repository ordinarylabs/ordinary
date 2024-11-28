use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use saferlmdb::ReadTransaction;

/// refresh_token.action?group
pub fn req(
    refresh_token: &[u8],
    action: u8,
    group_uuid: Option<&[u8; 16]>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(if group_uuid.is_some() {
        57 + 16 + 1
    } else {
        57 + 1
    });

    buf.put(refresh_token);
    buf.put_u8(action);

    if let Some(group_uuid) = group_uuid {
        buf.put(&group_uuid[..]);
    }

    Ok(buf.into())
}

/// access token
pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let with_group = match bytes.len() {
        74 => true,
        58 => false,
        _ => return Err("invalid format".into()),
    };

    let user_uuid = cbwaw::token::verify_without_group(0, &bytes[..57])?;
    let action = bytes[57];

    // todo: convert this to all be fixed 33 byte array creation/assignment
    let mut check = Vec::with_capacity(32);

    check.extend_from_slice(&user_uuid[..]);
    check.push(action);

    if with_group {
        let group_uuid: [u8; 16] = bytes[57 + 1..57 + 1 + 16].try_into()?;

        check.extend_from_slice(&group_uuid[..]);

        let txn = ReadTransaction::new(core.env.clone())?;
        let access = txn.access();

        access.get::<[u8], [u8]>(&core.group_db.clone(), &check)?;

        cbwaw::token::gen_with_group(action, &user_uuid, &group_uuid)
    } else {
        cbwaw::token::gen_without_group(action, &user_uuid)
    }
}
