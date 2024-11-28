use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use bytes::Bytes;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, ServerLogin, ServerLoginStartParameters, ServerRegistration, ServerSetup,
};
use rand::rngs::OsRng;

use crate::{token, DefaultCipherSuite};

/// (state, message)
pub fn server_start(
    setup: &ServerSetup<DefaultCipherSuite>,
    username: &[u8],
    password_file: &[u8],
    client_start: &[u8],
) -> Result<(Vec<u8>, Bytes), Box<dyn std::error::Error>> {
    let password_file =
        ServerRegistration::<DefaultCipherSuite>::deserialize(&password_file).unwrap();
    let mut rng = OsRng;
    let login_start_result = ServerLogin::start(
        &mut rng,
        setup,
        Some(password_file),
        CredentialRequest::deserialize(client_start).unwrap(),
        username,
        ServerLoginStartParameters::default(),
    )
    .unwrap();

    Ok((
        login_start_result.state.serialize().to_vec(),
        Bytes::copy_from_slice(&login_start_result.message.serialize()[..]),
    ))
}

/// encrypted token
pub fn server_finish(
    user_id: &[u8; 16],
    client_finish: &[u8],
    server_start: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let start_state = ServerLogin::<DefaultCipherSuite>::deserialize(server_start).unwrap();

    let finish_result = start_state
        .finish(CredentialFinalization::deserialize(client_finish).unwrap())
        .unwrap();

    let token = token::gen_without_group(0, user_id).unwrap();

    let mut key = [0u8; 32];

    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.update(&finish_result.session_key);
    hasher.finalize_variable(&mut key).unwrap();

    let cipher = XChaCha20Poly1305::new(&key.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let mut ciphertext = cipher.encrypt(&nonce, token.as_ref()).unwrap();

    ciphertext.extend_from_slice(&nonce);

    Ok(Bytes::copy_from_slice(&ciphertext))
}

/// (state, message)
pub fn client_start(password: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let mut rng = OsRng;

    match ClientLogin::<DefaultCipherSuite>::start(&mut rng, password) {
        Ok(login) => Ok((
            login.state.serialize().to_vec(),
            login.message.serialize().to_vec(),
        )),
        Err(err) => return Err(err.to_string().into()),
    }
}

/// (message, session_key)
pub fn client_finish(
    password: &[u8],
    client_start: &[u8],
    server_start: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let client_login = match ClientLogin::<DefaultCipherSuite>::deserialize(&client_start) {
        Ok(l) => l,
        Err(err) => return Err(err.to_string().into()),
    };

    let server_start = match CredentialResponse::deserialize(&server_start) {
        Ok(s) => s,
        Err(err) => return Err(err.to_string().into()),
    };

    match client_login.finish(
        password,
        server_start,
        ClientLoginFinishParameters::default(),
    ) {
        Ok(finish) => Ok((
            finish.message.serialize().to_vec(),
            finish.session_key.to_vec(),
        )),
        Err(err) => return Err(err.to_string().into()),
    }
}

/// token
pub fn decrypt_token(
    encrypted_token: &[u8],
    session_key: &[u8],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let mut key = [0u8; 32];

    let mut hasher = Blake2bVar::new(32).unwrap();
    hasher.update(session_key);
    hasher.finalize_variable(&mut key).unwrap();

    let nonce_start = encrypted_token.len() - 25;

    let cipher = XChaCha20Poly1305::new(&key.into());
    let nonce = XNonce::from_slice(&encrypted_token[nonce_start..]);

    let plaintext = cipher
        .decrypt(nonce, encrypted_token[..nonce_start].as_ref())
        .unwrap();

    Ok(Bytes::copy_from_slice(&plaintext))
}
