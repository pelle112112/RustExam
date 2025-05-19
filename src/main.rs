mod db;
mod auth;
use db::*;
use serde_json;
use auth::jwt::{Claims, create_jwt};
use auth::middleware::JwtMiddleware;
use futures_util::stream::TryStreamExt;

use poem::{
    get, post, handler, listener::TcpListener, Route, Server, Response,
    web::{Json, Path, Multipart, Data},
    EndpointExt,
    http::{HeaderValue,StatusCode},
    IntoResponse,
    Result,
    Response,
    Error
};

use mongodb::{
    bson::{doc, Document, Binary, Bson},
    bson::spec::BinarySubtype,
    Client, Collection,
};

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
    role: Vec<String>
}

impl User {
    pub fn new(username: String, password: String, role: Vec<String>) -> Self {
        Self {
            username,
            password,
            role
        }
    }
}

// Handles POST requests to /add_person. The #[handler] prefix is for poem to recognize it
// This function receives JSON data like { "name": "Alice" } and deserializes it
// into a Person, and inserts it into the MongoDB collection.
//
// If the insert is successful, it returns HTTP 201 Created.
// If the insert fails, it returns HTTP 500 Internal Server Error.
#[poem_grants::protect("admin")]
#[handler]
async fn add_user(
    Json(payload): Json<User>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, poem::error::Error> {
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
async fn get_user(
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
async fn user_update(
    Path(name): Path<String>,
    Json(payload): Json<User>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, poem::error::Error> {
    let collection = db.as_ref();
    // Attempt to update the Person document with the new name.
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

#[derive(Debug, Serialize, Deserialize)]
struct ImageDocument {
    filename: String,
    data: Binary,
}

#[handler]
async fn upload_image(
    mut multipart: Multipart,
    db: poem::web::Data<&Arc<Collection<ImageDocument>>>,
) -> Result<String, StatusCode> {
    let image_collection = db.as_ref();
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("file") {
            let filename = field.file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "upload".to_string());

            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();

            let image_doc = ImageDocument {
                filename: filename.clone(),
                data: bson::Binary {
                    subtype: bson::spec::BinarySubtype::Generic,
                    bytes,
                },
            };

            match insert_image(image_collection, image_doc).await {
                Ok(_) => return Ok(format!("Uploaded {}", filename)),
                Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }

    Err(StatusCode::BAD_REQUEST)
}


#[handler]
pub async fn download_image(
    Path(filename): Path<String>,
    db: Data<&Arc<Collection<ImageDocument>>>,
) -> Result<Response, Error> {
    match db::get_image_by_filename(&**db, &filename).await {
        Ok(Some(image_doc)) => {
            let content_disposition = format!("attachment; filename=\"{}\"", image_doc.filename);

            let mut response = image_doc.data.bytes.into_response();
            response.headers_mut().insert(
                "Content-Disposition",
                HeaderValue::from_str(&content_disposition).unwrap(),
            );
            response.headers_mut().insert(
                "Content-Type",
                HeaderValue::from_static("application/octet-stream"),
            );

            Ok(response)
        }
        Ok(None) => Err(Error::from_status(StatusCode::NOT_FOUND)),
        Err(_) => Err(Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}


#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
async fn user_delete(
    Path(username): Path<String>,
    db: Data<&Arc<Collection<User>>>,
) -> Result<StatusCode, poem::error::Error> {
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
async fn login(Json(payload): Json<LoginInfo>, db: Data<&Arc<Collection<User>>>) -> poem::Result<impl IntoResponse> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(poem::Error::from_string("Either username or password is missing", StatusCode::UNAUTHORIZED));
    }

    match db::login(db.as_ref(), &payload.username, &payload.password).await {
        Ok(user) => {
            let permissions = user.role;
            let claims = Claims::new(user.username, permissions);
            let jwt = create_jwt(claims)
                .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

            Ok(Json(serde_json::json!({ "token": jwt })))
        }
        Err(err) => Err(err),
    }
}

// Sends a JSON response with all the files in the mongoDB
//
// Arguments: Takes a mongodb collection. Collection<Document> is a generic mongodb collection with untyped BSON documents
//
// The cursor looks with doc! which matches with everything in the mongodb
// the Vec::new is a new dynamic array for the filenames
//
// "While let some" keeps looking as long as we get a document returned.
// try_next returns a Result<Option<Document>>
// We convert the BSON value to a string and push the filename to our array.
// We then return the documents in JSON format.


// This data structure stores the mongodb id and filename
// Its annotated with #derive(serialize) to automatically convert the data into JSON string format.
// We got a lot of errors like "the trait bound is not satisfied" without the annotation.
#[derive(Serialize)]
struct FileEntry {
    id: String,
    filename: String,
}

#[poem_grants::protect("user")]
#[handler]
async fn get_files(db: Data<&Arc<Collection<Document>>>) -> Result<Json<Vec<FileEntry>>, StatusCode> {
    let mut cursor = db.find(doc! {}).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut files = Vec::new();

    while let Some(doc) = cursor.try_next().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        if let (Some(id), Some(filename)) = (doc.get_object_id("_id").ok(), doc.get_str("filename").ok()) {
            files.push(FileEntry {
                id: id.to_hex(),
                filename: filename.to_string(),
            });
        }
    }

    Ok(Json(files))
}



// Handles upload of files endpoint to DB
//
// Arguments: takes a multipart files and Collection<Document> which is a generic mongodb collection with untyped BSON documents
// Returns a string message with code 200 when file has been uploaded
//
// while let loops through multiple uploaded files.
// ok(some) matches on the result
// multipart.next_field() gets the next uploaded part (file)
// We get the filename and assign it to the var filename, but default to file.bin if we cant get it for some reason
// We then read the whole file into memory (Bytes) and turn it into a byte array (Vec<u8>)
// Lastly we create a mongodb document with the filename and content (BSON)
// We then insert it into the db with insert_one
#[poem_grants::protect("user")]
#[handler]
async fn upload_file(mut multipart: Multipart, db: Data<&Arc<Collection<Document>>>) -> Result<String, StatusCode> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name().unwrap_or("file.bin").to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let buffer = data.to_vec();

        let file_doc = doc! {
            "filename": &filename,
            "content": Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: buffer,
            }),
        };

        let result = db.insert_one(file_doc)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Return the inserted file's ID
        return Ok(result.inserted_id.to_string());
    }

    Err(StatusCode::BAD_REQUEST)
}


// This endpoint is made to handle the download of a selected file.
//
// Arguments: path id and same as before Collection of documents
// Returns: this handler returns a status code as response.
//
// We create a filter query where we search for a specific filename
// We use the filter with a find_one look in the mongodb. If not found, we return an internal server error
// "if let Some(Bson::Binary(bin))" checks if theres a content field, and if the field is binary.
// the "let response" builds an http response. The "Content-Disposition" triggers a download in the browser for the selected file.
// body(..) Sends the file content and copies the bytes of the content field.
#[poem_grants::protect("user")]
#[handler]
async fn download_file(Path(id): Path<String>, db: Data<&Arc<Collection<Document>>>) -> Result<Response, StatusCode> {
    use mongodb::bson::oid::ObjectId;

    // Convert the string ID from the URL to a MongoDB ObjectId
    let obj_id = ObjectId::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let filter = doc! { "_id": obj_id };

    if let Some(doc) = db.find_one(filter).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        if let (Some(Bson::Binary(bin)), Some(Bson::String(filename))) = (
            doc.get("content"),
            doc.get("filename"),
        ) {
            let response = poem::Response::builder()
                .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
                .body(bin.bytes.clone());

            return Ok(response);
        }
    }

    Err(StatusCode::NOT_FOUND)
}






// The main entry point for the application, setting up the server and MongoDB connection.
//
// # Steps
// 1. Connects to the MongoDB server at `localhost:27017`.
// 2. Selects (or creates) the database `my_api` and collection `Persons`.
// 3. Sets up the API routes using Poem.


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let db = client.database("my_api");

    let collection = Arc::new(db.collection::<Person>("persons"));
    let image_collection = Arc::new(db.collection::<ImageDocument>("images"));
    let files_collection = Arc::new(db.collection::<Document>("files")); // For file binary data

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
        .at("/login", post(login))
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