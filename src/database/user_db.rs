use mongodb::{bson::doc, Collection, IndexModel, options::{IndexOptions}};
use poem::{http::StatusCode, Error as PoemError};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
    pub role: Vec<String>
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

 pub async fn insert_user(
     collection: &Collection<User>,
     user: &User,
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


pub async fn find_user(
    collection: &Collection<User>,
    username: &str,
) -> mongodb::error::Result<Option<User>> {
    // Create a filter to search for a document with the specified "name" field.
    let filter = doc! { "username": username };
    // Perform the query to find the user by name.
    collection.find_one(filter).await
}


pub async fn update_user(
    collection: &Collection<User>,
    username: &str,
    new_user_details: &User,
) -> Result<(), PoemError> {
    match find_user(collection, username).await{
        Ok(_) => {
            let update = doc! { "$set": { "username": &new_user_details.username, "password": &new_user_details.password, "role": &new_user_details.role } };
            let result = collection.update_one(doc! {"username": username}, update).await;
            match result {
                Ok(_) => Ok(()),
                Err(_) => Err(PoemError::from_string("Can't change username because it is already taken",StatusCode::CONFLICT))
            }
        }
        Err(_) => {
            Err(PoemError::from_string("Internal server error", StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

pub async fn delete_user(
    collection: &Collection<User>,
    username: &str,
) -> Result<(), PoemError> {
    // Create a filter to find the user by name.
    let filter = doc! { "username": username };
    // Execute the delete operation.
    match collection.delete_one(filter).await {
        Ok(deleted) => {
            if deleted.deleted_count == 0 {
                return Err(PoemError::from_string("The user you are trying to delete doesn't exist.", StatusCode::NOT_FOUND))
            }
            Ok(())
        },
        Err(_) => Err(PoemError::from_status(StatusCode::INTERNAL_SERVER_ERROR))
    }
}
 
 pub async fn login(collection: &Collection<User>, username: &str, password: &str) -> Result<User, PoemError>{
     // Attempt to find the user by username
     let user = collection
         .find_one(doc! { "username": username })
         .await
         .map_err(|e| {
             eprintln!("DB error: {}", e);
             PoemError::from_string("Database error", StatusCode::INTERNAL_SERVER_ERROR)
         })?
         .ok_or_else(|| {
             // If no user is found
             PoemError::from_string("Invalid username or password", StatusCode::UNAUTHORIZED)
         })?;

     // Password check
     if user.password != password {
         return Err(PoemError::from_string(
             "Invalid username or password",
             StatusCode::UNAUTHORIZED,
         ));
     }

     Ok(user)
 }

 pub async fn initial_user_db_setup(collection: &Collection<User>) -> mongodb::error::Result<bool> {

     let index_model = IndexModel::builder()
         .keys(doc! { "username": 1 })
         .options(
             IndexOptions::builder()
                 .unique(true)
                 .name("username_unique_index".to_string())
                 .build(),
         )
         .build();

     match collection.create_index(index_model).await {
         Ok(_) => println!("Index on username is created or already exists"),
         Err(_) => println!("Failed to create index")
     }
     
     let users_to_find :Vec<&str> = ["test", "test2"].to_vec();

     let cursor = collection.find(doc! {"username" : {"$in" : &users_to_find}}).await?;
     let test_users: Vec<User> = cursor.try_collect().await?;
     let mut admin_vector = Vec::new();
     admin_vector.push("admin".to_string());
     admin_vector.push("user".to_string());
     let mut user_vector = Vec::new();
     user_vector.push("user".to_string());
     if test_users.is_empty() {
         println!("No test users found - creating 2 test users.");
         let test_user_1 : User = User::new("test".to_string(), "test".to_string(), admin_vector);
         let test_user_2 : User = User::new("test2".to_string(), "test".to_string(), user_vector);
         if insert_user(collection, &test_user_1).await.is_ok() && insert_user(collection, &test_user_2).await.is_ok() {
             println!("Created 2 test users:");
             println!("{:?}", test_user_1);
             println!("{:?}", test_user_2);
         }
         
     } else if test_users.len() == 1 {
         println!("Found 1 existing user:");
         println!("{:?}", test_users[0]);
         if test_users[0].username.eq("test"){
            let test_user_2 : User = User::new("test2".to_string(), "test".to_string(), user_vector);
            let _ = insert_user(collection, &test_user_2).await;
            println!("Created following user");
             println!("{:?}", test_user_2)
         } else {
             let test_user_1 : User = User::new("test".to_string(), "test".to_string(), admin_vector);
             let _ = insert_user(collection, &test_user_1).await;
             println!("Created following user");
             println!("{:?}", test_user_1)
         }
     } else {
         println!("Retrieved {} test users:", test_users.len());
         for user in &test_users {
             println!("{:?}", user);
         }
     }

     Ok(true)
 }
