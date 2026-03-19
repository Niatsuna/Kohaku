// @generated automatically by Diesel CLI.

diesel::table! {
    api_keys (id) {
        id -> Int4,
        #[max_length = 255]
        hashed_key -> Varchar,
        #[max_length = 10]
        key_prefix -> Varchar,
        #[max_length = 255]
        owner -> Varchar,
        scopes -> Array<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Int4,
        topic_id -> Int4,
        key_id -> Int4,
        target_data -> Nullable<Jsonb>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    topics (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        description -> Varchar,
        #[max_length = 255]
        details -> Nullable<Varchar>,
        created_at -> Timestamp,
    }
}

diesel::joinable!(subscriptions -> api_keys (key_id));
diesel::joinable!(subscriptions -> topics (topic_id));

diesel::allow_tables_to_appear_in_same_query!(api_keys, subscriptions, topics,);
