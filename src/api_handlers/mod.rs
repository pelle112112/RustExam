pub mod file_handlers;
pub mod user_handlers;
use poem::{Request, http::StatusCode, Result};
use crate::auth::AuthUser;

fn extract_user(req: &Request) -> Result<AuthUser> {
    req.extensions()
        .get::<AuthUser>()
        .cloned()
        .ok_or(StatusCode::UNAUTHORIZED.into())
}