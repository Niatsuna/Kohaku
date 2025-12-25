use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, query_dsl::methods::FilterDsl, QueryDsl};
use serde::{self, Deserialize, Serialize};

use crate::{
    db::{
        get_connection,
        schema::{notification_codes, notification_targets},
    },
    utils::{comm::notify_client, error::KohakuError},
};

// =================== Notification Codes =================== //
/// Code Registry Entry - Depicts one TOPIC that can be subsribed to.
///
/// Fields:
///
/// - `code : String` - Identifier to subscribe to.
/// - `last_used : NaiveDateTime` - Timestamp of last received data (UTC), if no data was sent it is the timestamp of creation.
/// - `description : Option<String>` - Optional description to describe what can be subscribed to.
#[derive(Queryable, Identifiable, Selectable, AsChangeset, Insertable, Serialize)]
#[diesel(table_name = crate::db::schema::notification_codes)]
#[diesel(primary_key(code))]
pub struct NotificationCode {
    pub code: String,
    pub last_used: NaiveDateTime,
    pub description: Option<String>,
}

/// Registers a new code in the database.
///
/// Arguments:
///
/// - `code : &str` - Identifier for topic.
/// - `description : Option<String>` - Optional description to describe what can be subscribed to.
///
/// Return:
/// Either the registered `NotificationCode` struct or `KohakuError` if something went wrong.
pub fn register(code: &str, description: Option<String>) -> Result<NotificationCode, KohakuError> {
    let mut conn = get_connection()?;

    let entry = NotificationCode {
        code: code.to_string(),
        last_used: Utc::now().naive_utc(),
        description,
    };

    diesel::insert_into(notification_codes::table)
        .values(&entry)
        .get_result(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Updates the `last_used` timestamp for a given code.
///
/// Arguments:
/// - `code : &str` - Identifier for topic
///
/// Return:
/// Either the updated `NotificationCode` struct or `KohakuError` if something went wrong.
pub fn update_code_ts(code: &str) -> Result<NotificationCode, KohakuError> {
    let mut conn = get_connection()?;
    let new_ts = Utc::now().naive_utc();

    diesel::update(notification_codes::table.find(code))
        .set(notification_codes::last_used.eq(new_ts))
        .get_result(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Returns data of a single given code
///
/// Arguments:
/// - `code : String` - Identifier for topic
///
/// Returns:
/// - `NotificationCode` or a `KohakuError` if the operation failed.
pub fn get_code(code_param: String) -> Result<NotificationCode, KohakuError> {
    let mut conn = get_connection()?;
    let query = FilterDsl::filter(
        notification_codes::table,
        notification_codes::code.eq(code_param),
    );

    query
        .first::<NotificationCode>(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Returns a vector with all available registred codes.
pub fn get_all_codes() -> Result<Vec<NotificationCode>, KohakuError> {
    let mut conn = get_connection()?;
    notification_codes::table
        .load(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Removes a code from the code registry.
/// This will also remove any notifaction targets (subscriptions) for that code to ensure data consistency.
///
/// Arguments:
/// - `code : String` - Identifer for topic.
pub fn unregister(code: String) -> Result<(), KohakuError> {
    let mut conn = get_connection()?;

    diesel::delete(notification_codes::table.find(code))
        .execute(&mut conn)
        .map_err(KohakuError::DatabaseError)?;
    Ok(())
}

// =================== Notification Targets =================== //
#[derive(Serialize, Deserialize, Queryable, Identifiable, Associations, Selectable, Debug)]
#[diesel(table_name = crate::db::schema::notification_targets)]
#[diesel(belongs_to(NotificationCode, foreign_key = code))]
pub struct NotificationTarget {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub code: String,
    pub channel_id: i64,
    pub guild_id: i64,
    pub format: Option<String>,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, AsChangeset, Associations, Debug)]
#[diesel(table_name = crate::db::schema::notification_targets)]
#[diesel(belongs_to(NotificationCode, foreign_key = code))]
pub struct NewNotificationTarget {
    pub code: String,
    pub channel_id: i64,
    pub guild_id: i64,
    pub format: Option<String>,
}

/// Subscribes a channel in a given guild to a topic indicated by a code.
///
/// Arguments:
/// - `code: &str` - Identifier for the to be subscribed topic
/// - `channel_id: i64` - Discord given channel id
/// - `guild_id : i64` - Discord given guild id
/// - `format : Option<String>` - An optional format string that allows for customed designed messages. Will be used by the client to style each message.
///
/// Returns:
/// Either the registered `NotificationTarget` struct or a `KohakuError` if the operation fails.
pub fn subscribe(
    code: &str,
    channel_id: i64,
    guild_id: i64,
    format: Option<String>,
) -> Result<NotificationTarget, KohakuError> {
    let mut conn = get_connection()?;

    let target = NewNotificationTarget {
        code: code.to_string(),
        channel_id,
        guild_id,
        format,
    };

    diesel::insert_into(notification_targets::table)
        .values(&target)
        .get_result(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Returns all active subscriptions of a given guild, channel and code.
///
/// Arguments:
/// - `code_param : Option<&str>` - Optional: Topic identifier
/// - `channel_id_param : Option<i64>` - Optional: Channel identifier
/// - `guild_id_param : Option<i64>` - Optional: Guild Id to looks for.
///
/// Note: At least one argument must be set otherwise a `KohakuError::ValidationError` will be sent.
///
/// Returns:
/// Either a list of active subscriptions or a `KohakuError` if a operation failed.
pub fn get_subscriptions(
    code_param: Option<String>,
    channel_id_param: Option<i64>,
    guild_id_param: Option<i64>,
) -> Result<Vec<NotificationTarget>, KohakuError> {
    if code_param.is_none() && channel_id_param.is_none() && guild_id_param.is_none() {
        return Err(KohakuError::ValidationError(
            "Invalid arguments! At least one argument must be set!".to_string(),
        ));
    }

    use crate::db::schema::notification_targets::dsl::*;
    let mut conn = get_connection()?;

    let mut query = notification_targets.into_boxed();

    if let Some(c) = code_param {
        query = FilterDsl::filter(query, code.eq(c));
    }

    if let Some(chn) = channel_id_param {
        query = FilterDsl::filter(query, channel_id.eq(chn));
    }

    if let Some(g) = guild_id_param {
        query = FilterDsl::filter(query, guild_id.eq(g));
    }

    query
        .load::<NotificationTarget>(&mut conn)
        .map_err(KohakuError::DatabaseError)
}

/// Removes a active subscription of a given channel
///
/// Arguments:
/// - `code_param : &str` - Topic identifier
/// - `channel_id_param : i64` - Channel identifier
/// - `guild_id_param : i64` - Guild identifier
///
/// Returns:
/// Raises a `KohakuError` if a operation fails.
pub fn unsubscribe(
    code_param: &str,
    channel_id_param: i64,
    guild_id_param: i64,
) -> Result<(), KohakuError> {
    use crate::db::schema::notification_targets::dsl::*;
    let mut conn = get_connection()?;

    diesel::delete(notification_targets)
        .filter(
            code.eq(code_param)
                .and(channel_id.eq(channel_id_param))
                .and(guild_id.eq(guild_id_param)),
        )
        .execute(&mut conn)
        .map_err(KohakuError::DatabaseError)?;
    Ok(())
}

// =================== Notifications =================== //
/// In bound data to be send to a client per subscription.
/// This data is used to hold the actual data and is further modified by the format option from `NotificationTarget`.
///
/// Fields:
/// - `triggering_event : String` - Identifier how the event was triggered. Mainly for debugging purposes. (Example: `game1-news-scraper`, `github-release`)
/// - `channel_id : i64` - Identifier for the target channel.
/// - `guild_id : i64` - Identifier for the target guild.
/// - `embed : Option<serde_json::Value>` - Embed data. If the client is suppose to post an embed with the actual message.
/// - `message : Option<String>` - Text Input content. This field is modified by the format option given to each subscription. Please see below at `Message Formatting` for an example and further details.
///
/// Note: If `embed` and `message` are both empty, nothing will be sent to the client, as empty messages have no purpose.
///
/// Message Formatting
/// The field `message` is modified by the data stored in `format` in `NotificationTarget`.
/// The format can include mentions of roles and guild-available emotes. If the format features a field `{content}` the actual content of message will be substituted in it.
/// If the format is empty, but the message is not, the pure message is sent.
/// If the format is non-empty, but the message is empty, the pure format is sent instead.
/// If both are empty, only the available embed data is sent or if not applicable nothing is sent.
#[derive(Serialize, Deserialize)]
pub struct NotificationData {
    pub triggering_event: String,
    pub channel_id: i64,
    pub guild_id: i64,
    pub embed: Option<serde_json::Value>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NotificationPayload {
    pub code: String,
    pub timestamp: NaiveDateTime,
    pub data: Vec<NotificationData>,
}

/// Notifies the client to send data to subscribed channels based on data derived from a triggering event.
///
/// Arguments:
/// - `code : &str` - Topic Identifier. Used to look up channels to send to.
/// - `triggering_event : &str` - Event identifier. Mainly for debugging purposes.
/// - `embed : Option<serde_json::Value>` - Embed data.
/// - `message : Option<String>` - Text content. This field gets modified by the subscriptions format field. Please see `NotificationData` for more details`
pub async fn notify(
    code: &str,
    triggering_event: &str,
    embed: Option<serde_json::Value>,
    message: Option<String>,
) -> Result<(), KohakuError> {
    // Get all applicable subscriptions
    let subscriptions = get_subscriptions(Some(code.to_string()), None, None)?;
    let mut target_data: Vec<NotificationData> = Vec::new();

    // Convert
    for target in subscriptions {
        let embed = embed.clone();
        let message = message.clone();
        if target.format.is_some() || embed.is_some() || message.is_some() {
            // Non-empty message -> Proceed

            let msg = match (target.format, message) {
                (Some(fmt), Some(m)) => Some(fmt.replace("{message}", &m)),
                (Some(fmt), None) => Some(fmt),
                (None, Some(m)) => Some(m),
                (None, None) => None,
            };

            let data = NotificationData {
                triggering_event: triggering_event.to_string(),
                channel_id: target.channel_id,
                guild_id: target.guild_id,
                embed,
                message: msg,
            };

            target_data.push(data);
        }
    }

    // Construct Payload
    let payload = NotificationPayload {
        code: code.to_string(),
        timestamp: Utc::now().naive_utc(),
        data: target_data,
    };
    // Send
    notify_client(payload).await
}
