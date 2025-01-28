use serde::{Deserialize, Serialize};

use crate::hsr::models::db::item::Item as DatabaseItem;

/// Represents a direct response.
#[derive(Serialize, Deserialize)]
pub struct DirectItem {
    #[serde(flatten)]
    pub db: DatabaseItem,
}
