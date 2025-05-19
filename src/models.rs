use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: String,
    pub filename: String,
}