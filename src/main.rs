mod db;
mod auth;
use db::*;
use serde_json;
use auth::jwt:: {Claims, create_jwt};
use auth::middleware::JwtMiddleware;
use poem::{
    get, post, handler, listener::TcpListener, Route, Server,
    web::{Json, Path},
    EndpointExt,
    http::StatusCode,
    IntoResponse
};
use mongodb::{bson::doc, Client, Collection};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// This struct defines the shape of JSON data we receive or send in API requests/responses.
// The `name` field is expected in JSON payloads like: { "name": "Alice" }
// `Serialize` and `Deserialize` allow converting this struct to/from JSON via Serde.
// `Debug` is useful for printing the struct for debugging. See add_person comments for usage
#[derive(Debug, Serialize, Deserialize)]
struct User {
    username: String,
    password: String,
    role: String
}


// Handles POST requests to /add_person. The #[handler] prefix is for poem to recognize it
// This function receives JSON data like { "name": "Alice" } and deserializes it
// into a Person, and inserts it into the MongoDB collection.
//
// If the insert is successful, it returns HTTP 201 Created.
// If the insert fails, it returns HTTP 500 Internal Server Error.
#[handler]
async fn add_user(
    Json(payload): Json<User>,
    db: poem::web::Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, poem::error::Error> {
    let collection = db.as_ref();
    insert_user(collection, payload).await?;
    // If there’s an error, I don’t care what it is — just turn it into this fixed response.
    // In this case we ignore it. |_| is just shorthand rust way of saying
    // whatever the error is just throw this INTERNAL_SERVER_ERROR.
    // If an error occur we could decide to do something with that error.
    // If we want to do something with the error it can look like this
    //.map_err(|err| {
    //     eprintln!("Insert error: {:?}", err); // here the Debug in the struct is used {:?}
    //     StatusCode::INTERNAL_SERVER_ERROR
    // })?;
    // the ? forces a return in case of an error and skips the Ok(statuscode) on the next line.
    Ok(StatusCode::CREATED)
}


// Handles GET requests to fetch a Person by name from the database.
//
// # Arguments
// - `Path(name)`: Extracts the `:name` segment from the request path.
// - `db`: Shared MongoDB collection wrapped in Poem's `Data`.
//
// # Returns
// - `200 OK` with the Person document as JSON if found.
// - `404 Not Found` if no document matches the name.
// - `500 Internal Server Error` if a DB error occurs.
#[handler]
async fn get_user(
    Path(name): Path<String>,
    db: poem::web::Data<&Arc<Collection<User>>>,
) -> Result<Json<User>, StatusCode> {
    // Get a reference to the MongoDB collection.
    let collection = db.as_ref();

    // Attempt to find a Person document matching the provided name.
    match find_user(collection, &name).await {
        // If found, return it as JSON with 200 OK.
        Ok(Some(doc)) => Ok(Json(doc)),
        // If not found, return a 404 Not Found status.
        Ok(None) => Err(StatusCode::NOT_FOUND),
        // If a database error occurs, return a 500 Internal Server Error.
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Handles PUT requests to update a Person for a specific name in the database.
//
// # Arguments
// - `Path(name)`: Extracts the `:name` segment from the URL path (the name to update).
// - `Json(payload)`: Parses the request body as JSON into a `Person`.
// - `db`: Shared MongoDB collection injected using Poem's `Data`.
//
// # Returns
// - `200 OK` with a success message if the update was successful.
// - `404 Not Found` if no document matched the name (i.e., nothing was updated).
// - `500 Internal Server Error` if a DB error occurs.
#[handler]
async fn user_update(
    Path(name): Path<String>,
    Json(payload): Json<User>,
    db: poem::web::Data<&Arc<Collection<User>>>,
) -> Result<String, StatusCode> {
    let collection = db.as_ref(); // Extract &Collection<Person>
    // Attempt to update the Person document with the new name.
    match update_user(collection, &name, &payload.username).await {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(format!("Updated user to '{}'", payload.username)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Handles DELETE requests to remove a Person by name from the database.
//
// # Arguments
// - `Path(name)`: Extracts the `:name` segment from the URL path (the name to delete).
// - `db`: Shared MongoDB collection injected using Poem's `Data`.
//
// # Returns
// - `200 OK` with a success message if the deletion was successful.
// - `404 Not Found` if no document matched the name (i.e., nothing was deleted).
// - `500 Internal Server Error` if a DB error occurs.
#[handler]
async fn user_delete(
    Path(username): Path<String>,
    db: poem::web::Data<&Arc<Collection<User>>>,
) -> Result<String, StatusCode> {
    let collection = db.as_ref(); // Extract &Collection<User>
    match delete_user(collection, &username).await {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(format!("Deleted user '{}'", username)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct LoginInfo {
    username: String,
    password: String,
}

#[handler]
async fn login(Json(payload): Json<LoginInfo>, db: poem::web::Data<&Arc<Collection<User>>>) -> poem::Result<impl IntoResponse> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(poem::Error::from_string("Either username or password is missing", StatusCode::UNAUTHORIZED));
    }

    match db::login(db.as_ref(), &payload.username, &payload.password).await {
        Ok(user) => {
            let permissions = vec![user.role.to_string()];
            let claims = Claims::new(user.username, permissions);
            let jwt = create_jwt(claims)
                .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

            Ok(Json(serde_json::json!({ "token": jwt })))
        }
        Err(err) => Err(err),
    }
}

// The main entry point for the application, setting up the server and MongoDB connection.
//
// # Steps
// 1. Connects to the MongoDB server at `localhost:27017`.
// 2. Selects (or creates) the database `my_api` and collection `Persons`.
// 3. Sets up the API routes using Poem.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Connect to MongoDB at localhost (with the default MongoDB port 27017).
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();

    // Select the database `my_api`, creating it if it doesn't exist.
    let db = client.database("my_api");

    // Select the collection `Persons` in the `my_api` database, creating it if it doesn't exist.
    let collection = db.collection::<User>("users");

    // Wrap the collection in an Arc to safely share it across multiple threads.
    let collection = Arc::new(collection);

    // Configure the Poem app with routes for handling various HTTP methods.
    let app = Route::new()
        .at("/user/add", post(add_user))
        .at(
            "/user/:name",
            get(get_user)
                .put(user_update)
                .delete(user_delete),
        )
        .at("/login", post(login))
        .with(JwtMiddleware)
        .data(collection);
    // the route /person/:name is shared and will cause a duplication error.
    // .at("/person/:name", get(get_person))
    // .at("/person/:name", put(person_update))
    // .at("/person/:name", delete(person_delete))
    // to avoid this error we have to group these methods in a single .at()
    // then we can chain the .get .put .delete
    // test
    Server::new(TcpListener::bind("localhost:3000"))
        .run(app)
        .await
}