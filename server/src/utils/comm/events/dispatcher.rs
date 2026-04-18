use itertools::Itertools;
use serde_json::Value;
use tracing::info;

use crate::{
    db::get_connection,
    utils::{
        comm::{
            events::models::{get_subscription, get_topic, EventData, EventMessage},
            websocket::manager::get_manager,
        },
        error::KohakuError,
    },
};

/// Notifies connected client based on topic subscriptions.
///
/// # Parameters
/// - `source` : String identifier of origin of the event
/// - `topic` : Topic name
/// - `instruction` : Type of event for client handling
/// - `data` : Content of the event
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : Indicating a successful operation
/// - [`Err`] : A [`KohakuError`] based on the failing operation
pub async fn notify(
    source: &str,
    topic: &str,
    instruction: &str,
    data: Value,
) -> Result<(), KohakuError> {
    let name = Some(topic.to_string());

    let mut conn = get_connection()?;
    let _ = get_topic(&mut conn, None, name.clone()).await?;
    let subs = get_subscription(&mut conn, name, None, None).await?;
    let grouped = subs.into_iter().into_group_map_by(|s| s.key_id);
    let total = grouped.len();
    let mut connected = 0;
    let mut failed = 0;
    for (target, subs) in grouped {
        let target_data = subs
            .iter()
            .filter_map(|s| s.target_data.clone())
            .collect::<Vec<Value>>();

        let dt = EventData {
            content: data.clone(),
            target_data,
        };

        let message = EventMessage {
            source: source.to_string(),
            topic: topic.to_string(),
            instruction: instruction.to_string(),
            data: dt,
        };

        let manager = get_manager()?;
        if manager.check_if_active(&target) {
            connected += 1;
            if manager.send_to_client(&target, &message).await.is_err() {
                failed += 1;
            }
        }
    }
    if failed != 0 {
        info!("Failed to notify {} out of {} connected clients for the topic '{}'. (Total subscriptions: {})", failed, connected, topic, total)
    } else {
        info!("Successfully notified {} out of {} connected clients for the topic '{}'. (Total subscriptions: {})", connected, connected, topic, total)
    }
    Ok(())
}
