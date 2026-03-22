// Allowing as all code in this module is mostly foundational and will be used in the future
// TODO: Remove it, when everything is actually used
#![allow(dead_code)]

use tracing::{error, info};

use crate::utils::{
    comm::{auth::jwt::init_jwtservice, websocket::manager::init_manager},
    config::Config,
    scheduler::{get_scheduler, init_scheduler},
};

pub mod comm;
pub mod config;
pub mod error;
pub mod scheduler;
mod tests;

/// Initializes globally accessible services like scheduler, jwtservice and websocket manager.
///
/// # Parameters
/// - `config` : Reference to an active available [`Config`] from the main method.
pub async fn initialize_services(config: &Config) {
    if let Err(e) = init_scheduler().await {
        error!(
            "Couldn't initialize scheduler! Tasks will not be scheduled! Reason: {}",
            e
        );
    } else {
        info!("Scheduler initialized! Starting ...");
        let scheduler = get_scheduler().await;
        if let Err(er) = scheduler.start().await {
            error!(
                "Couldn't start scheduler! Tasks will not be scheduled! Reason: {}",
                er
            );
        } else {
            info!("Scheduler started!");
            //TODO: Add default tasks here!
        }
    }

    if let Err(e) = init_jwtservice(&config.encryption_key) {
        error!(
            "Couldn't initialize JWT service! Protected endpoints locked! Reason: {}",
            e
        );
    } else {
        info!("JWT Service initialized!");
    }

    if let Err(e) = init_manager() {
        error!(
            "Couldn't initialize websocket manager! Websocket connection deactivated! Reason: {}",
            e
        );
    } else {
        info!("Websocket Manager initialized!");
    }
}
