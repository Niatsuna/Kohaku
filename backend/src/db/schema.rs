// @generated automatically by Diesel CLI.

diesel::table! {
    urls (id) {
        id -> Int4,
        addr -> Varchar,
        last_scraped -> Timestamptz,
    }
}
