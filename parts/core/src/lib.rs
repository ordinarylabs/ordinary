use std::collections::BTreeMap;
use std::sync::Arc;

use bytes::Bytes;
use opaque_ke::{Ristretto255, ServerSetup};
use ops::access_get;
use parking_lot::Mutex;
use rand::rngs::OsRng;

use cbwaw::DefaultCipherSuite;
use saferlmdb::{self as lmdb, Database, DatabaseOptions, EnvBuilder, Environment, Stat};

// ?? all objects have an expiration time that you can renew

// ** because the user is connected to 3 clients at once, they can
// ** detect suspicious behavior faster.

// !! each entity can only have up to 255 properties or relationships

// ?? narrow

pub mod ops;

const MAX_USERNAME_LEN: u8 = 255;

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

    /// user_uuid -> state
    auth_state: Arc<Mutex<BTreeMap<[u8; 16], Vec<u8>>>>,

    /// DB env
    env: Arc<Environment>,

    /// username -> user_uuid.password_file
    auth_db: Arc<Database<'static>>,

    /// user_uuid -> user()
    user_db: Arc<Database<'static>>,

    /// (entity_uuid | user_uuid).group_id.action -> []
    /// is DUPSORT, and DUPFIXED
    group_db: Arc<Database<'static>>,

    /// single record:
    /// key(parent.kind.uuid) -> value(user_uuid.grandparent_uuid.parent_kind.entity)
    ///
    /// parent relationship:
    /// key(grandparent.kind.parent) -> value(user_uuid.great_grandparent_uuid.grandparent_kind.entity)
    ///                            key([  parent  ],[ kind ]|[  child  ]) -> backLink(grandparent.parent_kind), value(properties)
    entity_db: Arc<Database<'static>>,

    /// (entity_uuid | user_uuid).ref_type -> (entity_uuid | user_uuid)
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

        let auth_state = Arc::new(Mutex::new(BTreeMap::new()));

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
            auth_state,
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
        ops::access_get::handle(self, payload)
    }

    pub fn group_assign(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
    pub fn group_create(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::group_create::handle(self, payload)
    }
    pub fn group_drop(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    pub fn login_finish(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::login_finish::handle(self, payload)
    }
    pub fn login_start(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::login_finish::handle(self, payload)
    }

    pub fn registration_finish(&self, payload: Bytes) -> Result<(), Box<dyn std::error::Error>> {
        ops::registration_finish::handle(self, payload)
    }
    pub fn registration_start(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::registration_start::handle(self, payload)
    }

    pub fn secret_get(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }
    pub fn secret_put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        Ok(Bytes::new())
    }

    pub fn storage_put(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::storage_put::handle(self, payload)
    }

    pub fn storage_query(&self, payload: Bytes) -> Result<Bytes, Box<dyn std::error::Error>> {
        ops::storage_query::handle(self, payload)
    }
}
