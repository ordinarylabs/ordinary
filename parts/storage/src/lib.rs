use std::{collections::BTreeMap, sync::Arc};

use saferlmdb::{
    self as lmdb, put, Database, DatabaseOptions, EnvBuilder, Environment, ReadTransaction, Stat,
    WriteTransaction,
};

use bytes::Bytes;
use log::info;
use uuid::Uuid;

// !! one database per key size.
// ?? we need to switch all model names to numbers
// ?? and for the first 255 models we can use 1 byte.
// ** when we get to 256 models we will need to start
// ** at [0, 0] inside the u16 size db.
// !! we are intentionally using only one db per node, with
// !! this scheme.

pub mod ops;

/// On-disk/transport format
///
/// key([ kind ]|[  grandparent  ],[ kind ]|[  parent  ])-value(properties)-backLink([ kind ]|[  great grandparent  ])
///                            key([ kind ]|[  parent  ],[ kind ]|[  child  ])-value(properties)-backLink([ kind ]|[  grandparent  ])
///                                                 
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
#[derive(Clone)]
pub struct StorageSystem {
    env: Arc<Environment>,
    /// object -> group | user -> group | group -> group
    /// is DUPSORT, and DUPFIXED
    group_db: Arc<Database<'static>>,
    /// [kind.parent.object] -> [user.content]
    graph_db_u8: Arc<Database<'static>>,
}
//     /// map(category -> db(parent_id.self_id -> owner_id.value))
//     /// is DUPSORT, and DUPFIXED
//     /// map("group" -> db(self_id -> group) | db(user_id -> group) | db(group -> group)
//     storage: &Database<'static>,
// }

impl StorageSystem {
    pub fn new(env: Arc<Environment>) -> Result<Self, Box<dyn std::error::Error>> {
        let group_db = Arc::new(Database::open(
            env.clone(),
            Some("0"),
            &DatabaseOptions::new(
                lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
            ),
        )?);

        let graph_db_u8 = Arc::new(Database::open(
            env.clone(),
            Some("1"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        // TODO:
        // let graph_db_u16 = Arc::new(Database::open(
        //     env.clone(),
        //     Some("2"),
        //     &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        // )?);

        Ok(Self {
            env,
            group_db,
            graph_db_u8,
        })
    }

    /// Graph put
    pub fn put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (id, parent, group, key, content) = ops::storage_put::process(payload)?;

        let txn = WriteTransaction::new(self.env.clone())?;

        {
            let mut access = txn.access();

            let mut valid_group = false;
            let parent_group: &[u8] = access.get(&self.group_db, &parent[..])?;

            if parent_group != group {
                let mut cursor = txn.cursor(self.group_db.clone())?;

                cursor.seek_k::<[u8], [u8; 16]>(&access, &parent_group)?;

                let sub_group: &[[u8; 16]] = cursor.get_multiple(&access)?;

                for sub_group in sub_group {
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
                access.put(&self.graph_db_u8, &*key, &*content, put::Flags::empty())?;
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
            .cursor(self.graph_db_u8.clone())
            .expect("failed to create cursor");

        let object: (&[u8; 16], &[u8]) = cursor.last(&access).expect("failed to get first");

        let uuid = Uuid::from_slice(object.0)?;
        let string = std::str::from_utf8(object.1);

        info!("{:?}", uuid);
        info!("{:?}", string);

        Ok(Bytes::copy_from_slice(object.1))
    }
}
