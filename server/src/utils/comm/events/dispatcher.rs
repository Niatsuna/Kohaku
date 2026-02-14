use itertools::Itertools;
use serde_json::Value;

use crate::utils::{
    comm::{
        events::models::{get_subscription, get_topic, EventData, EventMessage},
        websocket::manager::get_manager,
    },
    error::KohakuError,
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
    let _ = get_topic(None, name.clone()).await?;
    let subs = get_subscription(name, None, None).await?;

    let grouped = subs.into_iter().into_group_map_by(|s| s.target_uuid);
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
        manager.send_to_client(&target, &message).await?;
    }
    Ok(())
}
