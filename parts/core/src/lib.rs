use std::collections::BTreeMap;
use std::sync::Arc;

use cbwaw::DefaultCipherSuite;
use opaque_ke::{rand::rngs::OsRng, Ristretto255, ServerSetup};
use saferlmdb::{
    self as lmdb, put, Database, DatabaseOptions, EnvBuilder, Environment, ReadTransaction, Stat,
    WriteTransaction,
};

use bytes::Bytes;
use log::info;
use uuid::Uuid;

// ?? all objects have an expiration time that you can renew

// ** because the user is connected to 3 clients at once, they can
// ** detect suspicious behavior faster.

// !! each entity can only have up to 255 properties or relationships

// ?? narrow

pub mod ops;

/// On-disk/transport format
///
/// key([  grandparent  ],[ kind ]|[  parent  ])-value(properties)-backLink([ kind ]|[  great grandparent  ], [ grandparent_kind ])
///                            key([  parent  ],[ kind ]|[  child  ])-value(properties)-backLink([ kind ]|[  grandparent  ],[ parent_kind ])
///                                                 
///
/// get[token[user_uuid.exp.hmac]h.kind.key[parent.uuid]]
///     
///
/// on-disk[key[parent.kind.uuid] -> value[user_uuid.grandparent_uuid.parent_kind.entity]]
///
/// get-response[h.entity.user_uuid]
/// query-response[h.list(kind.list(parent.list(obj[uuid.user_uuid.entity])))]
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
pub struct Core {
    opaque: ServerSetup<DefaultCipherSuite>,

    /// need a minimum of 5 dbs
    env: Arc<Environment>,

    /// username -> user_id.password_file
    auth_db: Arc<Database<'static>>,

    /// user_id -> user()
    user_db: Arc<Database<'static>>,

    /// (entity_id | user_id) -> group_id
    /// is DUPSORT, and DUPFIXED
    group_db: Arc<Database<'static>>,

    /// single record:
    /// key(parent.kind.uuid) -> value(user_uuid.grandparent_uuid.parent_kind.entity)
    ///
    /// parent relationship:
    /// key(grandparent.kind.parent) -> value(user_uuid.great_grandparent_uuid.grandparent_kind.entity)
    ///                            key([  parent  ],[ kind ]|[  child  ]) -> backLink(grandparent.parent_kind), value(properties)
    entity_db: Arc<Database<'static>>,

    /// (entity_id | user_id) -> ref_type.(entity_id | user_id)
    /// is DUPSORT, and DUPFIXED
    reference_db: Arc<Database<'static>>,

    /// uuid_v4 -> encrypted
    secrets_db: Arc<Database<'static>>,
}

impl Core {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut rng = OsRng;
        let opaque = ServerSetup::<DefaultCipherSuite>::new_with_key(
            &mut rng,
            opaque_ke::keypair::KeyPair::<Ristretto255>::from_private_key_slice(&[0u8; 32])
                .unwrap(),
        );

        std::fs::create_dir_all("./store")?;

        let env = Arc::new(unsafe {
            let mut env_builder = EnvBuilder::new().unwrap();
            env_builder.set_maxreaders(126).unwrap();
            env_builder.set_mapsize(10485760).unwrap();
            env_builder.set_maxdbs(6).unwrap();
            env_builder
                .open("./store", saferlmdb::open::Flags::empty(), 0o600)
                .unwrap()
        });

        let auth_db = Arc::new(Database::open(
            env.clone(),
            Some("0"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        let user_db = Arc::new(Database::open(
            env.clone(),
            Some("1"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        let group_db = Arc::new(Database::open(
            env.clone(),
            Some("2"),
            &DatabaseOptions::new(
                lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
            ),
        )?);

        let entity_db = Arc::new(Database::open(
            env.clone(),
            Some("3"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        let reference_db = Arc::new(Database::open(
            env.clone(),
            Some("4"),
            &DatabaseOptions::new(
                lmdb::db::Flags::DUPSORT | lmdb::db::Flags::DUPFIXED | lmdb::db::Flags::CREATE,
            ),
        )?);

        let secrets_db = Arc::new(Database::open(
            env.clone(),
            Some("5"),
            &DatabaseOptions::new(lmdb::db::Flags::CREATE),
        )?);

        Ok(Self {
            opaque,
            env,
            auth_db,
            user_db,
            group_db,
            entity_db,
            reference_db,
            secrets_db,
        })
    }

    pub fn stat(&self) -> Result<Stat, Box<dyn std::error::Error>> {
        let stat = self.env.stat()?;
        return Ok(stat);
    }

    pub fn access_get(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    pub fn group_assign(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
    pub fn group_create(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
    pub fn group_drop(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    pub fn login_finish(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (username, client_finish) = ops::login_finish::process(payload)?;

        // TODO: cbwaw::login::server_finish(user_uuid, &client_finish, server_start)

        Ok(Bytes::new())
    }
    pub fn login_start(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (username, client_start) = ops::login_start::process(payload)?;

        // TODO: get the password file

        // let (state, message) = cbwaw::login::server_start(
        //     &self.opaque,
        //     &username,
        //     &password_file,
        //     &client_start,
        // )?;

        // TODO: store state

        Ok(Bytes::new())
    }

    pub fn registration_finish(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (username, client_finish) = ops::registration_finish::process(payload)?;
        let password_file = cbwaw::registration::server_finish(&client_finish)?;

        // TODO: create user
        // TODO: write password file to auth_db

        Ok(Bytes::new())
    }
    pub fn registration_start(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (username, client_start) = ops::registration_start::process(payload)?;
        cbwaw::registration::server_start(&self.opaque, &username, &client_start)
    }

    pub fn secret_get(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
    pub fn secret_put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    pub fn storage_put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let (_grandparent, parent, id, _user, group, _kind, _parent_kind, key, _parent_key, entity) =
            ops::storage_put::process(payload)?;

        let txn = WriteTransaction::new(self.env.clone())?;

        {
            let mut access = txn.access();

            let mut valid_group = false;
            let parent_group: &[u8; 16] = access.get(&self.group_db, &parent[..])?;

            if parent_group != &group {
                let mut cursor = txn.cursor(self.group_db.clone())?;

                cursor.seek_k::<[u8; 16], [u8; 16]>(&access, parent_group)?;

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
                access.put(&self.group_db, &id, &group, put::Flags::empty())?;
                access.put(&self.entity_db, &key, &*entity, put::Flags::empty())?;
            } else {
                return Err("invalid group".into());
            }
        }

        txn.commit()?;

        Ok(Bytes::copy_from_slice(&id[..]))
    }

    pub fn storage_query(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        let txn = ReadTransaction::new(self.env.clone())?;
        let access = txn.access();

        let mut cursor = txn
            .cursor(self.entity_db.clone())
            .expect("failed to create cursor");

        let object: (&[u8; 16], &[u8]) = cursor.last(&access).expect("failed to get last");

        let uuid = Uuid::from_slice(object.0)?;
        let string = std::str::from_utf8(object.1);

        info!("{:?}", uuid);
        info!("{:?}", string);

        Ok(Bytes::copy_from_slice(object.1))
    }
}
