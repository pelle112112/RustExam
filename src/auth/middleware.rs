use poem::http::header::AUTHORIZATION;
use poem::{
    Endpoint, Middleware, Request, Result
};
use poem_grants::authorities::AttachAuthorities;
use crate::auth::AuthUser;

pub struct JwtMiddleware;

impl<E: Endpoint> Middleware<E> for JwtMiddleware {
    type Output = JwtMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output{
        JwtMiddlewareImpl { ep }
    }
}

pub struct JwtMiddlewareImpl<E> {
    ep: E,
}

impl<E: Endpoint> Endpoint for JwtMiddlewareImpl<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        if let Some(value) = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.starts_with("Bearer "))
            .map(|value| &value[7..])
        {
            let claims = crate::auth::jwt::decode_jwt(value)?;

            req.attach(claims.permissions.clone());
            
            req.extensions_mut().insert(AuthUser {
                username: claims.username,
            });
        }
        self.ep.call(req).await
    }
}