use uuid::Uuid;

use bytes::{BufMut, Bytes, BytesMut};

use crate::auth::token;

/// query[token[action.hmac.exp.user_uuid.group_uuid]h.kind.key[parent.uuid]list(h.kind_a[0..10].h.kind_b[0])]
/// - token
///     - action: 1 byte
///     - hmac: 32 bytes
///     - exp: 8 bytes
///     - user_uuid: 16 bytes
///     - group_uuid: 16 bytes
/// - h
///     - kind len: 1 byte (can't be more than 255 chars)
///     - count of sub-queries: 1 byte (max sub-queries is 255)
/// - kind: 1 - 255 bytes
/// - key
///     - parent: 16 bytes
///     - uuid: 16 bytes
/// - h (n)
///     - kind len: 1 byte
///     - kind: 1 byte (0 -> index, 1 -> range, 2 -> count)
///         - 0
///             - index: 1 byte (the max number of child messages under a given kind is capped at 255)
///         - 1
///             - starting position: 1 byte
///             - ending position: 1 byte
///         - 2
/// - kind: 1 - 255 bytes
///
/// !! needs to be compiled into the binary formatted query at build time
/// !! and then the query request can just be a reference to the built query
///
///  <div data-query="product">
///     Product ID: <span data-id>...</span>
///
///     <p data-content>Placeholder...</p>
///     <div data-user>
///         User ID: <span data-id>...</span>
///         Username: <span data-username>...</span>
///     </div>
///
///     <section data-subquery="reviews">
///         <div data-subquery="stars">
///             <div data-occurrences>
///                 <div data-value-ordered>{}</div>
///                 <progress id="stars-{value}" value="{count}" max="{count-max}"></progress>
///             </div>
///         </div>
///
///         <div data-range="-10,-1">
///
///         </div>
///     </section>
/// </div>
///
/// product.1234.comments[0..10].replies
pub fn new(
    token: &[u8; 73],
    parent: &[u8; 16],
    id: &[u8; 16],
    query: &str,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    // let kind_len = kind.len();

    // if kind_len > 255 {
    //     return Err("kind cannot be larger than 255 bytes".into());
    // }

    // let mut buf = BytesMut::with_capacity(content.len() + kind_len + 1 + 16);

    // buf.put(&token[..]);
    // buf.put(&parent[..]);
    // buf.put(&id[..]);

    // buf.put_u8(kind_len as u8);
    // buf.put(kind);

    // buf.put(content);

    // Ok(buf.into())
    Ok(Bytes::new())
}

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
    use uuid::Uuid;

    use super::{new, process};
    use crate::auth::token;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        let action: u8 = 8;
        let user = *Uuid::now_v7().as_bytes();
        let group = *Uuid::now_v7().as_bytes();

        let token = token::generate_access(action, &user, &group)?;

        let parent = *Uuid::now_v7().as_bytes();

        // let put_req = new(token[..].try_into()?, b"food", &parent, b"cheesecake")?;

        // let (id, p_parent, p_group, key, content) = process(put_req)?;

        // assert_eq!(&p_parent, &parent[..]);
        // assert_eq!(&p_group, &group[..]);

        // assert_eq!(&key[0..4], b"food");
        // assert_eq!(&key[4..20], &parent);
        // assert_eq!(Uuid::from_slice(&key[20..36])?, Uuid::from_bytes(id));

        // assert_eq!(&content[0..16], &user);
        // assert_eq!(&content[16..], b"cheesecake");

        Ok(())
    }
}
