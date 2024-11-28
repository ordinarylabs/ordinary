use stewball::ops;
use stewball::Core;

#[test]
fn all() -> Result<(), Box<dyn std::error::Error>> {
    let core = Core::new()?;

    // AUTH: registration start
    let (state, req) = ops::registration_start::req(b"username", b"password")?;
    let res = core.registration_start(req)?;

    // AUTH: registration finish
    let req = ops::registration_finish::req(b"username", b"password", &state, &res)?;
    core.registration_finish(req)?;

    // AUTH: login start
    let (state, req) = ops::login_start::req(b"username", b"password")?;
    let res = core.login_start(req)?;

    // AUTH: login finish
    let (req, session_key) = ops::login_finish::req(b"username", b"password", &state, &res)?;
    let res = core.login_finish(req)?;

    let refresh_token = ops::login_finish::res(res, &session_key);

    // AUTH: get group_create access token for 0000000000000000

    // create a group

    // AUTH: get group_assign access token for 0000000000000000

    // add group to your user

    // AUTH: get put access token for new group

    // create a property on your user with new group

    // AUTH: get read access token for 0000000000000000 ?? see how many of these show up so that we can make group optional

    // query your user

    Ok(())
}
