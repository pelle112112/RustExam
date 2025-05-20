use bson::{Binary, Document, doc, binary};
use futures_util::stream::Collect;
use mongodb::{error::Error, Collection, bson::oid::ObjectId};
use poem::http::StatusCode;
use poem::web::Json;
use serde::{Deserialize, Serialize};
use futures_util::stream::TryStreamExt;



#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: String,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageDocument {
    pub filename: String,
    pub data: Binary,
}

pub async fn insert_image(
    collection: &Collection<ImageDocument>,
    image: ImageDocument,
) -> mongodb::error::Result<()> {
    collection.insert_one(image).await?;
    Ok(())
}

pub async fn get_image_by_filename(
    collection: &Collection<ImageDocument>,
    filename: &str,
) -> Result<Option<ImageDocument>, Error> {
    let filter = doc! { "filename": filename };
    collection.find_one(filter).await
}


#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentEntry {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub filename: String,
    pub content: Binary,
    pub user: String,
}

pub async fn insert_document(
    collection: &Collection<DocumentEntry>,
    document: DocumentEntry,
) -> Result<ObjectId, Error> {
    let result = collection.insert_one(document).await?;
    result.inserted_id.as_object_id().ok_or_else(|| {
        Error::from(std::io::Error::new(std::io::ErrorKind::Other, "Missing ObjectId"))
    })
}

pub async fn get_document_by_id(
    collection: &Collection<DocumentEntry>,
    id: &str,
) -> Result<Option<DocumentEntry>, Error> {
    let obj_id = ObjectId::parse_str(id)
        .map_err(|_| Error::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid ObjectId")))?;;
    let filter = doc! { "_id": obj_id };
    collection.find_one(filter).await
}

pub async fn get_documents_for_user(
    collection: &Collection<DocumentEntry>,
    username: &str,
) -> Result<Vec<FileEntry>, Error> {
    let filter = doc! { "user": username };
    let mut cursor = collection.find(filter).await?;
    let mut files = Vec::new();

    while let Some(doc) = cursor.try_next().await? {
        if let (Some(id), filename) = (doc.id, doc.filename) {
            files.push(FileEntry {
                id: id.to_hex(),
                filename,
            });
        }
    }

    Ok(files)
}