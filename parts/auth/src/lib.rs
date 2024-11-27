use argon2::Argon2;
use opaque_ke::ciphersuite::CipherSuite;

// pub use opaque_ke::ServerSetup;

pub mod login;
pub mod registration;
pub mod token;

pub struct DefaultCipherSuite;

impl CipherSuite for DefaultCipherSuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::key_exchange::tripledh::TripleDh;

    type Ksf = Argon2<'static>;
}
