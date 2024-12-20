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
    access_token: &[u8],

    parent_uuid: &[u8; 16],
    kind: u8,

    grandparent_uuid: &[u8; 16],
    parent_kind: u8,

    entity: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(73 + 1 + 16 + entity.len());

    buf.put(&access_token[..]);

    buf.put(&parent_uuid[..]);
    buf.put_u8(kind);

    buf.put(&grandparent_uuid[..]);
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
        return Err("req does not include access token".into());
    }

    let (user_uuid, group_uuid) = token::verify_with_group(12, &bytes[0..73])?;

    let parent_uuid: [u8; 16] = bytes[73..89].try_into()?;

    let kind = bytes[89];

    let grandparent_uuid: [u8; 16] = bytes[90..106].try_into()?;
    let parent_kind = bytes[106];

    let uuid = Uuid::now_v7();
    let id = *uuid.as_bytes();

    let mut key = [0u8; 33];

    key[0..16].copy_from_slice(&parent_uuid[..]);
    key[16] = kind;
    key[17..33].copy_from_slice(&id[..]);

    let entity_len = len - (107);

    let mut entity = BytesMut::with_capacity(16 + 16 + 1 + entity_len);

    entity.put(&grandparent_uuid[..]);
    entity.put_u8(parent_kind);
    entity.put(&user_uuid[..]);

    entity.put(&bytes[107..]);

    let mut parent_key = [0u8; 33];

    parent_key[0..16].copy_from_slice(&grandparent_uuid[..]);
    parent_key[16] = parent_kind;
    parent_key[17..33].copy_from_slice(&parent_uuid[..]);

    let txn = WriteTransaction::new(core.env.clone())?;

    {
        let mut access = txn.access();

        // check the parent for this group association
        let mut check = Vec::with_capacity(33);

        check.extend_from_slice(&parent_uuid[..]);
        check.extend_from_slice(&group_uuid[..]);
        check.push(0);

        access.get::<[u8], [u8]>(&core.group_db.clone(), &check)?;
        access.put(&core.entity_db, &key, &*entity, put::Flags::empty())?;
    }

    txn.commit()?;

    Ok(Bytes::copy_from_slice(&id[..]))
}
