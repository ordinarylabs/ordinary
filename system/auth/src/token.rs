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
pub fn gen_with_group(
    action: u8,
    user_uuid: &[u8; 16],
    group_uuid: &[u8; 16],
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
                .chain_update(&user_uuid)
                .chain_update(&group_uuid)
                .finalize_fixed();

            let mut buf = BytesMut::with_capacity(73);

            buf.put_u8(action);

            buf.put(&exp_as_bytes[..]);
            buf.put(&hmac[..]);
            buf.put(&user_uuid[..]);
            buf.put(&group_uuid[..]);

            Ok(buf.into())
        }
        None => Err("date is out of range".into()),
    }
}

/// (user, group)
#[inline(always)]
pub fn verify_with_group(
    action: u8,
    token: &[u8],
) -> Result<([u8; 16], [u8; 16]), Box<dyn std::error::Error>> {
    let exp_as_bytes: [u8; 8] = token[1..9].try_into()?;
    let exp = u64::from_be_bytes(exp_as_bytes);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    if exp < now {
        return Err("token is expired".into());
    }

    let user_uuid: [u8; 16] = token[41..57].try_into()?;
    let group_uuid: [u8; 16] = token[57..73].try_into()?;

    let comp = blake2::Blake2sMac256::new_from_slice(&HMAC_KEY)?
        .chain_update(&[action])
        .chain_update(&exp_as_bytes)
        .chain_update(&user_uuid)
        .chain_update(&group_uuid)
        .finalize_fixed();

    let hmac = GenericArray::<u8, typenum::U32>::from_slice(&token[9..41]);

    if &comp == hmac {
        Ok((user_uuid, group_uuid))
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
pub fn gen_without_group(
    action: u8,
    user_uuid: &[u8; 16],
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
                .chain_update(&user_uuid)
                .finalize_fixed();

            let mut buf = BytesMut::with_capacity(73);

            buf.put_u8(action);

            buf.put(&exp_as_bytes[..]);
            buf.put(&hmac[..]);
            buf.put(&user_uuid[..]);

            Ok(buf.into())
        }
        None => Err("date is out of range".into()),
    }
}

/// (user, group)
#[inline(always)]
pub fn verify_without_group(
    action: u8,
    token: &[u8],
) -> Result<[u8; 16], Box<dyn std::error::Error>> {
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
        .chain_update(&[action])
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
    fn with_group() -> Result<(), Box<dyn std::error::Error>> {
        let action: u8 = 0;
        let user_uuid = *Uuid::now_v7().as_bytes();
        let group = *Uuid::now_v7().as_bytes();

        let token = gen_with_group(action, &user_uuid, &group)?;

        let (v_user_uuid, v_group) = verify_with_group(action, &token)?;

        assert_eq!(&user_uuid[..], &v_user_uuid[..]);
        assert_eq!(&group[..], &v_group[..]);

        Ok(())
    }

    #[test]
    fn without_group() -> Result<(), Box<dyn std::error::Error>> {
        let action: u8 = 0;
        let user_uuid = *Uuid::now_v7().as_bytes();

        let token = gen_without_group(action, &user_uuid)?;

        let v_user_uuid = verify_without_group(action, &token)?;

        assert_eq!(&user_uuid[..], &v_user_uuid[..]);

        Ok(())
    }
}
