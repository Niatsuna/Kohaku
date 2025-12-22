use std::time::Duration;

use chrono::Utc;
use rstest::rstest;

use crate::utils::comm::{
    auth::{sign_message, verify_message},
    ws::RateLimiter,
    MessageType, WsMessage,
};
/*
  This file features unit tests of the underlying subprocesses like
  rate limiting and the authentication functions.

  For the actual connection, testing can be found as integration tests under server/tests/!
*/

// Unit Test for RateLimiter
#[tokio::test]
async fn test_rate_limiter_allows_messages_within_limit() {
    let mut limiter = RateLimiter::new(3, 10);

    assert!(limiter.check_and_add());
    assert!(limiter.check_and_add());
    assert!(limiter.check_and_add());
    assert!(!limiter.check_and_add());
}

#[tokio::test]
async fn test_rate_limiter_resets_after_window() {
    let mut limiter = RateLimiter::new(2, 1);

    assert!(limiter.check_and_add());
    assert!(limiter.check_and_add());
    assert!(!limiter.check_and_add());

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert!(limiter.check_and_add());
}

// Unit Test for Authentication

fn create_base_message() -> WsMessage {
    let timestamp = 1234567890;
    let message_id = "fixed-id-123".to_string();
    let id = "test-ping".to_string();

    WsMessage {
        timestamp,
        message_id,
        message: MessageType::Ping { id },
    }
}

/// Tests if two messages with the exact same data result in the exact same signature
#[tokio::test]
async fn test_message_signature_same_if_same_data() {
    let secret = "test-secret".to_string().into_bytes();

    let message1 = create_base_message();
    let message2 = create_base_message();

    let signed1 = sign_message(&message1, &secret);
    let signed2 = sign_message(&message2, &secret);
    assert_eq!(signed1, signed2);
}

/// Tests if two messages with not the exact same data result in two different sinatures
#[rstest]
#[case::different_timestamp(
  WsMessage {
    timestamp: 0987654321,
    message_id: "fixed-id-123".to_string(),
    message: MessageType::Ping { id: "test-ping".to_string() },
  }
)]
#[case::different_message_id(
  WsMessage {
    timestamp: 1234567890,
    message_id: "different-id-456".to_string(),
    message: MessageType::Ping { id: "test-ping".to_string() },
  }
)]
#[case::different_message_content(
  WsMessage {
    timestamp: 1234567890,
    message_id: "fixed-id-123".to_string(),
    message: MessageType::Ping { id: "different-ping".to_string() },
  }
)]
#[case::different_message_type(
  WsMessage {
    timestamp: 1234567890,
    message_id: "fixed-id-123".to_string(),
    message: MessageType::Pong { id: "test-ping".to_string() },
  }
)]
fn test_message_signature_unique(#[case] message2: WsMessage) {
    let secret = "test-secret".to_string().into_bytes();

    let message1 = create_base_message();

    let signed1 = sign_message(&message1, &secret);
    let signed2 = sign_message(&message2, &secret);
    assert_ne!(signed1, signed2);
}

/// Tests if a decoded verified message (signature -> WsMessage) holds the correct content
#[tokio::test]
async fn test_message_verification_valid_signature() {
    let secret = "test-secret".to_string().into_bytes();

    let msg = WsMessage {
        timestamp: Utc::now().timestamp(),
        message_id: "fixed-id-123".to_string(),
        message: MessageType::Pong {
            id: "test-ping".to_string(),
        },
    };

    let signed = sign_message(&msg, &secret);
    let verified = verify_message(&signed, &secret);

    //#1 Check if verification was successful (Should as we used the same secret)
    assert!(verified.is_ok(), "Invalid message detected");

    let verified_message = verified.unwrap();

    //#2 Check if content is the same
    let expected_ts = msg.timestamp;
    let expected_message_id = msg.message_id;
    let expected_message = msg.message;

    assert_eq!(
        verified_message.timestamp, expected_ts,
        "Timestamp differs from expected: Expected {} but was {}",
        expected_ts, verified_message.timestamp
    );
    assert_eq!(verified_message.message_id, expected_message_id);
    assert_eq!(verified_message.message, expected_message);
}

#[tokio::test]
async fn test_message_verification_invalid_signature() {
    let secret = "test-secret".to_string().into_bytes();

    let msg = WsMessage {
        timestamp: Utc::now().timestamp(),
        message_id: "fixed-id-123".to_string(),
        message: MessageType::Pong {
            id: "test-ping".to_string(),
        },
    };

    let signed = sign_message(&msg, &secret);

    // Invalify signature
    let invalid_signed = signed + "a";

    let verified = verify_message(&invalid_signed, &secret);

    //#1 Check if verification has failed
    assert!(verified.is_err(), "Valid message detected");
}
