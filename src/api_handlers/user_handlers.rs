use std::sync::Arc;
use mongodb::Collection;
use poem::{handler, Error, IntoResponse};
use poem::http::StatusCode;
use poem::web::{Data, Json, Path};
use crate::auth::jwt::{create_jwt, Claims};
use crate::database;
use serde::{Deserialize};
use crate::database::user_db::*;

// Handles POST requests to /add_person. The #[handler] prefix is for poem to recognize it
// This function receives JSON data like { "name": "Alice" } and deserializes it
// into a Person, and inserts it into the MongoDB collection.
//
// If the insert is successful, it returns HTTP 201 Created.
// If the insert fails, it returns HTTP 500 Internal Server Error.
#[poem_grants::protect("admin")]
#[handler]
pub async fn add_user(
    Json(payload): Json<User>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, Error> {
    let collection = db.as_ref();
    insert_user(collection, &payload).await?;
    // the ? forces a return in case of an error and skips the Ok(status code) on the next line.
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
#[poem_grants::protect("admin")]
#[handler]
pub async fn get_user(
    Path(name): Path<String>,
    db: Data<&Arc<Collection<User>>>,
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
#[poem_grants::protect("admin")]
#[handler]
pub async fn user_update(
    Path(name): Path<String>,
    Json(payload): Json<User>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, Error> {
    let collection = db.as_ref();
    update_user(collection, &name, &payload).await?;
    Ok(StatusCode::OK)
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
#[poem_grants::protect("admin")]
#[handler]
pub async fn user_delete(
    Path(username): Path<String>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, Error> {
    let collection = db.as_ref();
    delete_user(collection, &username).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct LoginInfo {
    username: String,
    password: String,
}

#[handler]
pub async fn login(Json(payload): Json<LoginInfo>, db: Data<&Arc<Collection<User>>>) -> poem::Result<impl IntoResponse> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(Error::from_string("Either username or password is missing", StatusCode::UNAUTHORIZED));
    }

    match database::user_db::login(db.as_ref(), &payload.username, &payload.password).await {
        Ok(user) => {
            let permissions = user.role;
            let claims = Claims::new(user.username, permissions);
            let jwt = create_jwt(claims)
                .map_err(|e| Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

            Ok(Json(serde_json::json!({ "token": jwt })))
        }
        Err(err) => Err(err),
    }
}