use stewball::ops;
use stewball::Core;

#[test]
fn all() -> Result<(), Box<dyn std::error::Error>> {
    let core = Core::new()?;

    // registration start
    let (state, req) = ops::registration_start::req(b"username", b"password")?;
    let res = core.registration_start(req)?;

    // registration finish
    let req = ops::registration_finish::req(b"username", b"password", &state, &res)?;
    core.registration_finish(req)?;

    // login start
    let (state, req) = ops::login_start::req(b"username", b"password")?;
    let res = core.login_start(req)?;

    // login finish
    let (req, session_key) = ops::login_finish::req(b"username", b"password", &state, &res)?;
    core.login_finish(req)?;

    Ok(())
}
