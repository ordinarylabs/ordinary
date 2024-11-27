use crate::Core;
use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
use saferlmdb::{put, WriteTransaction};
use uuid::Uuid;

/// reversed <-
/// put[token[action.hmac.exp.user_uuid.group_uuid]parent.kind.grandparent.parent_kind.entity]
/// - token
///     - action: 1 byte
///     - hmac: 32 bytes
///     - exp: 8 bytes
///     - user_uuid: 16 bytes
///     - group_uuid: 16 bytes
/// - h
///     - parent: 16 bytes
///     - kind: 1 byte (max 255 entities)
///     - grand_parent: 16 bytes
///     - parent_kind: 1 byte
/// - entity:
///     - entity: ..
pub fn req(
    token: &[u8; 73],

    parent: &[u8; 16],
    kind: u8,

    grandparent: &[u8; 16],
    parent_kind: u8,

    entity: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(73 + 1 + 16 + entity.len());

    buf.put(&token[..]);

    buf.put(&parent[..]);
    buf.put_u8(kind);

    buf.put(&grandparent[..]);
    buf.put_u8(parent_kind);

    buf.put(entity);

    Ok(buf.into())
}

/// !! should also add build-time query-caching because
/// !! at least for the primary application it should be knowable
/// !! at build time.
///
/// ?? could also add other query caches.
///
/// ** breaking changes will happen, because humans are human and stupid
/// ** we need the ability to notify and propagate those changes for
/// ** downstream consumers. i think the best way to do that is with
/// ** the "subnetverse."
///
/// !! "an upstream provider has made a change to a data model you depend on; see the diff ..."
/// !! "see if you're impacted and resolve any discrepancies ..."
pub fn handle(core: &Core, bytes: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
    let len = bytes.len();

    if len < 73 {
        return Err("does not include token".into());
    }

    let (user_id, group_id) = token::verify_access(&bytes[0..73])?;

    let parent_id: [u8; 16] = bytes[73..89].try_into()?;
    let kind = bytes[89];

    let grandparent_id: [u8; 16] = bytes[90..106].try_into()?;
    let parent_kind = bytes[106];

    let uuid = Uuid::now_v7();
    let id = *uuid.as_bytes();

    let mut key = [0u8; 33];

    key[0..16].copy_from_slice(&parent_id[..]);
    key[16] = kind;
    key[17..33].copy_from_slice(&id[..]);

    let entity_len = len - (107);

    let mut entity = BytesMut::with_capacity(16 + 16 + 1 + entity_len);

    entity.put(&grandparent_id[..]);
    entity.put_u8(parent_kind);
    entity.put(&user_id[..]);

    entity.put(&bytes[107..]);

    let mut parent_key = [0u8; 33];

    parent_key[0..16].copy_from_slice(&grandparent_id[..]);
    parent_key[16] = parent_kind;
    parent_key[17..33].copy_from_slice(&parent_id[..]);

    let txn = WriteTransaction::new(core.env.clone())?;

    {
        let mut access = txn.access();

        let mut valid_group = false;
        let parent_group: &[u8; 16] = access.get(&core.group_db, &parent_id[..])?;

        if parent_group != &group_id {
            let mut cursor = txn.cursor(core.group_db.clone())?;

            cursor.seek_k::<[u8; 16], [u8; 16]>(&access, parent_group)?;

            let sub_groups: &[[u8; 16]] = cursor.get_multiple(&access)?;

            for sub_group in sub_groups {
                if sub_group == &group_id[..] {
                    valid_group = true;
                    break;
                }
            }

            if !valid_group {
                'outer: loop {
                    let sub_groups: &[[u8; 16]] = cursor.next_multiple(&access)?;

                    for sub_group in sub_groups {
                        if sub_group == &group_id[..] {
                            valid_group = true;
                            break 'outer;
                        }
                    }
                }
            }
        }

        if valid_group {
            access.put(&core.group_db, &id, &group_id, put::Flags::empty())?;
            access.put(&core.entity_db, &key, &*entity, put::Flags::empty())?;
        } else {
            return Err("invalid group".into());
        }
    }

    txn.commit()?;

    Ok(Bytes::copy_from_slice(&id[..]))
}
