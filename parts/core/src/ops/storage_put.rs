use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
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
pub fn new(
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
///
/// (id, parent_id, group_id, kind, key, entity, grandparent_id, parent_kind, parent_key)
pub fn process(
    bytes: Bytes,
) -> Result<
    (
        [u8; 16],
        [u8; 16],
        [u8; 16],
        [u8; 16],
        [u8; 16],
        u8,
        u8,
        [u8; 33],
        [u8; 33],
        Bytes,
    ),
    Box<dyn std::error::Error>,
> {
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

    Ok((
        grandparent_id,
        parent_id,
        id,
        user_id,
        group_id,
        kind,
        parent_kind,
        key,
        parent_key,
        entity.into(),
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        let action: u8 = 8;
        let user = *Uuid::now_v7().as_bytes();
        let group = *Uuid::now_v7().as_bytes();

        let token = token::generate_access(action, &user, &group)?;

        let parent = *Uuid::now_v7().as_bytes();
        let grandparent = *Uuid::now_v7().as_bytes();

        let put_req = new(
            token[..].try_into()?,
            &parent,
            1,
            &grandparent,
            2,
            b"cheesecake",
        )?;

        let (
            p_grandparent,
            p_parent,
            id,
            p_user,
            p_group,
            kind,
            p_parent_kind,
            key,
            _parent_key, // TODO: test
            entity,
        ) = process(put_req)?;

        assert_eq!(&p_parent, &parent[..]);
        assert_eq!(&p_group, &group[..]);

        assert_eq!(key[16], kind);
        assert_eq!(&key[0..16], &parent);
        assert_eq!(Uuid::from_slice(&key[17..33])?, Uuid::from_bytes(id));

        assert_eq!(&entity[0..16], &p_grandparent);
        assert_eq!(&entity[16], &p_parent_kind);
        assert_eq!(&entity[17..33], &p_user);

        assert_eq!(&entity[33..], b"cheesecake");

        Ok(())
    }
}
