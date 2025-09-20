use std::{env, sync::Arc};

use crate::utils::config::{get_config, init_config, Config};

use rstest::rstest;
use serial_test::serial;

fn setup_env_vars(only_required: bool) {
    env::set_var("DATABASE_URL", "some_url/db");
    if !only_required {
        // Skip these that are not required to not panic Config::new()
        env::set_var("SERVER_ADDR", "localhost");
        env::set_var("SERVER_PORT", "9000");
        env::set_var("SERVER_LOGGING_LEVEL", "WARN");
        env::set_var("CLIENT_ADDR", "localhost2");
        env::set_var("CLIENT_PORT", "1234");
    }
}

fn cleanup_env_vars() {
    let vars = vec![
        "SERVER_ADDR",
        "SERVER_PORT",
        "SERVER_LOGGING_LEVEL",
        "DATABASE_URL",
        "CLIENT_ADDR",
        "CLIENT_PORT",
    ];
    for v in vars {
        env::remove_var(v);
    }
}

#[test]
#[serial]
#[should_panic]
fn test_config_not_initialized_before_get() {
    let _ = get_config();
}

#[test]
#[serial]
fn test_config_singleton() {
    setup_env_vars(true);

    // #1 Test that first initialization suceeds and second fails
    assert!(init_config().is_ok());
    assert!(init_config().is_err());

    // #2 Test if getter returns same instance
    let c1 = get_config();
    let c2 = get_config();
    assert!(Arc::ptr_eq(&c1, &c2));

    cleanup_env_vars();
}

// ------------------------------------------------------------------------

#[test]
#[serial]
fn test_config_with_env_vars() {
    setup_env_vars(false);

    let config = Config::new();
    assert_eq!(config.server_addr, "localhost");
    assert_eq!(config.server_port, 9000);
    assert_eq!(config.logging_level, tracing::Level::WARN);
    assert_eq!(config.database_url, "some_url/db");
    assert_eq!(config.client_addr, "localhost2");
    assert_eq!(config.client_port, 1234);

    cleanup_env_vars();
}

#[test]
#[serial]
fn test_config_defaults() {
    setup_env_vars(true);

    let config = Config::new();
    assert_eq!(config.server_addr, "127.0.0.1");
    assert_eq!(config.server_port, 8080);
    assert_eq!(config.logging_level, tracing::Level::INFO);
    assert_eq!(config.client_addr, "127.0.0.1");
    assert_eq!(config.client_port, 8081);

    cleanup_env_vars();
}

#[test]
#[serial]
#[should_panic]
fn test_missing_required_env_var() {
    cleanup_env_vars();
    Config::new(); // Should panic as DATABASE_URL is required and not set
}

#[rstest]
#[case("SERVER_PORT", "abc")]
#[case("SERVER_PORT", "1.5")]
#[case("SERVER_PORT", "-1")]
#[case("CLIENT_PORT", "abc")]
#[case("CLIENT_PORT", "-1")]
#[case("CLIENT_PORT", "1.5")]
#[serial]
fn test_parsing_fails(#[case] env_name: &str, #[case] invalid_value: &str) {
    setup_env_vars(true);
    env::set_var(env_name, invalid_value);

    let result = std::panic::catch_unwind(|| Config::new());

    assert!(result.is_err());
    cleanup_env_vars();
}

#[rstest]
#[case("SERVER_PORT", "8080")]
#[case("SERVER_PORT", "1234")]
#[case("CLIENT_PORT", "8080")]
#[case("CLIENT_PORT", "1234")]
#[case("SERVER_LOGGING_LEVEL", "INFO")]
#[case("SERVER_LOGGING_LEVEL", "ERROR")]
#[case("SERVER_LOGGING_LEVEL", "WARN")]
#[case("SERVER_LOGGING_LEVEL", "DEBUG")]
#[case("SERVER_LOGGING_LEVEL", "TRACE")]
#[serial]
fn test_parsing_succeeds(#[case] env_name: &str, #[case] invalid_value: &str) {
    setup_env_vars(true);
    env::set_var(env_name, invalid_value);

    let result = std::panic::catch_unwind(|| Config::new());

    assert!(result.is_ok());
    cleanup_env_vars();
}
