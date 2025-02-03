use crate::Core;
use bytes::Bytes;
use cbwaw::token;
use saferlmdb::{put, WriteTransaction};
use uuid::Uuid;

pub fn req(access_token: &[u8]) -> Result<Bytes, Box<dyn std::error::Error>> {
    Ok(Bytes::copy_from_slice(access_token))
}

pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let len = bytes.len();

    if len < 57 {
        return Err("req does not include access token".into());
    }

    let user_uuid = token::verify_without_group(3, &bytes[0..57])?;

    let uuid = Uuid::new_v4();
    let group_uuid = uuid.as_bytes();

    let mut rule = Vec::with_capacity(33);

    rule.extend_from_slice(&user_uuid[..]);
    rule.extend_from_slice(&group_uuid[..]);
    // read/write
    rule.push(0);

    let txn = WriteTransaction::new(core.env.clone())?;

    {
        let mut access = txn.access();
        access.put::<[u8], [u8]>(&core.group_db, &rule, &[], put::Flags::empty())?;
    }

    Ok(Bytes::copy_from_slice(group_uuid))
}

pub fn res(res: Bytes) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    let group_uuid: [u8; 16] = res[0..16].try_into()?;
    Ok(group_uuid)
}
