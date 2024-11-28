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
    let res = core.login_finish(req)?;

    let refresh_token = ops::login_finish::res(res, &session_key)?;

    // get GROUP_CREATE access token
    let req = ops::access_get::req(&refresh_token, 3, None)?;
    let access_token = core.access_get(req)?;

    // create a group
    let req = ops::group_create::req(&access_token)?;
    let res = core.group_create(req)?;
    let group_uuid = ops::group_create::res(res)?;

    // get STORAGE_PUT access token for new group
    let req = ops::access_get::req(&refresh_token, 12, Some(&group_uuid))?;
    let access_token = core.access_get(req)?;

    // create a property on your user with new group
    // let req = ops::storage_put::req(
    //     &access_token,
    //     parent_uuid, // user uuid
    //     9,
    //     grandparent_uuid, // user uuid
    //     parent_kind,      // 0
    //     entity,
    // )?;

    // get STORAGE_QUERY access token
    let req = ops::access_get::req(&refresh_token, 13, None)?;
    let access_token = core.access_get(req)?;

    // query your user
    // let req = ops::storage_query::req(&access_token, query)?;

    Ok(())
}
