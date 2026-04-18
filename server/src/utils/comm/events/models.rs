use chrono::NaiveDateTime;
use diesel::{prelude::*, query_dsl::methods::FilterDsl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{db::schema, utils::error::KohakuError};
// ========================================== MESSAGE ========================================== //

/// Actual inner data send in events
#[derive(Debug, Serialize, Deserialize)]
pub struct EventData {
    /// Actual content derived from the event (e.g. a message, link or anything else)
    pub content: serde_json::Value,
    /// Target data stored in the subscription for client sided handling (e.g. Discord channel and guild ids)
    pub target_data: Vec<serde_json::Value>,
}

/// Message struct that get send to the client fromt he dispatcher
#[derive(Debug, Serialize, Deserialize)]
pub struct EventMessage {
    /// Origin of the event on the servers side
    pub source: String,
    /// Topic name
    pub topic: String,
    /// Type of Event (e.g. Notify, Remove, etc.)
    pub instruction: String,
    /// Actual inner data send. Includes the content as well as the target data
    pub data: EventData,
}

// ======================================= SUBSCRIPTION ======================================== //

/// Represents a subscription of a topic for a target (client)
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::subscriptions)]
pub struct Subscription {
    pub id: i32,
    pub topic_id: i32,
    pub key_id: i32,
    pub target_data: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Clone)]
#[diesel(table_name = crate::db::schema::subscriptions)]
pub struct NewSubscription {
    topic_id: i32,
    key_id: i32,
    target_data: Option<Value>,
}

/// Represents input data for endpoints to create subscriptions
#[derive(Debug, Deserialize, Clone)]
pub struct CreateSubscription {
    pub topic: String,
    pub target_uuid: Uuid,
    pub target_data: Option<Value>,
}

/// Represents input data for endpoints to delete subscriptions
#[derive(Debug, Deserialize, Clone)]
pub struct DeleteSubscription {
    pub topic: Option<String>,
    pub target_uuid: Uuid,
    pub target_data: Option<Value>,
}

/// Creates an entry for a new subscription in the database
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `topic` : [`String`] representation of the topic name
/// - `key_id` : [`i32`] of the subscribing clients API Key (e.g. Discord Client)
/// - `target_data` : Optional additional information for the resulting event (e.g. Discord channel id and guild id)
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`Subscription`] that mirrors the now stored subscription entry in the database
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn create_subscription<T: Serialize>(
    conn: &mut PgConnection,
    topic: String,
    key_id: i32,
    target_data_: Option<T>,
) -> Result<Subscription, KohakuError> {
    let topic_ = get_topic(conn, None, Some(topic)).await?;
    let data = serde_json::to_value(target_data_)
        .map_err(|_| KohakuError::ValidationError("Malformed target data!".to_string()))?;

    let new_subscription = NewSubscription {
        topic_id: topic_.id,
        key_id,
        target_data: Some(data),
    };

    diesel::insert_into(schema::subscriptions::table)
        .values(&new_subscription)
        .get_result(conn)
        .map_err(KohakuError::DatabaseQueryError)
}

/// Gets stored subscriptions based on either the topic name or the targets uuid.
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `topic_` : Topic name.
/// - `key_id_` : Client identifier based on associated api key.
/// - `target_data_` : Additional target data for unique identification of subscription.
///
/// Either `topic_` or `key_id_` must be set.
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : Vector of [`Subscription`]s
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn get_subscription(
    conn: &mut PgConnection,
    topic_: Option<String>,
    key_id_: Option<i32>,
    target_data_: Option<Value>,
) -> Result<Vec<Subscription>, KohakuError> {
    use crate::db::schema::subscriptions::dsl::*;
    if topic_.is_none() && key_id_.is_none() {
        return Err(KohakuError::ValidationError(
            "Illegal Argument: At least one of the parameters must be set!".to_string(),
        ));
    }
    let mut query = subscriptions.into_boxed();

    if topic_.is_some() {
        let topic = get_topic(conn, None, topic_).await?;
        query = FilterDsl::filter(query, topic_id.eq(topic.id));
    }

    if let Some(k) = key_id_ {
        query = FilterDsl::filter(query, key_id.eq(k));
    }

    if let Some(td) = target_data_ {
        query = FilterDsl::filter(query, target_data.eq(td));
    }

    query.load(conn).map_err(KohakuError::DatabaseQueryError)
}

/// Deletes a prior stored subscription from the database
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `topic_` : Topic name.
/// - `key_id_` : Client identifier based on associated api key.
/// - `target_data_` : Additional target data for unique identification of subscription.
///
/// Either `topic_` or `key_id_` must be set.
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : Indicating the subscription is now deleted
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn delete_subscription(
    conn: &mut PgConnection,
    topic_: Option<String>,
    key_id_: Option<i32>,
    target_data_: Option<Value>,
) -> Result<(), KohakuError> {
    use crate::db::schema::subscriptions::dsl::*;
    if topic_.is_none() && key_id_.is_none() {
        return Err(KohakuError::ValidationError(
            "Illegal Argument: At least one of the parameters must be set!".to_string(),
        ));
    }
    let mut query = diesel::delete(subscriptions).into_boxed();

    if topic_.is_some() {
        let topic = get_topic(conn, None, topic_).await?;
        query = FilterDsl::filter(query, topic_id.eq(topic.id));
    }

    if let Some(k) = key_id_ {
        query = FilterDsl::filter(query, key_id.eq(k));
    }

    if let Some(td) = target_data_ {
        query = FilterDsl::filter(query, target_data.eq(td));
    }

    query
        .execute(conn)
        .map_err(KohakuError::DatabaseQueryError)?;
    Ok(())
}

// =========================================== TOPIC =========================================== //

/// Represents a subscription topic for events.
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::topics)]
pub struct Topic {
    /// Serial id in the database
    pub id: i32,
    /// Given name that must be used to subscribe and notify to this topic
    pub name: String,
    /// Description of the content of said topic
    pub description: String,
    //// Description what the conent will be formatted to (e.g. {content} = URL)
    pub details: Option<String>,
    //// Timestamp of creation in the database
    pub created_at: NaiveDateTime,
}

/// Creation form for [Topic]
#[derive(Debug, Insertable, Clone)]
#[diesel(table_name = crate::db::schema::topics)]
struct NewTopic {
    name: String,
    description: String,
    details: Option<String>,
}

/// Creates an entry for a new topic in the database
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `name` : Identifier for the topic. Used to subscribe to it and send notification to subscribed targets. Maximum length is 255 characters
/// - `description` : [`String`] explaining what exactly the topic represents. Maximum length is 255 characters
/// - `details` : Optional string explaining the result and how it can be formatted. Maximum length is 255 characters
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`Topic`] that mirrors the now stored topic entry in the database
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn create_topic(
    conn: &mut PgConnection,
    name: &str,
    description: &str,
    details: Option<String>,
) -> Result<Topic, KohakuError> {
    if name.len() > 255 {
        return Err(KohakuError::ValidationError(
            "Name of topic too long. Maximum length is 255 characters!".to_string(),
        ));
    }

    if description.len() > 255 {
        return Err(KohakuError::ValidationError(
            "Description of topic too long. Maximum length is 255 characters!".to_string(),
        ));
    }

    if details.is_some() && details.clone().unwrap().len() > 255 {
        return Err(KohakuError::ValidationError(
            "Details of topic too long. Maximum length is 255 characters!".to_string(),
        ));
    }

    let new_topic = NewTopic {
        name: name.to_string(),
        description: description.to_string(),
        details,
    };

    diesel::insert_into(schema::topics::table)
        .values(&new_topic)
        .get_result(conn)
        .map_err(KohakuError::DatabaseQueryError)
}

/// Gets all stored topics
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A vector of [`Topic`]s
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn get_all_topics(conn: &mut PgConnection) -> Result<Vec<Topic>, KohakuError> {
    use crate::db::schema::topics::dsl::*;
    topics.load(conn).map_err(KohakuError::DatabaseQueryError)
}

/// Gets a stored topic based on either the id or the name
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `id_` : Serial primary key of the database entry. Either this or `name_` must be set
/// - `name_` : Topic name. Either this or `id_` must be set
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : Identified [`Topic`]
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn get_topic(
    conn: &mut PgConnection,
    id_: Option<i32>,
    name_: Option<String>,
) -> Result<Topic, KohakuError> {
    use crate::db::schema::topics::dsl::*;
    if id_.is_none() && name_.is_none() {
        return Err(KohakuError::ValidationError(
            "Illegal Argument: At least one of the parameters must be set!".to_string(),
        ));
    }
    let mut query = topics.into_boxed();

    if let Some(i) = id_ {
        query = FilterDsl::filter(query, id.eq(i));
    }

    if let Some(n) = name_ {
        query = FilterDsl::filter(query, name.eq(n));
    }

    query
        .get_result(conn)
        .map_err(KohakuError::DatabaseQueryError)
}

/// Deletes a prior stored topic from the database
///
/// # Parameters
/// - `conn` : Mutable [`PgConnection`] reference for database access
/// - `id_` : Serial primary key of the database entry. Either this or `name_` must be set
/// - `name_` : Topic name. Either this or `id_` must be set
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : Indicating the topic is now deleted
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn delete_topic(
    conn: &mut PgConnection,
    id_: Option<i32>,
    name_: Option<String>,
) -> Result<(), KohakuError> {
    use crate::db::schema::topics::dsl::*;
    if id_.is_none() && name_.is_none() {
        return Err(KohakuError::ValidationError(
            "Illegal Argument: At least one of the parameters must be set!".to_string(),
        ));
    }
    let mut query = diesel::delete(topics).into_boxed();

    if let Some(i) = id_ {
        query = FilterDsl::filter(query, id.eq(i));
    }

    if let Some(n) = name_ {
        query = FilterDsl::filter(query, name.eq(n));
    }

    query
        .execute(conn)
        .map_err(KohakuError::DatabaseQueryError)?;
    Ok(())
}
