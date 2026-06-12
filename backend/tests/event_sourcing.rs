//! Unit tests for the event-sourcing core: validate, apply, replay, game
//! state, store.

use uuid::Uuid;

use deaths_door::apply::{apply, replay, validate};
use deaths_door::error::GameError;
use deaths_door::event_store::EventStore;
use deaths_door::events::{describe_event, EventPayload, GameEvent};
use deaths_door::game_manager::GameManager;
use deaths_door::game_state::{ChoppingBlock, GameState};

fn evt(state: &GameState, payload: EventPayload) -> GameEvent {
    GameEvent::new(state.game_id, state.version, payload)
}

/// Build a Trouble Brewing game with the given roles already included.
fn game_with_roles(roles: &[&str]) -> GameState {
    let game_id = Uuid::new_v4();
    let mut state = GameState::initial(game_id, "");
    state = apply(
        &state,
        &evt(
            &state,
            EventPayload::GameCreated {
                script_name: "trouble_brewing".to_string(),
            },
        ),
    );
    state = apply(
        &state,
        &evt(
            &state,
            EventPayload::RolesIncluded {
                names: roles.iter().map(|s| s.to_string()).collect(),
            },
        ),
    );
    state
}

fn add_player(state: &GameState, name: &str, role: &str, alignment: &str) -> GameState {
    apply(
        state,
        &evt(
            state,
            EventPayload::PlayerAdded {
                player_name: name.to_string(),
                character_name: role.to_string(),
                alignment: alignment.to_string(),
            },
        ),
    )
}

#[test]
fn game_created_sets_version_and_script() {
    let state = GameState::initial(Uuid::new_v4(), "");
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::GameCreated {
                script_name: "trouble_brewing".to_string(),
            },
        ),
    );
    assert_eq!(next.version, 1);
    assert_eq!(next.script_name, "trouble_brewing");
    assert_eq!(next.current_night_step, "Dusk");
    assert!(next.is_first_night);
}

#[test]
fn demon_bluffs_set_and_resolved() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::DemonBluffsSet {
                bluffs: vec!["Mayor".to_string(), "Slayer".to_string()],
            },
        ),
    );
    assert_eq!(next.version, state.version + 1);
    assert_eq!(
        next.demon_bluffs,
        vec!["Mayor".to_string(), "Slayer".to_string()]
    );
    // Resolves against the script to full Character objects.
    let resolved: Vec<&str> = next
        .get_demon_bluffs()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(resolved, ["Mayor", "Slayer"]);

    // Setting an empty list clears them.
    let cleared = apply(
        &next,
        &evt(&next, EventPayload::DemonBluffsSet { bluffs: vec![] }),
    );
    assert!(cleared.demon_bluffs.is_empty());
    assert!(cleared.get_demon_bluffs().is_empty());
}

#[test]
fn chopping_block_set_and_cleared() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");

    // Set with a vote count.
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::ChoppingBlockSet {
                player_name: "Alice".to_string(),
                votes: Some(4),
            },
        ),
    );
    assert_eq!(next.version, state.version + 1);
    assert_eq!(
        next.chopping_block,
        Some(ChoppingBlock {
            player_name: "Alice".to_string(),
            votes: Some(4),
        })
    );

    // Re-setting without a vote count replaces the block.
    let next = apply(
        &next,
        &evt(
            &next,
            EventPayload::ChoppingBlockSet {
                player_name: "Alice".to_string(),
                votes: None,
            },
        ),
    );
    assert_eq!(next.chopping_block.as_ref().unwrap().votes, None);

    // Explicit clear.
    let cleared = apply(&next, &evt(&next, EventPayload::ChoppingBlockCleared));
    assert_eq!(cleared.chopping_block, None);
}

#[test]
fn chopping_block_clears_when_its_player_dies_or_leaves() {
    let state = game_with_roles(&["Imp", "Chef", "Mayor"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");
    let on_block = apply(
        &state,
        &evt(
            &state,
            EventPayload::ChoppingBlockSet {
                player_name: "Alice".to_string(),
                votes: None,
            },
        ),
    );

    // Killing a different player leaves the block alone.
    let bob_dead = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::PlayerAliveSet {
                player_name: "Bob".to_string(),
                is_alive: false,
                cleared_effects: vec![],
            },
        ),
    );
    assert!(bob_dead.chopping_block.is_some());

    // Killing the player on the block clears it.
    let alice_dead = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::PlayerAliveSet {
                player_name: "Alice".to_string(),
                is_alive: false,
                cleared_effects: vec![],
            },
        ),
    );
    assert_eq!(alice_dead.chopping_block, None);

    // Removing the player on the block clears it.
    let alice_removed = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::PlayerRemoved {
                player_name: "Alice".to_string(),
            },
        ),
    );
    assert_eq!(alice_removed.chopping_block, None);

    // Renaming the player on the block follows the rename.
    let alice_renamed = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::PlayerRenamed {
                old_name: "Alice".to_string(),
                new_name: "Alicia".to_string(),
            },
        ),
    );
    assert_eq!(alice_renamed.chopping_block.unwrap().player_name, "Alicia");
}

#[test]
fn chopping_block_clears_when_night_begins() {
    let state = game_with_roles(&["Imp"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let on_block = apply(
        &state,
        &evt(
            &state,
            EventPayload::ChoppingBlockSet {
                player_name: "Alice".to_string(),
                votes: Some(3),
            },
        ),
    );

    // Re-bookmarking "Dawn" (still day) keeps the block.
    let still_day = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::NightStepSet {
                step: "Dawn".to_string(),
            },
        ),
    );
    assert!(still_day.chopping_block.is_some());

    // Stepping into the night clears it.
    let night = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::NightStepSet {
                step: "Dusk".to_string(),
            },
        ),
    );
    assert_eq!(night.chopping_block, None);

    // Toggling the first-night flag (resets to Dusk) clears it too.
    let night = apply(
        &on_block,
        &evt(
            &on_block,
            EventPayload::FirstNightSet {
                is_first_night: false,
            },
        ),
    );
    assert_eq!(night.chopping_block, None);
}

#[test]
fn player_added_consumes_role_from_pool() {
    let state = game_with_roles(&["Imp", "Chef"]);
    assert_eq!(state.included_role_names.len(), 2);
    let next = add_player(&state, "Alice", "Imp", "evil");
    assert_eq!(next.players.len(), 1);
    assert_eq!(next.players[0].name, "Alice");
    assert_eq!(next.included_role_names, vec!["Chef".to_string()]);
}

#[test]
fn player_removed_returns_role_to_pool() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerRemoved {
                player_name: "Alice".to_string(),
            },
        ),
    );
    assert_eq!(next.players.len(), 0);
    assert!(next.included_role_names.contains(&"Imp".to_string()));
}

#[test]
fn living_count_and_execution_threshold() {
    let state = game_with_roles(&["Imp", "Chef", "Empath", "Mayor", "Monk"]);
    let mut state = state;
    for (n, r) in [
        ("A", "Imp"),
        ("B", "Chef"),
        ("C", "Empath"),
        ("D", "Mayor"),
        ("E", "Monk"),
    ] {
        state = add_player(&state, n, r, "good");
    }
    assert_eq!(state.living_player_count(), 5);
    // ceil(5/2) = 3
    assert_eq!(state.execution_threshold(), 3);

    // Kill one -> 4 living, ceil(4/2) = 2
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerAliveSet {
                player_name: "A".to_string(),
                is_alive: false,
                cleared_effects: vec![],
            },
        ),
    );
    assert_eq!(state.living_player_count(), 4);
    assert_eq!(state.execution_threshold(), 2);
}

#[test]
fn status_effects_add_and_remove_are_idempotent() {
    let state = game_with_roles(&["Imp"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::StatusEffectAdded {
                player_name: "Alice".to_string(),
                effect: "Poisoned".to_string(),
            },
        ),
    );
    // Adding again does not duplicate.
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::StatusEffectAdded {
                player_name: "Alice".to_string(),
                effect: "Poisoned".to_string(),
            },
        ),
    );
    assert_eq!(
        state.get_player("Alice").unwrap().status_effects,
        vec!["Poisoned"]
    );

    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::StatusEffectRemoved {
                player_name: "Alice".to_string(),
                effect: "Poisoned".to_string(),
            },
        ),
    );
    assert!(state.get_player("Alice").unwrap().status_effects.is_empty());
}

#[test]
fn player_alive_set_clears_cascading_effects() {
    let state = game_with_roles(&["Poisoner", "Chef"]);
    let state = add_player(&state, "Pat", "Poisoner", "evil");
    let state = add_player(&state, "Cara", "Chef", "good");
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::StatusEffectAdded {
                player_name: "Cara".to_string(),
                effect: "Poisoned".to_string(),
            },
        ),
    );
    // Killing the poisoner clears the Poisoned effect from Cara.
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerAliveSet {
                player_name: "Pat".to_string(),
                is_alive: false,
                cleared_effects: vec![("Cara".to_string(), "Poisoned".to_string())],
            },
        ),
    );
    assert!(state.get_player("Cara").unwrap().status_effects.is_empty());
    assert!(!state.get_player("Pat").unwrap().is_alive);
}

#[test]
fn characters_swapped() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::CharactersSwapped {
                name1: "Alice".to_string(),
                name2: "Bob".to_string(),
            },
        ),
    );
    assert_eq!(state.get_player("Alice").unwrap().character_name, "Chef");
    assert_eq!(state.get_player("Bob").unwrap().character_name, "Imp");
}

#[test]
fn dead_players_with_vote() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerAliveSet {
                player_name: "Alice".to_string(),
                is_alive: false,
                cleared_effects: vec![],
            },
        ),
    );
    assert_eq!(
        state.get_dead_players_with_vote(),
        vec!["Alice".to_string()]
    );
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::DeadVoteUsedSet {
                player_name: "Alice".to_string(),
                has_used_dead_vote: true,
            },
        ),
    );
    assert!(state.get_dead_players_with_vote().is_empty());
}

#[test]
fn night_steps_filter_by_living_characters() {
    // Poisoner present and alive -> Poisoner step shows on the first night.
    let state = game_with_roles(&["Poisoner", "Imp"]);
    let state = add_player(&state, "Pat", "Poisoner", "evil");
    let steps = state.get_night_steps();
    let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Dusk")); // always_show
    assert!(names.contains(&"Poisoner")); // living character
    assert!(!names.contains(&"Monk")); // not in play
}

#[test]
fn replay_reconstructs_state() {
    let game_id = Uuid::new_v4();
    let mut events = Vec::new();
    let mut state = GameState::initial(game_id, "");

    for payload in [
        EventPayload::GameCreated {
            script_name: "trouble_brewing".to_string(),
        },
        EventPayload::RolesIncluded {
            names: vec!["Imp".to_string(), "Chef".to_string()],
        },
        EventPayload::PlayerAdded {
            player_name: "Alice".to_string(),
            character_name: "Imp".to_string(),
            alignment: "evil".to_string(),
        },
    ] {
        let e = GameEvent::new(game_id, state.version, payload);
        state = apply(&state, &e);
        events.push(e);
    }

    let replayed = replay(&events).unwrap();
    assert_eq!(replayed, state);
    assert_eq!(replayed.version, 3);
    assert_eq!(replayed.players.len(), 1);
}

#[test]
fn replay_empty_is_error() {
    assert!(replay(&[]).is_err());
}

#[test]
fn event_payload_json_uses_type_tag() {
    let payload = EventPayload::PlayerAdded {
        player_name: "Alice".to_string(),
        character_name: "Imp".to_string(),
        alignment: "evil".to_string(),
    };
    let v = serde_json::to_value(&payload).unwrap();
    assert_eq!(v["type"], "player_added");
    assert_eq!(v["player_name"], "Alice");
    assert_eq!(v["character_name"], "Imp");

    // Round-trips back to the same value.
    let back: EventPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back, payload);
}

#[test]
fn describe_event_is_human_readable() {
    let p = EventPayload::PlayerAdded {
        player_name: "Ryan".to_string(),
        character_name: "Baron".to_string(),
        alignment: "evil".to_string(),
    };
    assert_eq!(describe_event(&p), "Ryan joined as Baron");
    let p = EventPayload::PlayerAliveSet {
        player_name: "Yash".to_string(),
        is_alive: false,
        cleared_effects: vec![],
    };
    assert_eq!(describe_event(&p), "Yash died");
}

// --- Dispatch-time validation ---

fn player_added(name: &str, role: &str) -> EventPayload {
    EventPayload::PlayerAdded {
        player_name: name.to_string(),
        character_name: role.to_string(),
        alignment: "evil".to_string(),
    }
}

#[test]
fn validate_rejects_duplicate_player_and_drawn_role() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");

    // Same name again -> Conflict, even with a free role.
    assert!(matches!(
        validate(&state, &player_added("Alice", "Chef")),
        Err(GameError::Conflict(_))
    ));
    // Imp was drawn from the pool when Alice joined -> InvalidInput.
    assert!(matches!(
        validate(&state, &player_added("Bob", "Imp")),
        Err(GameError::InvalidInput(_))
    ));
    // A fresh name and a pooled role is fine.
    assert!(validate(&state, &player_added("Bob", "Chef")).is_ok());
}

#[test]
fn validate_rejects_chopping_block_for_dead_or_unknown_player() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");

    let block = EventPayload::ChoppingBlockSet {
        player_name: "Alice".to_string(),
        votes: None,
    };
    assert!(validate(&state, &block).is_ok());

    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerAliveSet {
                player_name: "Alice".to_string(),
                is_alive: false,
                cleared_effects: vec![],
            },
        ),
    );
    assert!(matches!(
        validate(&state, &block),
        Err(GameError::InvalidInput(_))
    ));
    assert!(matches!(
        validate(
            &state,
            &EventPayload::ChoppingBlockSet {
                player_name: "Ghost".to_string(),
                votes: None,
            }
        ),
        Err(GameError::NotFound(_))
    ));
}

#[test]
fn validate_rejects_rename_collision() {
    let state = game_with_roles(&["Imp", "Chef"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");

    assert!(matches!(
        validate(
            &state,
            &EventPayload::PlayerRenamed {
                old_name: "Bob".to_string(),
                new_name: "Alice".to_string(),
            }
        ),
        Err(GameError::Conflict(_))
    ));
    // Renaming a player to their current name is a no-op, not a collision.
    assert!(validate(
        &state,
        &EventPayload::PlayerRenamed {
            old_name: "Bob".to_string(),
            new_name: "Bob".to_string(),
        }
    )
    .is_ok());
}

#[test]
fn validate_rejects_blank_or_misaddressed_status_effects() {
    let state = game_with_roles(&["Imp"]);
    let state = add_player(&state, "Alice", "Imp", "evil");

    assert!(matches!(
        validate(
            &state,
            &EventPayload::StatusEffectAdded {
                player_name: "Alice".to_string(),
                effect: "   ".to_string(),
            }
        ),
        Err(GameError::InvalidInput(_))
    ));
    assert!(matches!(
        validate(
            &state,
            &EventPayload::StatusEffectAdded {
                player_name: "Ghost".to_string(),
                effect: "Poisoned".to_string(),
            }
        ),
        Err(GameError::NotFound(_))
    ));
}

#[tokio::test]
async fn dispatch_validates_and_rejected_events_leave_no_trace() {
    let manager = GameManager::new(EventStore::in_memory().unwrap());
    manager.create_game("trouble_brewing").await.unwrap();
    manager
        .dispatch(EventPayload::RolesIncluded {
            names: vec!["Imp".to_string()],
        })
        .await
        .unwrap();
    manager
        .dispatch(player_added("Alice", "Imp"))
        .await
        .unwrap();

    // Duplicate name -> Conflict; role already drawn -> InvalidInput.
    assert!(matches!(
        manager.dispatch(player_added("Alice", "Imp")).await,
        Err(GameError::Conflict(_))
    ));
    assert!(matches!(
        manager.dispatch(player_added("Bob", "Imp")).await,
        Err(GameError::InvalidInput(_))
    ));

    // Rejected events must not mutate state or be persisted.
    let state = manager.state().await.unwrap();
    assert_eq!(state.players.len(), 1);
    let history = manager.get_history().await.unwrap();
    assert_eq!(history.len() as i64, state.version);
}

// --- Event store ---

#[test]
fn store_append_and_load_round_trip() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    let mut state = GameState::initial(game_id, "");

    for payload in [
        EventPayload::GameCreated {
            script_name: "trouble_brewing".to_string(),
        },
        EventPayload::RoleIncluded {
            name: "Imp".to_string(),
        },
    ] {
        let e = GameEvent::new(game_id, state.version, payload);
        state = apply(&state, &e);
        store.append(&e).unwrap();
    }

    let loaded = store.get_events(game_id, None).unwrap();
    assert_eq!(loaded.len(), 2);
    let rebuilt = replay(&loaded).unwrap();
    assert_eq!(rebuilt.script_name, "trouble_brewing");
    assert_eq!(rebuilt.included_role_names, vec!["Imp".to_string()]);
}

#[test]
fn store_delete_after_sequence_rewinds() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    let mut state = GameState::initial(game_id, "");
    for (i, payload) in [
        EventPayload::GameCreated {
            script_name: "trouble_brewing".to_string(),
        },
        EventPayload::RoleIncluded {
            name: "Imp".to_string(),
        },
        EventPayload::RoleIncluded {
            name: "Chef".to_string(),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let e = GameEvent::new(game_id, i as i64, payload);
        state = apply(&state, &e);
        store.append(&e).unwrap();
    }
    assert_eq!(store.get_latest_sequence(game_id).unwrap(), 2);

    // Delete everything after sequence 0 (keep only GameCreated).
    let deleted = store.delete_after_sequence(game_id, 0).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(store.get_events(game_id, None).unwrap().len(), 1);
}

#[test]
fn store_fork_copies_events_to_new_game() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    let mut state = GameState::initial(game_id, "");
    for (i, payload) in [
        EventPayload::GameCreated {
            script_name: "trouble_brewing".to_string(),
        },
        EventPayload::RoleIncluded {
            name: "Imp".to_string(),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let e = GameEvent::new(game_id, i as i64, payload);
        state = apply(&state, &e);
        store.append(&e).unwrap();
    }
    let _ = state;

    let new_id = store.fork_game(game_id, 1).unwrap();
    assert_ne!(new_id, game_id);
    let forked = store.get_events(new_id, None).unwrap();
    assert_eq!(forked.len(), 2);

    let ids = store.get_all_game_ids().unwrap();
    assert_eq!(ids.len(), 2);
}

#[test]
fn store_most_recent_game_id_follows_insertion_order() {
    let store = EventStore::in_memory().unwrap();
    assert_eq!(store.get_most_recent_game_id().unwrap(), None);

    // Fixed IDs where the game appended LAST sorts FIRST by UUID string, so
    // ordering by game_id (the old resume behavior) would pick the wrong one.
    // Resume must follow insertion order.
    let older = Uuid::from_u128(u128::MAX); // lexicographically largest
    let newer = Uuid::from_u128(1); // lexicographically smallest
    for game_id in [older, newer] {
        let e = GameEvent::new(
            game_id,
            0,
            EventPayload::GameCreated {
                script_name: "trouble_brewing".to_string(),
            },
        );
        store.append(&e).unwrap();
    }

    assert_eq!(store.get_most_recent_game_id().unwrap(), Some(newer));
}
