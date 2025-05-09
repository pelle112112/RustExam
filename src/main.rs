mod db;
use db::*;
use poem::{
    get, post, handler, listener::TcpListener, Route, Server,
    web::{Json, Path},
    EndpointExt,
    http::StatusCode
};
use mongodb::{bson::doc, Client, Collection};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// This struct defines the shape of JSON data we receive or send in API requests/responses.
// The `name` field is expected in JSON payloads like: { "name": "Alice" }
// `Serialize` and `Deserialize` allow converting this struct to/from JSON via Serde.
// `Debug` is useful for printing the struct for debugging. See add_person comments for usage
#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: i64
}

// Handles POST requests to /add_person. The #[handler] prefix is for poem to recognize it
// This function receives JSON data like { "name": "Alice" } and deserializes it
// into a Person, and inserts it into the MongoDB collection.
//
// If the insert is successful, it returns HTTP 201 Created.
// If the insert fails, it returns HTTP 500 Internal Server Error.
#[handler]
async fn add_person(
    Json(payload): Json<Person>,
    db: poem::web::Data<&Arc<Collection<Person>>>,
) -> Result<StatusCode, StatusCode> {
    let collection = db.as_ref();
    insert_person(collection, payload).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
async fn get_person(
    Path(name): Path<String>,
    db: poem::web::Data<&Arc<Collection<Person>>>,
) -> Result<Json<Person>, StatusCode> {
    // Get a reference to the MongoDB collection.
    let collection = db.as_ref();

    // Attempt to find a Person document matching the provided name.
    match find_person(collection, &name).await {
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
async fn person_update(
    Path(name): Path<String>,
    Json(payload): Json<Person>,
    db: poem::web::Data<&Arc<Collection<Person>>>,
) -> Result<String, StatusCode> {
    let collection = db.as_ref(); // Extract &Collection<Person>
    // Attempt to update the Person document with the new name.
    match update_person(collection, &name, &payload.name).await {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(format!("Updated Person to '{}'", payload.name)),
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
async fn person_delete(
    Path(name): Path<String>,
    db: poem::web::Data<&Arc<Collection<Person>>>,
) -> Result<String, StatusCode> {
    let collection = db.as_ref(); // Extract &Collection<Person>
    match delete_person(collection, &name).await {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(format!("Deleted Person '{}'", name)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
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
    let collection = db.collection::<Person>("persons");

    // Wrap the collection in an Arc to safely share it across multiple threads.
    let collection = Arc::new(collection);

    // Configure the Poem app with routes for handling various HTTP methods.
    let app = Route::new()
        .at("/hello/:name", get(hello))
        .at("/add_person", post(add_person))
        .at(
            "/person/:name",
            get(get_person)
                .put(person_update)
                .delete(person_delete),
        )
        .data(collection);
    // the route /person/:name is shared and will cause a duplication error.
    // .at("/person/:name", get(get_person))
    // .at("/person/:name", put(person_update))
    // .at("/person/:name", delete(person_delete))
    // to avoid this error we have to group these methods in a single .at()
    // then we can chain the .get .put .delete

    Server::new(TcpListener::bind("localhost:3000"))
        .run(app)
        .await
}