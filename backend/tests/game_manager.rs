//! Tests for `GameManager`: dispatch, boot-time resume, rewind/fork
//! validation, and the SSE broadcast channel.

use deaths_door::event_store::EventStore;
use deaths_door::events::{EventPayload, GameEvent};
use deaths_door::game_manager::GameManager;
use uuid::Uuid;

fn manager() -> GameManager {
    GameManager::new(EventStore::in_memory().expect("in-memory store"))
}

/// Seed a store with a created game and return its id.
fn seed_game(store: &EventStore, roles: &[&str]) -> Uuid {
    let game_id = Uuid::new_v4();
    let mut events = vec![EventPayload::GameCreated {
        script_name: "trouble_brewing".to_string(),
    }];
    events.extend(roles.iter().map(|r| EventPayload::RoleIncluded {
        name: r.to_string(),
    }));
    for (i, payload) in events.into_iter().enumerate() {
        store
            .append(&GameEvent::new(game_id, i as i64, payload))
            .unwrap();
    }
    game_id
}

#[tokio::test]
async fn operations_require_an_active_game() {
    let m = manager();
    assert!(!m.has_active_game().await);
    assert!(m.state().await.is_err());
    assert!(m
        .dispatch(EventPayload::RoleIncluded {
            name: "Imp".to_string(),
        })
        .await
        .is_err());
    assert!(m.rewind(1).await.is_err());
    assert!(m.fork(1).await.is_err());
    assert!(m.get_history().await.is_err());
}

#[tokio::test]
async fn get_state_creates_a_game_when_the_store_is_empty() {
    let m = manager();
    let state = m.get_state().await.unwrap();
    assert_eq!(state.script_name, "trouble_brewing");
    assert_eq!(state.version, 1);
    assert!(state.players.is_empty());
    assert!(m.has_active_game().await);
}

#[tokio::test]
async fn get_state_resumes_the_most_recently_played_game() {
    let store = EventStore::in_memory().unwrap();
    let _older = seed_game(&store, &["Chef"]);
    let newer = seed_game(&store, &["Imp", "Baron"]);

    let m = GameManager::new(store);
    let state = m.get_state().await.unwrap();
    assert_eq!(state.game_id, newer);
    assert_eq!(state.included_role_names, vec!["Imp", "Baron"]);

    // Dispatch keeps working from the replayed version (sequence numbers must
    // continue where the log left off, or the store's UNIQUE constraint trips).
    let next = m
        .dispatch(EventPayload::RoleIncluded {
            name: "Mayor".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(next.version, state.version + 1);
}

#[tokio::test]
async fn load_game_unknown_id_is_an_error() {
    let m = manager();
    assert!(m.load_game(Uuid::new_v4()).await.is_err());
}

#[tokio::test]
async fn rewind_validates_versions_and_dispatch_still_works_after() {
    let m = manager();
    m.create_game("trouble_brewing").await.unwrap();
    m.dispatch(EventPayload::RoleIncluded {
        name: "Imp".to_string(),
    })
    .await
    .unwrap();
    m.dispatch(EventPayload::RoleIncluded {
        name: "Chef".to_string(),
    })
    .await
    .unwrap();

    // Version must be 1..=current.
    assert!(m.rewind(0).await.is_err());
    assert!(m.rewind(4).await.is_err());

    let state = m.rewind(2).await.unwrap();
    assert_eq!(state.version, 2);
    assert_eq!(state.included_role_names, vec!["Imp"]);

    // Appending after a rewind reuses the freed sequence numbers cleanly.
    let next = m
        .dispatch(EventPayload::RoleIncluded {
            name: "Baron".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(next.version, 3);
    assert_eq!(next.included_role_names, vec!["Imp", "Baron"]);
}

#[tokio::test]
async fn fork_leaves_the_original_game_untouched() {
    let m = manager();
    let original = m.create_game("trouble_brewing").await.unwrap();
    m.dispatch(EventPayload::RoleIncluded {
        name: "Imp".to_string(),
    })
    .await
    .unwrap();

    let fork = m.fork(1).await.unwrap();
    assert_ne!(fork.game_id, original.game_id);
    assert_eq!(fork.version, 1);
    assert!(fork.included_role_names.is_empty());

    // Mutating the fork doesn't affect the original timeline.
    m.dispatch(EventPayload::RoleIncluded {
        name: "Baron".to_string(),
    })
    .await
    .unwrap();
    let original_reloaded = m.load_game(original.game_id).await.unwrap();
    assert_eq!(original_reloaded.included_role_names, vec!["Imp"]);
    assert_eq!(original_reloaded.version, 2);
}

#[tokio::test]
async fn dispatch_broadcasts_the_new_state_to_subscribers() {
    let m = manager();
    m.create_game("trouble_brewing").await.unwrap();

    let mut rx = m.subscribe();
    let dispatched = m
        .dispatch(EventPayload::RoleIncluded {
            name: "Imp".to_string(),
        })
        .await
        .unwrap();

    let broadcast = rx.recv().await.unwrap();
    assert_eq!(broadcast, dispatched);
}
