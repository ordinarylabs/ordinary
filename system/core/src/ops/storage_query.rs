use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
use saferlmdb::ReadTransaction;

/// [uuid][kind count][kinds][uuid][kind count][kinds]
pub fn req(
    token: &[u8],
    query: Vec<(&[u8; 16], Vec<u8>)>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    if query.len() > 255 {
        return Err("query cannot contain more than 255 entities".into());
    }

    let mut buf = BytesMut::new();

    buf.put(&token[..]);

    for (entity_uuid, kinds) in query {
        buf.put(&entity_uuid[..]);

        if kinds.len() > 255 {
            return Err("cannot have more than 255 kinds".into());
        }

        buf.put_u8(kinds.len() as u8);

        for kind in kinds {
            buf.put_u8(kind);
        }
    }

    Ok(buf.into())
}

/// (id, parent, group, key, content)
pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let len = bytes.len();

    if len < 73 {
        return Err("does not include token".into());
    }

    let (user_uuid, group_uuid) = token::verify_with_group(12, &bytes[0..73])?;

    // todo: 1 construct the set of keys you'll need to cursor over
    // todo: 2 verify that the group_uuid is valid for returned entities

    //     let mut cursor = txn.cursor(core.group_db.clone())?;

    //     cursor.seek_k::<[u8; 16], [u8; 16]>(&access, parent_group)?;

    //     let sub_groups: &[[u8; 16]] = cursor.get_multiple(&access)?;

    //     for sub_group in sub_groups {
    //         if sub_group == &group_uuid[..] {
    //             valid_group = true;
    //             break;
    //         }
    //     }

    let txn = ReadTransaction::new(core.env.clone())?;
    let access = txn.access();

    let mut cursor = txn
        .cursor(core.entity_db.clone())
        .expect("failed to create cursor");

    // todo: 3 build the output
    // for entity_uuid in entity_uuids {
    //     for kind in kinds {
    //         cursor.seek_k::<[u8], [u8]>(&access, uuid_and_kind)
    //     }
    // }

    Ok(Bytes::copy_from_slice(&[0]))
}
