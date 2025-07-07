use uuid::Uuid;
use webauthn_rs::{
    Webauthn, WebauthnBuilder,
    prelude::{Passkey, Url},
};

use super::SqliteClient;
use crate::{
    db::interface::DatabaseClient,
    models::{NewPasskeyCredential, PasskeyRegistrationState, Session, SessionState, UserCreate},
};

struct Tools {
    client: SqliteClient,
    webauthn: Webauthn,
}

/// Create a new set of tools/clients for a test.
async fn tools() -> Tools {
    Tools {
        client: SqliteClient::new_memory()
            .await
            .expect("expected client creation to succeed"),
        webauthn: WebauthnBuilder::new("example.org", &Url::parse("http://example.org").unwrap())
            .expect("expected webauthn builder creation to succeed")
            .build()
            .expect("expected webauthn creation to succeed"),
    }
}

#[tokio::test]
async fn test_create_user() {
    let Tools { client, .. } = tools().await;
    let user = client
        .create_user(
            &Uuid::new_v4(),
            &UserCreate {
                email: "test@example.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .expect("expected user creation to succeed");
    assert_eq!(user.email(), "test@example.com");
    assert_eq!(user.display_name(), "Test User");
}

#[tokio::test]
async fn test_create_passkey_registration() {
    let Tools { client, webauthn } = tools().await;
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let display_name = "Test User";
    let (_, reg) = webauthn
        .start_passkey_registration(user_id, email, display_name, None)
        .unwrap();
    let registration = PasskeyRegistrationState {
        id: Uuid::new_v4(),
        user_id,
        email: email.to_string(),
        registration: sqlx::types::Json(reg),
        created_at: chrono::Utc::now(),
    };
    client
        .create_passkey_registration(&registration)
        .await
        .expect("expected create passkey registration to succeed");
}

#[tokio::test]
async fn test_get_passkey_registration_by_id() {
    let Tools { client, webauthn } = tools().await;
    // Set up: create a passkey registration
    let email = "test@kasad.com";
    let display_name = "Test User";
    let id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (_, reg) = webauthn
        .start_passkey_registration(user_id, email, display_name, None)
        .unwrap();
    let registration = PasskeyRegistrationState {
        id,
        user_id,
        email: email.to_string(),
        registration: sqlx::types::Json(reg),
        created_at: chrono::Utc::now(),
    };
    client
        .create_passkey_registration(&registration)
        .await
        .unwrap();

    // Test: get the passkey registration by id
    let registration = client.get_passkey_registration_by_id(&id).await.unwrap();
    assert_eq!(registration.user_id, user_id);
    assert_eq!(registration.email, email);
}

#[tokio::test]
async fn test_create_session() {
    let Tools { client, .. } = tools().await;

    // Set up: create a user
    let user = client
        .create_user(
            &Uuid::new_v4(),
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .expect("expected user creation to succeed");

    // Test: create session
    let session_id: u64 = 123456789;
    let session = Session {
        user_id: *user.id(),
        id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(1),
    };
    client.create_session(&session).await.unwrap();
}

#[tokio::test]
async fn test_get_session_by_id_hash() {
    let Tools { client, .. } = tools().await;

    // Set up: create a user
    let user = client
        .create_user(
            &Uuid::new_v4(),
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .expect("expected user creation to succeed");

    // Set up: create session
    let session_id: u64 = 123456789;
    let session = Session {
        user_id: *user.id(),
        id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(1),
    };
    client.create_session(&session).await.unwrap();

    // Test: get session by id hash
    let session = client
        .get_session_by_id_hash(&session.id_hash)
        .await
        .unwrap();
    assert_eq!(session.user_id, *user.id());
    assert_eq!(session.id_hash.0, session.id_hash.0);
    assert_eq!(session.state, SessionState::Active);
    assert_eq!(session.created_at, session.created_at);
    assert_eq!(session.expires_at, session.expires_at);
}

#[tokio::test]
async fn test_create_passkey() {
    let Tools { client, .. } = tools().await;
    let user_id = Uuid::new_v4();
    client
        .create_user(
            &user_id,
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .unwrap();
    let passkey: Passkey = serde_json::from_str(include_str!("resources/passkey.json")).unwrap();
    client
        .create_passkey(
            &Uuid::new_v4(),
            &user_id,
            &NewPasskeyCredential {
                display_name: None,
                passkey,
            },
        )
        .await
        .unwrap();
}
