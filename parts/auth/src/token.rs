use std::time::{Duration, SystemTime};

use blake2::digest::{
    generic_array::{typenum, GenericArray},
    FixedOutput, Mac,
};
use bytes::{BufMut, Bytes, BytesMut};

// half needs to be pulled from one of the other 2+ threads,
// processes, or servers (depending on the level)
const HMAC_KEY: &'static [u8] = b"TODO: replace this asap";

/// 73 bytes
///
/// action: u8,
/// exp: [u8; 8],
/// hmac: [u8; 32],
/// user_id: [u8; 16],
/// group_id: [u8; 16]
#[inline(always)]
pub fn generate_access(
    action: u8,
    user: &[u8; 16],
    group: &[u8; 16],
) -> Result<Bytes, Box<dyn std::error::Error>> {
    match SystemTime::now().checked_add(Duration::from_secs(60 * 60 * 24)) {
        Some(tmrw) => {
            let exp_as_bytes = tmrw
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs()
                .to_be_bytes();

            let hmac = blake2::Blake2sMac256::new_from_slice(&HMAC_KEY)?
                .chain_update(&[action])
                .chain_update(&exp_as_bytes)
                .chain_update(&user)
                .chain_update(&group)
                .finalize_fixed();

            let mut buf = BytesMut::with_capacity(73);

            buf.put_u8(action);

            buf.put(&exp_as_bytes[..]);
            buf.put(&hmac[..]);
            buf.put(&user[..]);
            buf.put(&group[..]);

            Ok(buf.into())
        }
        None => Err("date is out of range".into()),
    }
}

/// (user, group)
#[inline(always)]
pub fn verify_access(token: &[u8]) -> Result<([u8; 16], [u8; 16]), Box<dyn std::error::Error>> {
    let exp_as_bytes: [u8; 8] = token[1..9].try_into()?;
    let exp = u64::from_be_bytes(exp_as_bytes);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    if exp < now {
        return Err("token is expired".into());
    }

    let user: [u8; 16] = token[41..57].try_into()?;
    let group: [u8; 16] = token[57..73].try_into()?;

    let comp = blake2::Blake2sMac256::new_from_slice(&HMAC_KEY)?
        .chain_update(&[token[0]])
        .chain_update(&exp_as_bytes)
        .chain_update(&user)
        .chain_update(&group)
        .finalize_fixed();

    let hmac = GenericArray::<u8, typenum::U32>::from_slice(&token[9..41]);

    if &comp == hmac {
        Ok((user, group))
    } else {
        return Err("invalid token".into());
    }
}

/// 73 bytes
///
/// action: u8,
/// exp: [u8; 8],
/// hmac: [u8; 32],
/// user_id: [u8; 16]
#[inline(always)]
pub fn generate_refresh(user: &[u8; 16]) -> Result<Bytes, Box<dyn std::error::Error>> {
    match SystemTime::now().checked_add(Duration::from_secs(60 * 60 * 24)) {
        Some(tmrw) => {
            let exp_as_bytes = tmrw
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs()
                .to_be_bytes();

            let hmac = blake2::Blake2sMac256::new_from_slice(&HMAC_KEY)?
                .chain_update(&[0])
                .chain_update(&exp_as_bytes)
                .chain_update(&user)
                .finalize_fixed();

            let mut buf = BytesMut::with_capacity(73);

            buf.put_u8(0);

            buf.put(&exp_as_bytes[..]);
            buf.put(&hmac[..]);
            buf.put(&user[..]);

            Ok(buf.into())
        }
        None => Err("date is out of range".into()),
    }
}

/// (user, group)
#[inline(always)]
pub fn verify_refresh(token: &[u8]) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    let exp_as_bytes: [u8; 8] = token[1..9].try_into()?;
    let exp = u64::from_be_bytes(exp_as_bytes);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    if exp < now {
        return Err("token is expired".into());
    }

    let user: [u8; 16] = token[41..57].try_into()?;

    let comp = blake2::Blake2sMac256::new_from_slice(&HMAC_KEY)?
        .chain_update(&[token[0]])
        .chain_update(&exp_as_bytes)
        .chain_update(&user)
        .finalize_fixed();

    let hmac = GenericArray::<u8, typenum::U32>::from_slice(&token[9..41]);

    if &comp == hmac {
        Ok(user)
    } else {
        return Err("invalid token".into());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn access() -> Result<(), Box<dyn std::error::Error>> {
        let action: u8 = 0;
        let user = *Uuid::now_v7().as_bytes();
        let group = *Uuid::now_v7().as_bytes();

        let token = generate_access(action, &user, &group)?;

        let (v_user, v_group) = verify_access(&token)?;

        assert_eq!(&user[..], &v_user[..]);
        assert_eq!(&group[..], &v_group[..]);

        Ok(())
    }
}
