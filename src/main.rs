use poem::{get, post, handler, listener::TcpListener, Route, Server, web::{Json, Path}, EndpointExt, IntoResponse};
use serde::Deserialize;
use serde_json;
mod auth;
use auth::jwt:: {Claims, create_jwt};
use auth::middleware::JwtMiddleware;

#[derive(Deserialize)]
struct GreetRequest {
    name: String,
}

#[derive(Deserialize)]
struct LoginInfo {
    username: String,
    password: String,
}

#[poem_grants::protect("user")]
#[handler]
async fn greet(Json(payload): Json<GreetRequest>) -> String {
    format!("Hello, {}!", payload.name)
}

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}


#[handler]
async fn login(Json(payload): Json<LoginInfo>) -> poem::Result<impl IntoResponse> {
    if &payload.username !="" && &payload.password !="" {
        let mut permissions: Vec<String> = Vec::new();
        permissions.push("user".to_string());
        let claims = Claims::new(payload.username, permissions);
        let jwt = create_jwt(claims)?;
        
        Ok(Json(serde_json::json!({"token": jwt})))
    } else {
        Err(poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/greet", post(greet))
        .at("/login", post(login))
        .with(JwtMiddleware);
    Server::new(TcpListener::bind("localhost:3000"))
        .run(app)
        .await
}