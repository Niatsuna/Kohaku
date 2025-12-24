// @generated automatically by Diesel CLI.

diesel::table! {
    notification_codes (code) {
        #[max_length = 255]
        code -> Varchar,
        last_used -> Timestamp,
        description -> Nullable<Text>,
    }
}

diesel::table! {
    notification_targets (id) {
        id -> Int4,
        created_at -> Timestamp,
        #[max_length = 255]
        code -> Varchar,
        channel_id -> Int8,
        guild_id -> Int8,
        format -> Nullable<Text>,
    }
}

diesel::joinable!(notification_targets -> notification_codes (code));

diesel::allow_tables_to_appear_in_same_query!(notification_codes, notification_targets,);
