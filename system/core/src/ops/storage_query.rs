use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
use saferlmdb::ReadTransaction;

/// [parent][uuid][kind count][kinds][parent][uuid][kind count][kinds]
pub fn req(
    token: &[u8],
    query: Vec<(&[u8; 16], &[u8; 16], Vec<u8>)>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    if query.len() > 255 {
        return Err("query cannot contain more than 255 entities".into());
    }

    let mut buf = BytesMut::new();

    buf.put(&token[..]);

    for (parent_uuid, entity_uuid, kinds) in query {
        buf.put(&parent_uuid[..]);
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

    let mut query_cursor = 73;
    let mut keys = vec![];

    'outer: loop {
        if len == query_cursor {
            break 'outer;
        } else if len > query_cursor + 34 {
            return Err("invalid query".into());
        }

        let parent_uuid = [];
        query_cursor += 16;

        let entity_uuid = [];
        query_cursor += 16;

        let kind_count = bytes[query_cursor] as usize;
        query_cursor += 1;

        let end = query_cursor + kind_count;

        if len > end {
            return Err("invalid query".into());
        }

        'inner: loop {
            if query_cursor > end {
                break 'inner;
            }

            let mut buf = BytesMut::with_capacity(33);

            buf.put(&parent_uuid[..]);

            buf.put_u8(bytes[query_cursor]);
            query_cursor += 1;

            buf.put(&entity_uuid[..]);

            keys.push(buf);
        }
    }

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

    for key in keys {
        let v = cursor.seek_k::<[u8], [u8]>(&access, &key);

        loop {
            let (k, v) = cursor.next::<[u8], [u8]>(&access)?;

            if k[17] != key[17] {
                break;
            } else {
                // todo: cram into output
            }
        }
    }

    Ok(Bytes::copy_from_slice(&[0]))
}
