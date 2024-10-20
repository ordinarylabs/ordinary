use std::sync::Arc;

use saferlmdb::{
    self as lmdb, put, Database, DatabaseOptions, Environment, ReadTransaction, Stat,
    WriteTransaction,
};

use bytes::Bytes;
use log::info;
use uuid::Uuid;

/// On-disk/transport format
///
/// ([ kind ]|[ grandparent ][   parent   ])
///                ([ kind ]|[   parent   ][  child   ])
///
/// get[token[user_uuid.exp.hmac]h.kind.key[parent.uuid]]
///     
///
/// on-disk[key[kind.parent.uuid] -> value[user_uuid.h.content.block.allow]]
///
/// get-response[h.content.user_uuid]
/// query-response[h.list(kind.list(parent.list(obj[uuid.user_uuid.content])))]
///
/// TODO
///
/// TRANSIT OPTIONS
/// compression: zstd | gzip | deflate | none
/// authentication: mTLS + MFA | password + access/refresh tokens + TLS + MFA
///
/// STORAGE OPTIONS
/// compression: zstd | gzip | deflate | none
/// encryption: e2ee | server | none

pub mod op {
    pub mod put {
        use uuid::Uuid;

        use bytes::{BufMut, Bytes, BytesMut};

        use crate::auth::token;

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
    }

    pub mod query {
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
    }
}

pub struct StorageSystem {
    env: Arc<Environment>,

    /// object -> group | user -> group | group -> group
    /// is DUPSORT, and DUPFIXED
    group_db: Arc<Database<'static>>,

    /// [u8; ?] -> [u8; ?]
    kv_store: Arc<Database<'static>>,

    /// [kind.parent.object] -> [user.content]
    graph_db: Arc<Database<'static>>,
}

impl StorageSystem {
    pub fn new(env: Arc<Environment>) -> Result<Self, Box<dyn std::error::Error>> {
        let group_db = Arc::new(Database::open(
            env.clone(),
            Some("group"),
            &DatabaseOptions::new(
                lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
            ),
        )?);

        let kv_store = Arc::new(Database::open(
            env.clone(),
            Some("kv_store"),
            &DatabaseOptions::new(
                lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
            ),
        )?);

        let graph_db = Arc::new(Database::open(
            env.clone(),
            Some("graph"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        Ok(Self {
            env,
            group_db,
            kv_store,
            graph_db,
        })
    }

    /// Cloned LMDB Stat
    pub fn stat(&self) -> Result<Stat, Box<dyn std::error::Error>> {
        let stat = self.env.stat()?;
        Ok(stat.clone())
    }

    /// K/V insert
    pub fn insert(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    /// K/V get
    pub fn get(&self, key: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let txn = ReadTransaction::new(self.env.clone())?;
        let access = txn.access();

        let bytes: &[u8] = access.get(&self.kv_store, &key[..])?;

        Ok(Bytes::copy_from_slice(bytes))
    }

    /// Graph put
    pub fn put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();

        rayon::spawn(|| match op::put::process(payload) {
            Ok(put) => {
                if let Err(err) = tx.send(put) {
                    log::error!("{:?}", err);
                }
            }
            Err(err) => {
                log::error!("{:?}", err);
            }
        });

        let (id, parent, group, key, content) = rx.recv()?;

        let txn = WriteTransaction::new(self.env.clone())?;

        {
            let mut access = txn.access();

            let mut valid_group = false;
            let parent_group: &[u8] = access.get(&self.group_db, &parent[..])?;

            if parent_group != group {
                let mut cursor = txn.cursor(self.group_db.clone())?;

                cursor.seek_k::<[u8], [u8; 16]>(&access, &parent_group)?;

                let sub_groups: &[[u8; 16]] = cursor.get_multiple(&access)?;

                for sub_group in sub_groups {
                    if sub_group == &group[..] {
                        valid_group = true;
                        break;
                    }
                }

                if !valid_group {
                    'outer: loop {
                        let sub_groups: &[[u8; 16]] = cursor.next_multiple(&access)?;

                        for sub_group in sub_groups {
                            if sub_group == &group[..] {
                                valid_group = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }

            if valid_group {
                access.put(&self.group_db, &id, &*group, put::Flags::empty())?;
                access.put(&self.graph_db, &*key, &*content, put::Flags::empty())?;
            } else {
                return Err("invalid group".into());
            }
        }

        txn.commit()?;

        Ok(Bytes::copy_from_slice(&key[key.len() - 16..key.len()]))
    }

    /// Graph query
    pub fn query(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let txn = ReadTransaction::new(self.env.clone())?;
        let access = txn.access();

        let mut cursor = txn
            .cursor(self.graph_db.clone())
            .expect("failed to create cursor");

        let object: (&[u8; 16], &[u8]) = cursor.last(&access).expect("failed to get first");

        let uuid = Uuid::from_slice(object.0)?;
        let string = std::str::from_utf8(object.1);

        info!("{:?}", uuid);
        info!("{:?}", string);

        Ok(Bytes::copy_from_slice(object.1))
    }

    /// File read
    pub fn read(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    /// File write
    pub fn write(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    /// Object upload
    pub fn upload(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    /// Object download
    pub fn download(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
}
