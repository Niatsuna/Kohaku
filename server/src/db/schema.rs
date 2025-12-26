// @generated automatically by Diesel CLI.

diesel::table! {
    api_keys (id) {
        id -> Int4,
        #[max_length = 255]
        hashed_key -> Varchar,
        #[max_length = 6]
        key_prefix -> Varchar,
        #[max_length = 255]
        owner -> Varchar,
        scopes -> Array<Text>,
        created_at -> Timestamp,
    }
}
