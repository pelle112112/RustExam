mod database;
mod auth;
mod api_handlers;
mod models;

use database::user_db::*;
use database::file_db::*;
use api_handlers::user_handlers::*;
use api_handlers::file_handlers::*;
use auth::middleware::JwtMiddleware;
use poem::{
    get, post, listener::TcpListener, Route, Server,
    EndpointExt,
    Result,
};
use mongodb::{bson::{Document}, Client};
use std::sync::Arc;

// The main entry point for the application, setting up the server and MongoDB connection.
//
// # Steps
// 1. Connects to the MongoDB server at `localhost:27017`.
// 2. Selects (or creates) the database `my_api` and collection `users` - adds test users if they do not already exist, and ensures uniqueness of usernames.
// 3. Sets up the API routes using Poem.


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let db = client.database("my_api");

    let collection = Arc::new(db.collection::<User>("users"));
    let image_collection = Arc::new(db.collection::<ImageDocument>("images"));
    let files_collection = Arc::new(db.collection::<DocumentEntry>("files"));

    let _ = initial_user_db_setup(&collection).await;
    // Configure the Poem app with routes for handling various HTTP methods.
    let app = Route::new()
        .at("/user/add", post(add_user))
        .at(
            "/user/:name",
            get(get_user)
                .put(user_update)
                .delete(user_delete),
        )
        .at("/login", post(api_handlers::user_handlers::login))
        .at("/upload", post(upload_file))
        .at("/download_file/:filename", get(download_file))
        .at("/files", get(get_files))
        .at("/upload_image", post(upload_image))
        .at("/download_image/:imagename", get(download_image) )
        .with(JwtMiddleware)
        .data(image_collection)
        .data(collection)
        .data(files_collection);

    Server::new(TcpListener::bind("localhost:3000"))
        .run(app)
        .await
}