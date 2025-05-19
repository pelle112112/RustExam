pub mod jwt;
pub mod middleware;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
}