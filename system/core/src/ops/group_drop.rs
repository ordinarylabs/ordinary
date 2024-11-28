use bytes::Bytes;

pub fn req(refresh_token: [u8; 73], action: u8) -> Result<Bytes, Box<dyn std::error::Error>> {
    // TODO: validate refresh token
    // TODO: check that user has
    // let token = token::generate_access(action, &user, &group)?;
    Ok(Bytes::new())
}

pub fn handle(bytes: Bytes) {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
