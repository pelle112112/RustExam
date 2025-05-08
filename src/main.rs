use poem::{
    get, post, handler, listener::TcpListener, Route, Server,
    web::{Json, Path},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct GreetRequest {
    name: String,
}

#[handler]
async fn greet(Json(payload): Json<GreetRequest>) -> String {
    format!("Hello, {}!", payload.name)
}

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}



#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/greet", post(greet));
    Server::new(TcpListener::bind("localhost:3000"))
        .run(app)
        .await
}