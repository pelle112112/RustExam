use bson::Binary;
use mongodb::{error::Error, Collection, bson::doc};
use serde::{Deserialize, Serialize};

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