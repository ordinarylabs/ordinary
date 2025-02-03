use crate::Core;
use bitcode::{Decode, Encode};
use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
use saferlmdb::ReadTransaction;
use std::collections::BTreeMap;

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct Entity {
    uuid: [u8; 16],
    user: [u8; 16],
    value: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct QueryResult {
    /// uuid -> kind -> entity[]
    pub entities: BTreeMap<[u8; 16], BTreeMap<u8, Vec<Entity>>>,
}

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

pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let len = bytes.len();
    let mut query_cursor = 73;

    if len < query_cursor {
        return Err("does not include token".into());
    }

    let (_, group_uuid) = token::verify_with_group(13, &bytes[0..query_cursor])?;

    let txn = ReadTransaction::new(core.env.clone())?;
    let access = txn.access();

    let mut entity_cursor = txn
        .cursor(core.entity_db.clone())
        .expect("failed to create entity cursor");

    let mut query_result = QueryResult {
        entities: BTreeMap::new(),
    };

    'outer: loop {
        if len == query_cursor {
            break 'outer;
        } else if len > query_cursor + 34 {
            return Err("invalid query".into());
        }

        let parent_uuid = &bytes[query_cursor..query_cursor + 16];
        query_cursor += 16;

        let entity_uuid = &bytes[query_cursor..query_cursor + 16];
        query_cursor += 16;

        let kind_count = bytes[query_cursor] as usize;
        query_cursor += 1;

        let end = query_cursor + kind_count;

        if len > end {
            return Err("invalid query".into());
        }

        'inner: loop {
            let mut root_key = [0u8; 33];

            root_key[0..16].copy_from_slice(&parent_uuid[..]);
            root_key[17] = bytes[query_cursor];
            root_key[17..33].copy_from_slice(&entity_uuid[..]);

            loop {
                if query_cursor > end {
                    break 'inner;
                }

                let mut start_key = [0u8; 17];

                start_key[0..16].copy_from_slice(&entity_uuid[..]);
                start_key[16] = bytes[query_cursor];
                query_cursor += 1;

                let (mut entity_key, mut entity_value) =
                    entity_cursor.seek_k_both::<[u8], [u8]>(&access, &start_key)?;

                loop {
                    if entity_key[17] != start_key[16] {
                        break;
                    } else {
                        let mut rule = [0u8; 33];

                        rule[0..16].copy_from_slice(&entity_key);
                        rule[16..32].copy_from_slice(&group_uuid[..]);

                        // todo: this should probably just be a list
                        // todo: of perms on the group, instead of each
                        // todo: group perm getting its own entry.
                        rule[32] = 1;

                        access.get::<[u8; 33], [u8]>(&core.group_db.clone(), &rule)?;

                        let entities = query_result
                            .entities
                            .entry(parent_uuid.try_into().expect("failed to convert"))
                            .or_insert(BTreeMap::new());

                        let entities = entities.entry(start_key[16]).or_insert(vec![]);

                        entities.push(Entity {
                            uuid: entity_key[17..33].try_into().expect("failed to convert"),
                            user: entity_value[17..33].try_into().expect("failed to convert"),
                            value: entity_value[33..].to_vec(),
                        });
                    }

                    (entity_key, entity_value) = entity_cursor.next::<[u8; 33], [u8]>(&access)?;
                }
            }
        }
    }

    let encoded: Vec<u8> = bitcode::encode(&query_result);

    Ok(encoded.into())
}
