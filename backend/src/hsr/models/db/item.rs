use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::hsr_items;

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = hsr_items)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub rarity: i32,
    pub description: Option<String>,
    pub description_bg: Option<String>,

    pub types: Vec<String>,
    pub sources: Vec<String>,
    pub item_group: Option<i32>,

    pub api_url: String,
    pub wiki_url: String,
    pub img_url: String,
}

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = hsr_items)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ShortItem {
    pub id: i32,
    pub name: String,
    pub rarity: i32,
    pub types: Vec<String>,
}

#[derive(AsChangeset, Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = hsr_items)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewItem {
    pub name: String,
    pub rarity: i32,
    pub description: Option<String>,
    pub description_bg: Option<String>,

    pub types: Vec<String>,
    pub sources: Vec<String>,
    pub item_group: Option<i32>,

    pub api_url: String,
    pub wiki_url: String,
    pub img_url: String,
}
