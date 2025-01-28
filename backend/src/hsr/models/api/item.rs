use serde::{Deserialize, Serialize};

use crate::hsr::models::db::item::Item as DatabaseItem;

/// Represents a direct response.
#[derive(Serialize, Deserialize)]
pub struct DirectItem {
    #[serde(flatten)]
    pub db: DatabaseItem,
}

#[derive(Serialize, Deserialize)]
pub struct SimilarityItemEntry {
    pub name: String,
    pub rarity: i32,
    pub types: Vec<String>,
}

impl From<&DatabaseItem> for SimilarityItemEntry {
    fn from(item: &DatabaseItem) -> Self {
        SimilarityItemEntry {
            name: item.name.clone(),
            rarity: item.rarity.clone(),
            types: item.types.clone(),
        }
    }
}

/// Represents a guessed response list.
/// If the query e.g. has a typo,
/// the backend guesses based on similarity and returns a list of possible entities
#[derive(Serialize, Deserialize)]
pub struct SimilarityItem {
    #[serde(flatten)]
    pub db: Vec<SimilarityItemEntry>,
}
