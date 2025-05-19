use std::sync::Arc;
use bson::{doc, Binary, Bson, Document};
use bson::spec::BinarySubtype;
use mongodb::Collection;
use poem::{handler, Error, Response, IntoResponse, Request};
use poem::http::{HeaderValue, StatusCode};
use poem::web::{Data, Json, Multipart, Path};
use serde::{Serialize};
use crate::database::file_db::{get_image_by_filename, insert_image, ImageDocument};
use futures_util::stream::TryStreamExt;
use crate::api_handlers::extract_user;

#[poem_grants::protect("user")]
#[handler]
pub async fn upload_image(
    mut multipart: Multipart,
    db: Data<&Arc<Collection<ImageDocument>>>,
) -> poem::Result<String, StatusCode> {
    let image_collection = db.as_ref();
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("file") {
            let filename = field.file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "upload".to_string());

            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();

            let image_doc = ImageDocument {
                filename: filename.clone(),
                data: Binary {
                    subtype: BinarySubtype::Generic,
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

#[poem_grants::protect("user")]
#[handler]
pub async fn download_image(
    Path(filename): Path<String>,
    db: Data<&Arc<Collection<ImageDocument>>>,
) -> poem::Result<Response, Error> {
    match get_image_by_filename(&**db, &filename).await {
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
pub async fn get_files(req: &Request, db: Data<&Arc<Collection<Document>>>) -> poem::Result<Json<Vec<FileEntry>>, StatusCode> {
    let auth_user = extract_user(req).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let filter = doc! { "user": &auth_user.username };
    let mut cursor = db.find(filter).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn upload_file(req: &Request, mut multipart: Multipart, db: Data<&Arc<Collection<Document>>>) -> poem::Result<String, StatusCode> {
    let auth_user = extract_user(req).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
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
            "user": &auth_user.username,
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
pub async fn download_file(Path(id): Path<String>, db: Data<&Arc<Collection<Document>>>) -> poem::Result<Response, StatusCode> {
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