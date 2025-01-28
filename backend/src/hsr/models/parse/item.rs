use serde::{Deserialize, Serialize};

/// API Responses from api.hakush.in for materials / items

/// Represents a source of an item (Domain, Weekly, Shop etc.)
/// Present in some Item responses
#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    #[serde(rename = "Desc")]
    pub desc: String, // Other fields discarded
}

/// Represents API response for one individual item
#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "Rarity")]
    pub rarity: String,

    #[serde(rename = "PurposeType")]
    pub purpose_type: i32,

    #[serde(rename = "ItemName")]
    pub name: String,

    #[serde(rename = "ItemDesc")]
    pub desc: Option<String>,

    #[serde(rename = "ItemBGDesc")]
    pub bgdesc: Option<String>,

    #[serde(rename = "ItemFigureIconPath")]
    pub icon_path: String,

    #[serde(rename = "ItemGroup", default)]
    pub item_group: Option<i32>,

    #[serde(rename = "ItemComefrom")]
    pub source: Vec<Source>, // Other fields discarded
}

/// Represents API response in the list of items
/// Convert response to a HashMap<String, ListItem>
#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(rename = "PurposeType")]
    pub purpose_type: i32, // Other fields discarded
}
