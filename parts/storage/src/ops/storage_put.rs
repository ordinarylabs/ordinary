use bytes::{BufMut, Bytes, BytesMut};
use cbwaw::token;
use uuid::Uuid;

/// reversed <-
/// put[token[action.hmac.exp.user_uuid.group_uuid]h.kind.parent.content]
/// - token
///     - action: 1 byte
///     - hmac: 32 bytes
///     - exp: 8 bytes
///     - user_uuid: 16 bytes
///     - group_uuid: 16 bytes
/// - h
///     - kind len: 1 byte (can't be more than 255 chars)
/// - content: 0
pub fn new(
    token: &[u8; 73],
    kind: &[u8],
    parent: &[u8; 16],
    content: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let kind_len = kind.len();

    if kind_len > 255 {
        return Err("kind cannot be larger than 255 bytes".into());
    }

    let mut buf = BytesMut::with_capacity(content.len() + kind_len + 1 + 16);

    buf.put(&token[..]);

    buf.put(&parent[..]);

    buf.put_u8(kind_len as u8);
    buf.put(kind);

    buf.put(content);

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
/// (id, parent, group, key, content)
pub fn process(
    bytes: Bytes,
) -> Result<([u8; 16], Bytes, Bytes, Bytes, Bytes), Box<dyn std::error::Error>> {
    let len = bytes.len();

    if len < 73 {
        return Err("does not include token".into());
    }

    let (user, group) = token::validate_access(&bytes[0..73])?;

    let mut key = BytesMut::with_capacity(255 + 16 + 16);

    let parent = Bytes::copy_from_slice(&bytes[73..89]);

    let kind_len = bytes[89] as usize;
    let kind = &bytes[90..90 + kind_len];

    key.put(kind);
    key.put(&parent[..]);

    let uuid = Uuid::now_v7();
    let id = *uuid.as_bytes();

    key.put(&id[..]);

    let content_len = len - (17 + kind_len);

    let mut content = BytesMut::with_capacity(16 + content_len);

    content.put(&user[..]);
    content.put(&bytes[90 + kind_len..]);

    Ok((id, parent, group, key.into(), content.into()))
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

        let put_req = new(token[..].try_into()?, b"food", &parent, b"cheesecake")?;

        let (id, p_parent, p_group, key, content) = process(put_req)?;

        assert_eq!(&p_parent, &parent[..]);
        assert_eq!(&p_group, &group[..]);

        assert_eq!(&key[0..4], b"food");
        assert_eq!(&key[4..20], &parent);
        assert_eq!(Uuid::from_slice(&key[20..36])?, Uuid::from_bytes(id));

        assert_eq!(&content[0..16], &user);
        assert_eq!(&content[16..], b"cheesecake");

        Ok(())
    }
}
