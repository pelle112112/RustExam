use mongodb::{bson::doc, Collection, error::Error};
use crate::{ImageDocument, Person};

// Inserts a new Person into the MongoDB collection.
//
// # Arguments
// - `collection`: The MongoDB collection where the person will be inserted.
// - `person`: The `Person` object to be inserted.
//
// # Returns
// - `mongodb::error::Result<()>`: Returns an error if the insert fails, or `Ok(())` if successful.
pub async fn insert_person(
    collection: &Collection<Person>,
    person: Person,
) -> mongodb::error::Result<()> {
    // Insert the person into the collection. The `map` function is used to convert
    // the result from `InsertOneResult` to `()`.
    collection.insert_one(person).await.map(|_| ())
}

// Finds a person by name in the MongoDB collection.
//
// # Arguments
// - `collection`: The MongoDB collection to search in.
// - `name`: The name of the person to search for.
//
// # Returns
// - `mongodb::error::Result<Option<Person>>`:
//   - `Ok(Some(person))` if a person with the given name is found.
//   - `Ok(None)` if no matching person is found.
//   - `Err(error)` if an error occurs during the query.
pub async fn find_person(
    collection: &Collection<Person>,
    name: &str,
) -> mongodb::error::Result<Option<Person>> {
    // Create a filter to search for a document with the specified "name" field.
    let filter = doc! { "name": name };
    // Perform the query to find the person by name.
    collection.find_one(filter).await
}

// Updates a person's name in the MongoDB collection.
//
// # Arguments
// - `collection`: The MongoDB collection to update.
// - `old_name`: The current name of the person to be updated.
// - `new_name`: The new name to update the person to.
//
// # Returns
// - `mongodb::error::Result<u64>`:
//   - Returns the number of documents matched for the update.
//   - If no documents were matched (i.e., the old name doesn't exist), it returns `Ok(0)`.
//   - If there’s an error during the update, it returns an error.
pub async fn update_person(
    collection: &Collection<Person>,
    old_name: &str,
    new_name: &str,
) -> mongodb::error::Result<u64> {
    // Create a filter to search for the document with the old name.
    let filter = doc! { "name": old_name };
    // Create an update document that sets the "name" field to the new name.
    let update = doc! { "$set": { "name": new_name } };
    // Execute the update operation.
    let result = collection.update_one(filter, update).await?;
    // Return the count of matched documents.
    Ok(result.matched_count)
}

// Deletes a person by name from the MongoDB collection.
//
// # Arguments
// - `collection`: The MongoDB collection to delete from.
// - `name`: The name of the person to be deleted.
//
// # Returns
// - `mongodb::error::Result<u64>`:
//   - Returns the number of documents deleted.
//   - If no document matched the name, it returns `Ok(0)`.
//   - If there’s an error during the delete, it returns an error.
pub async fn delete_person(
    collection: &Collection<Person>,
    name: &str,
) -> mongodb::error::Result<u64> {
    // Create a filter to find the person by name.
    let filter = doc! { "name": name };
    // Execute the delete operation.
    let result = collection.delete_one(filter).await?;
    // Return the count of deleted documents.
    Ok(result.deleted_count)
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
