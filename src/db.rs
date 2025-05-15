 use mongodb::{bson::doc,Collection};
 use poem::{http::StatusCode, Error as PoemError};
 use crate::User;
 
 // Inserts a new Person into the MongoDB collection.
//
// # Arguments
// - `collection`: The MongoDB collection where the person will be inserted.
// - `person`: The `Person` object to be inserted.
//
// # Returns
// - `mongodb::error::Result<()>`: Returns an error if the insert fails, or `Ok(())` if successful.
 pub async fn insert_user(
     collection: &Collection<User>,
     user: User,
 ) -> Result<(), PoemError> {
     let existing_user = collection.find_one(doc! {"username": &user.username})
         .await
         .map_err(|e| PoemError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

     if existing_user.is_some() {
         return Err(PoemError::from_string("User with that username already exists", StatusCode::CONFLICT));
     }

     collection.insert_one(user)
         .await
         .map_err(|e| PoemError::new(e, StatusCode::INTERNAL_SERVER_ERROR))?;

     Ok(())
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
pub async fn find_user(
    collection: &Collection<User>,
    username: &str,
) -> mongodb::error::Result<Option<User>> {
    // Create a filter to search for a document with the specified "name" field.
    let filter = doc! { "username": username };
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
pub async fn update_user(
    collection: &Collection<User>,
    old_username: &str,
    new_username: &str,
) -> mongodb::error::Result<u64> {
    // Create a filter to search for the document with the old name.
    let filter = doc! { "username": old_username };
    // Create an update document that sets the "name" field to the new name.
    let update = doc! { "$set": { "username": new_username } };
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
pub async fn delete_user(
    collection: &Collection<User>,
    username: &str,
) -> mongodb::error::Result<u64> {
    // Create a filter to find the person by name.
    let filter = doc! { "username": username };
    // Execute the delete operation.
    let result = collection.delete_one(filter).await?;
    // Return the count of deleted documents.
    Ok(result.deleted_count)
}
 
 
 pub async fn login(collection: &Collection<User>, username: &str, password: &str) -> Result<User, PoemError>{
     // Attempt to find the user by username
     let user = collection
         .find_one(doc! { "username": username })
         .await
         .map_err(|e| {
             eprintln!("DB error: {}", e); // optional: log internal error
             PoemError::from_string("Database error", StatusCode::INTERNAL_SERVER_ERROR)
         })?
         .ok_or_else(|| {
             // If no user is found
             PoemError::from_string("Invalid username or password", StatusCode::UNAUTHORIZED)
         })?;

     // Password check — replace this with proper hash check in production
     if user.password != password {
         return Err(PoemError::from_string(
             "Invalid username or password",
             StatusCode::UNAUTHORIZED,
         ));
     }

     Ok(user)
 }