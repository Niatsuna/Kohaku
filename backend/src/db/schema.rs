// @generated automatically by Diesel CLI.

diesel::table! {
    hsr_items (id) {
        id -> Int4,
        name -> Varchar,
        rarity -> Int4,
        description -> Nullable<Varchar>,
        description_bg -> Nullable<Varchar>,
        types -> Array<Text>,
        sources -> Array<Text>,
        item_group -> Nullable<Int4>,
        api_url -> Varchar,
        wiki_url -> Varchar,
        img_url -> Varchar,
    }
}

diesel::table! {
    urls (id) {
        id -> Int4,
        addr -> Varchar,
        last_scraped -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(hsr_items, urls,);
