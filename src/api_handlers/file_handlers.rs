use std::sync::Arc;
use bson::{doc, Binary, Bson, Document};
use bson::spec::BinarySubtype;
use mongodb::Collection;
use poem::{handler, Error, Response, IntoResponse, Request};
use poem::http::{HeaderValue, StatusCode};
use poem::web::{Data, Json, Multipart, Path};
use serde::{Serialize};
use crate::database::file_db::{get_image_by_filename, insert_image, ImageDocument, insert_document, get_document_by_id, DocumentEntry, get_documents_for_user};
use futures_util::stream::TryStreamExt;
use crate::api_handlers::extract_user;
use crate::models::FileEntry;


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
// Arguments: takes a request and a mongodb collection
//
// Returns: a JSON response with the files
//
// We use the get_documents_for_user function to get the files from the mongodb.
// We use the address of a double pointer to the mongodb collection.
// The documents are returned as a vector of FileEntry structs.
// The documents are filtered by the user, so only the files of the user are returned.
// The user is extracted from the request using the extract_user function.
//
// We return a JSON response with the documents.


#[poem_grants::protect("user")]
#[handler]
pub async fn get_files(
    req: &Request,
    db: Data<&Arc<Collection<DocumentEntry>>>,
) -> poem::Result<Json<Vec<FileEntry>>, StatusCode> {
    let user = extract_user(req).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let documents = get_documents_for_user(&**db, &user.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(documents))
}



// Handles upload of files endpoint to DB
//
// Arguments: takes an adress to a request, a multipart form data and a mongodb collection
// Returns: a string with the id of the uploaded file
//
// We use the multipart form data to get the file field.
// The filename is extracted from the field, and if not found, we set it to "upload".
// The bytes are extracted from the field and converted to a vector.
// We create a DocumentEntry struct with the filename, content and user.
//
// The insert_document function is called to insert the document into the mongodb.
// If the insert is successful, we return the id of the document as a hex string.
// If the insert fails, we return an internal server error.
#[poem_grants::protect("user")]
#[handler]
pub async fn upload_file(
    req: &Request,
    mut multipart: Multipart,
    db: Data<&Arc<Collection<DocumentEntry>>>,
) -> poem::Result<String, StatusCode> {
    let user = extract_user(req).map_err(|_| StatusCode::UNAUTHORIZED)?;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("file") {
            let filename = field.file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "upload".to_string());

            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();

            let document = DocumentEntry {
                id: None,  // We set this to None, as MongoDB will generate an ObjectId for us
                filename: filename.clone(),
                content: Binary {
                    subtype: bson::spec::BinarySubtype::Generic,
                    bytes,
                },
                user: user.username,
            };

            match insert_document(db.as_ref(), document).await {
                Ok(id) => return Ok(id.to_hex()),
                Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }

    Err(StatusCode::BAD_REQUEST)
}



// This endpoint is made to handle the download of a selected file.
//
// Arguments: takes a path with the id of the file and a mongodb collection
// Returns: a response with the file content
//
// We use the get_document_by_id function to get the file from the mongodb.
// We use the address of a double pointer to the mongodb collection.
// The filename is extracted from the document and used to set the content-disposition header for the response
// The content type is set to application/octet-stream.


// If the file is not found, we return a 404 Not Found error

#[poem_grants::protect("user")]
#[handler]
pub async fn download_file(
    Path(id): Path<String>,
    db: Data<&Arc<Collection<DocumentEntry>>>,
) -> poem::Result<Response, Error> {
    match get_document_by_id(&**db, &id).await {
        Ok(Some(doc)) => {
            let content_disposition = format!("attachment; filename=\"{}\"", doc.filename);

            let mut response = doc.content.bytes.into_response();
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