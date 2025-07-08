use chrono::SubsecRound;
use uuid::Uuid;
use webauthn_rs::{
    Webauthn, WebauthnBuilder,
    prelude::{Passkey, Url},
};

use super::SqliteClient;
use crate::{
    db::interface::DatabaseClient,
    models::{
        NewPasskeyCredential, PasskeyAuthenticationState, PasskeyAuthenticationStateType,
        PasskeyCredentialUpdate, PasskeyRegistrationState, Session, SessionState, SessionUpdate,
        UserCreate, ViaJson,
    },
};

struct Tools {
    client: SqliteClient,
    webauthn: Webauthn,
}

/// Create a new set of tools/clients for a test.
async fn tools() -> Tools {
    // Enable debug logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

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
        registration: ViaJson(reg),
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
        registration: ViaJson(reg),
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
    let session_id: u64 = 123_456_789;
    let session = Session {
        user_id: *user.id(),
        id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        is_admin: false,
        parent_id_hash: None,
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
    let session_id: u64 = 123_456_789;
    let session = Session {
        user_id: *user.id(),
        id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        is_admin: false,
        parent_id_hash: None,
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
    let passkey: Passkey =
        serde_json::from_str(include_str!("tests/resources/passkey.json")).unwrap();
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

#[tokio::test]
async fn test_non_discoverable_passkey_authentication() {
    let Tools { client, webauthn } = tools().await;

    // Create user so the email exists
    client
        .create_user(
            &Uuid::new_v4(),
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .unwrap();

    // Create passkey data
    let passkey: Passkey =
        serde_json::from_str(include_str!("tests/resources/passkey.json")).unwrap();
    let (_, auth_state) = webauthn.start_passkey_authentication(&[passkey]).unwrap();
    let state = PasskeyAuthenticationState {
        id: Uuid::new_v4(),
        email: Some("test@kasad.com".to_string()),
        state: ViaJson(PasskeyAuthenticationStateType::Regular(auth_state)),
        created_at: chrono::Utc::now(),
    };

    // Test create
    client.create_passkey_authentication(&state).await.unwrap();

    // Test get
    let got_state = client
        .get_passkey_authentication_by_id(&state.id)
        .await
        .unwrap();
    assert_eq!(got_state.id, state.id);
    assert_eq!(got_state.email, state.email);
    assert!(matches!(
        got_state.state.0,
        PasskeyAuthenticationStateType::Regular(_)
    ));
}

#[tokio::test]
async fn test_discoverable_passkey_authentication() {
    let Tools { client, webauthn } = tools().await;

    // Create user so the email exists
    client
        .create_user(
            &Uuid::new_v4(),
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .unwrap();

    // Create passkey data
    let (_, disco_state) = webauthn.start_discoverable_authentication().unwrap();
    let state = PasskeyAuthenticationState {
        id: Uuid::new_v4(),
        email: Some("test@kasad.com".to_string()),
        state: ViaJson(PasskeyAuthenticationStateType::Discoverable(disco_state)),
        created_at: chrono::Utc::now(),
    };

    // Test create
    client.create_passkey_authentication(&state).await.unwrap();

    // Test get
    let got_state = client
        .get_passkey_authentication_by_id(&state.id)
        .await
        .unwrap();
    assert_eq!(got_state.id, state.id);
    assert_eq!(got_state.email, state.email);
    assert!(matches!(
        got_state.state.0,
        PasskeyAuthenticationStateType::Discoverable(_)
    ));
}

#[tokio::test]
async fn test_update_passkey() {
    let Tools { client, .. } = tools().await;

    // Load passkeys from JSON files
    let passkey: Passkey =
        serde_json::from_str(include_str!("tests/resources/passkey.json")).unwrap();
    let passkey_incremented: Passkey =
        serde_json::from_str(include_str!("tests/resources/passkey-incremented.json")).unwrap();

    // Create user for foreign key constraints
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

    // Create passkey
    let pkid = Uuid::new_v4();
    client
        .create_passkey(
            &pkid,
            &user_id,
            &NewPasskeyCredential {
                display_name: None,
                passkey,
            },
        )
        .await
        .unwrap();

    // Update display name
    let update = PasskeyCredentialUpdate::new().with_display_name(Some("My passkey"));
    client.update_passkey(&pkid, &update).await.unwrap();

    // Get updated passkey and ensure display name is updated
    let passkey = client.get_passkey_by_id(&pkid).await.unwrap();
    assert_eq!(passkey.display_name, Some("My passkey".to_string()));

    // Update passkey
    let update = PasskeyCredentialUpdate::new().with_passkey(passkey_incremented.clone());
    client.update_passkey(&pkid, &update).await.unwrap();

    // Get updated passkey and ensure counter is incremented
    let passkey = client.get_passkey_by_id(&pkid).await.unwrap();
    assert_eq!(passkey.passkey.0, passkey_incremented);
}

#[tokio::test]
async fn test_update_session() {
    let Tools { client, .. } = tools().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = client
        .create_user(
            &user_id,
            &UserCreate {
                email: "test@kasad.com".to_string(),
                display_name: "Test User".to_string(),
            },
        )
        .await
        .unwrap();

    // Create session
    let session_id: u64 = 123_456_789;
    let session = Session {
        user_id: *user.id(),
        id_hash: blake3::hash(&session_id.to_le_bytes()).into(),
        state: SessionState::Active,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        is_admin: false,
        parent_id_hash: None,
    };
    client.create_session(&session).await.unwrap();

    // Update state
    let update = SessionUpdate::new().with_state(SessionState::LoggedOut);
    let session = client
        .update_session(&session.id_hash, &update)
        .await
        .unwrap();
    assert_eq!(session.state, SessionState::LoggedOut);

    // Update expires_at
    let new_expires_at = chrono::Utc::now() + chrono::Duration::days(2);
    let update = SessionUpdate::new().with_expires_at(new_expires_at);
    let session = client
        .update_session(&session.id_hash, &update)
        .await
        .unwrap();
    assert_eq!(session.expires_at, new_expires_at.trunc_subsecs(0));
}
