//! Unit tests for the event-sourcing core: apply, replay, game state, store.

use uuid::Uuid;

use deaths_door::apply::{apply, replay};
use deaths_door::event_store::EventStore;
use deaths_door::events::{describe_event, EventPayload, GameEvent};
use deaths_door::game_state::{ChoppingBlock, GameState, NominationOutcome, Phase};

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
fn role_removed_is_case_insensitive_and_removes_one_copy() {
    let state = game_with_roles(&["Imp", "Imp", "Chef"]);
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::RoleRemoved {
                name: "  imp ".to_string(),
            },
        ),
    );
    assert_eq!(next.included_role_names, vec!["Imp", "Chef"]);
}

#[test]
fn traveler_added_does_not_consume_the_role_pool() {
    let state = game_with_roles(&["Imp"]);
    let next = apply(
        &state,
        &evt(
            &state,
            EventPayload::TravelerAdded {
                player_name: "Wanderer".to_string(),
                traveler_name: "Beggar".to_string(),
                alignment: "good".to_string(),
            },
        ),
    );
    assert_eq!(next.players.len(), 1);
    assert_eq!(next.players[0].character_name, "Beggar");
    // Travelers are chosen directly, never drawn from the pool.
    assert_eq!(next.included_role_names, vec!["Imp"]);
}

/// Replay must never panic on events referencing players that no longer match
/// (e.g. hand-edited logs); they apply as version-bumping no-ops.
#[test]
fn events_for_unknown_players_are_safe_noops() {
    let state = game_with_roles(&["Imp"]);
    let state = add_player(&state, "Alice", "Imp", "evil");

    let payloads = [
        EventPayload::PlayerRemoved {
            player_name: "Ghost".to_string(),
        },
        EventPayload::PlayerRenamed {
            old_name: "Ghost".to_string(),
            new_name: "Spectre".to_string(),
        },
        EventPayload::CharactersSwapped {
            name1: "Alice".to_string(),
            name2: "Ghost".to_string(),
        },
        EventPayload::StatusEffectAdded {
            player_name: "Ghost".to_string(),
            effect: "Poisoned".to_string(),
        },
        EventPayload::DeadVoteUsedSet {
            player_name: "Ghost".to_string(),
            has_used_dead_vote: true,
        },
    ];
    for payload in payloads {
        let next = apply(&state, &evt(&state, payload.clone()));
        assert_eq!(next.version, state.version + 1, "{payload:?}");
        assert_eq!(next.players, state.players, "{payload:?}");
    }
}

#[test]
fn living_count_and_execution_threshold() {
    let state = game_with_roles(&["Imp", "Chef", "Empath", "Mayor", "Monk"]);
    // No players yet: nothing living, nothing needed to execute.
    assert_eq!(state.living_player_count(), 0);
    assert_eq!(state.execution_threshold(), 0);

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
fn night_steps_show_when_dead_keeps_step_after_death() {
    // The Ravenkeeper's other-night step is flagged show_when_dead; the
    // Poisoner's is not. Kill both and only the Ravenkeeper's step survives.
    let state = game_with_roles(&["Ravenkeeper", "Poisoner"]);
    let state = add_player(&state, "Rae", "Ravenkeeper", "good");
    let state = add_player(&state, "Pat", "Poisoner", "evil");
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::FirstNightSet {
                is_first_night: false,
            },
        ),
    );

    let names: Vec<String> = state
        .get_night_steps()
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert!(names.contains(&"Ravenkeeper".to_string()));
    assert!(names.contains(&"Poisoner".to_string()));

    let mut state = state;
    for name in ["Rae", "Pat"] {
        state = apply(
            &state,
            &evt(
                &state,
                EventPayload::PlayerAliveSet {
                    player_name: name.to_string(),
                    is_alive: false,
                    cleared_effects: vec![],
                },
            ),
        );
    }
    let names: Vec<String> = state
        .get_night_steps()
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert!(names.contains(&"Ravenkeeper".to_string()), "{names:?}");
    assert!(!names.contains(&"Poisoner".to_string()), "{names:?}");
}

#[test]
fn unclaimed_travelers_exclude_those_in_play() {
    let state = game_with_roles(&[]);
    assert_eq!(state.get_unclaimed_travelers().len(), 5);

    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::TravelerAdded {
                player_name: "Wanderer".to_string(),
                traveler_name: "Beggar".to_string(),
                alignment: "good".to_string(),
            },
        ),
    );
    let unclaimed: Vec<&str> = state
        .get_unclaimed_travelers()
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(unclaimed.len(), 4);
    assert!(!unclaimed.contains(&"Beggar"));
}

#[test]
fn player_out_resolves_travelers_and_falls_back_for_unknown_characters() {
    use deaths_door::game_state::{player_state_to_out, PlayerState};
    use deaths_door::scripts::get_script_by_name;

    let script = get_script_by_name("trouble_brewing").unwrap();

    // Travelers resolve through the script's traveler list.
    let traveler = PlayerState::new(
        "Wanderer".to_string(),
        "Beggar".to_string(),
        "good".to_string(),
    );
    let out = player_state_to_out(&traveler, script);
    assert_eq!(out.character.name, "Beggar");
    assert!(!out.character.description.is_empty());

    // An unknown character yields a minimal fallback instead of a panic
    // (this can happen when replaying a log written against other data).
    let unknown = PlayerState::new(
        "Alice".to_string(),
        "Mystery Role".to_string(),
        "evil".to_string(),
    );
    let out = player_state_to_out(&unknown, script);
    assert_eq!(out.character.name, "Mystery Role");
    assert_eq!(out.character.icon_path, "mysteryrole.png");
    assert!(out.character.description.is_empty());
}

#[test]
fn derived_getters_are_empty_for_an_unknown_script() {
    let mut state = GameState::initial(Uuid::new_v4(), "not_a_real_script");
    state.included_role_names = vec!["Imp".to_string()];
    state.demon_bluffs = vec!["Mayor".to_string()];

    assert!(state.get_script().is_none());
    assert!(state.get_night_steps().is_empty());
    assert!(state.get_included_roles().is_empty());
    assert!(state.get_demon_bluffs().is_empty());
    assert!(state.get_status_effects().is_empty());
    assert!(state.get_unclaimed_travelers().is_empty());
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

/// One instance of every payload variant, for exhaustive serialization checks.
/// (A new variant won't be picked up automatically — add it here too.)
fn all_payload_variants() -> Vec<EventPayload> {
    let s = |v: &str| v.to_string();
    vec![
        EventPayload::GameCreated {
            script_name: s("trouble_brewing"),
        },
        EventPayload::NightStepSet { step: s("Dusk") },
        EventPayload::FirstNightSet {
            is_first_night: false,
        },
        EventPayload::RoleVisibilitySet {
            should_reveal_roles: true,
        },
        EventPayload::RoleIncluded { name: s("Imp") },
        EventPayload::RolesIncluded {
            names: vec![s("Imp"), s("Chef")],
        },
        EventPayload::RoleRemoved { name: s("Imp") },
        EventPayload::PlayerAdded {
            player_name: s("Alice"),
            character_name: s("Imp"),
            alignment: s("evil"),
        },
        EventPayload::TravelerAdded {
            player_name: s("Wanderer"),
            traveler_name: s("Beggar"),
            alignment: s("good"),
        },
        EventPayload::PlayerRemoved {
            player_name: s("Alice"),
        },
        EventPayload::PlayerRenamed {
            old_name: s("Alice"),
            new_name: s("Alicia"),
        },
        EventPayload::CharactersSwapped {
            name1: s("Alice"),
            name2: s("Bob"),
        },
        EventPayload::PlayerAliveSet {
            player_name: s("Alice"),
            is_alive: false,
            cleared_effects: vec![(s("Cara"), s("Poisoned"))],
        },
        EventPayload::DeadVoteUsedSet {
            player_name: s("Alice"),
            has_used_dead_vote: true,
        },
        EventPayload::PlayerAlignmentSet {
            player_name: s("Alice"),
            alignment: s("good"),
        },
        EventPayload::StatusEffectAdded {
            player_name: s("Alice"),
            effect: s("Poisoned"),
        },
        EventPayload::StatusEffectRemoved {
            player_name: s("Alice"),
            effect: s("Poisoned"),
        },
        EventPayload::DemonBluffsSet {
            bluffs: vec![s("Mayor")],
        },
        EventPayload::ChoppingBlockSet {
            player_name: s("Alice"),
            votes: Some(4),
        },
        EventPayload::ChoppingBlockCleared,
        EventPayload::DayBegan,
        EventPayload::NightBegan,
        EventPayload::DeathAnnounced {
            player_name: s("Alice"),
        },
        EventPayload::NominationRecorded {
            player_name: s("Alice"),
            voters: vec![s("Bob"), s("Carol")],
            votes: 2,
        },
        EventPayload::PlayerExecuted {
            player_name: s("Alice"),
            cleared_effects: vec![(s("Cara"), s("Poisoned"))],
        },
        EventPayload::GameEnded { winner: s("good") },
    ]
}

/// The serde `type` tag is what's stored on disk, and `event_type()` is what's
/// written to the store's `event_type` column — they must agree for every
/// variant, and every variant must round-trip losslessly.
#[test]
fn every_event_variant_round_trips_with_matching_type_tag() {
    for payload in all_payload_variants() {
        let v = serde_json::to_value(&payload).unwrap();
        assert_eq!(
            v["type"],
            payload.event_type(),
            "serde tag and event_type() disagree for {payload:?}"
        );
        let back: EventPayload = serde_json::from_value(v).unwrap();
        assert_eq!(back, payload);
    }
}

/// Events written before `cleared_effects` / `votes` existed have no such keys
/// in the on-disk JSON; replaying an old database must still work.
#[test]
fn legacy_event_json_without_optional_fields_deserializes() {
    let alive: EventPayload = serde_json::from_str(
        r#"{"type": "player_alive_set", "player_name": "Alice", "is_alive": false}"#,
    )
    .unwrap();
    assert_eq!(
        alive,
        EventPayload::PlayerAliveSet {
            player_name: "Alice".to_string(),
            is_alive: false,
            cleared_effects: vec![],
        }
    );

    let block: EventPayload =
        serde_json::from_str(r#"{"type": "chopping_block_set", "player_name": "Alice"}"#).unwrap();
    assert_eq!(
        block,
        EventPayload::ChoppingBlockSet {
            player_name: "Alice".to_string(),
            votes: None,
        }
    );
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

// --- Day/night phase and nominations ---

fn set_alive(state: &GameState, name: &str, is_alive: bool) -> GameState {
    apply(
        state,
        &evt(
            state,
            EventPayload::PlayerAliveSet {
                player_name: name.to_string(),
                is_alive,
                cleared_effects: vec![],
            },
        ),
    )
}

#[test]
fn phase_transitions_and_day_counting() {
    let state = game_with_roles(&["Imp"]);
    // Games open on the first night.
    assert_eq!(state.phase, Phase::Night);
    assert_eq!(state.day_number, 0);
    assert!(state.is_first_night);

    let day = apply(&state, &evt(&state, EventPayload::DayBegan));
    assert_eq!(day.phase, Phase::Day);
    assert_eq!(day.day_number, 1);
    assert_eq!(day.current_night_step, "Dawn");
    assert!(day.is_first_night); // untouched until a night begins

    let night = apply(&day, &evt(&day, EventPayload::NightBegan));
    assert_eq!(night.phase, Phase::Night);
    assert_eq!(night.current_night_step, "Dusk");
    assert!(!night.is_first_night); // a night after a day is never the first

    // The raw night-step bookmark keeps the phase coherent for old clients.
    let via_step = apply(
        &night,
        &evt(
            &night,
            EventPayload::NightStepSet {
                step: "Dawn".to_string(),
            },
        ),
    );
    assert_eq!(via_step.phase, Phase::Day);
    let back = apply(
        &via_step,
        &evt(
            &via_step,
            EventPayload::NightStepSet {
                step: "Poisoner".to_string(),
            },
        ),
    );
    assert_eq!(back.phase, Phase::Night);
}

#[test]
fn night_deaths_queue_for_announcement_and_day_deaths_do_not() {
    let state = game_with_roles(&["Imp", "Chef", "Mayor"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");

    // Night kill -> queued (once, even if re-marked dead).
    let state = set_alive(&state, "Alice", false);
    let state = set_alive(&state, "Alice", false);
    assert_eq!(state.deaths_to_announce, vec!["Alice"]);

    // Revival takes the death back off the queue.
    let state = set_alive(&state, "Alice", true);
    assert!(state.deaths_to_announce.is_empty());

    // A daytime death is public — nothing to announce.
    let state = set_alive(&state, "Alice", false);
    let day = apply(&state, &evt(&state, EventPayload::DayBegan));
    let day = set_alive(&day, "Bob", false);
    assert_eq!(day.deaths_to_announce, vec!["Alice"]);

    // Announcing checks the player off; renames follow; removal drops them.
    let announced = apply(
        &day,
        &evt(
            &day,
            EventPayload::DeathAnnounced {
                player_name: "Alice".to_string(),
            },
        ),
    );
    assert!(announced.deaths_to_announce.is_empty());

    let renamed = apply(
        &day,
        &evt(
            &day,
            EventPayload::PlayerRenamed {
                old_name: "Alice".to_string(),
                new_name: "Alicia".to_string(),
            },
        ),
    );
    assert_eq!(renamed.deaths_to_announce, vec!["Alicia"]);

    let removed = apply(
        &day,
        &evt(
            &day,
            EventPayload::PlayerRemoved {
                player_name: "Alice".to_string(),
            },
        ),
    );
    assert!(removed.deaths_to_announce.is_empty());
}

#[test]
fn nominations_resolve_the_chopping_block() {
    // Five players, all alive -> threshold ceil(5/2) = 3.
    let mut state = game_with_roles(&["Imp", "Chef", "Empath", "Mayor", "Monk"]);
    for (n, r) in [
        ("Alice", "Imp"),
        ("Bob", "Chef"),
        ("Carol", "Empath"),
        ("Dave", "Mayor"),
        ("Erin", "Monk"),
    ] {
        state = add_player(&state, n, r, "good");
    }
    let state = apply(&state, &evt(&state, EventPayload::DayBegan));
    let nominate = |state: &GameState, nominee: &str, voters: &[&str], votes: u32| {
        apply(
            state,
            &evt(
                state,
                EventPayload::NominationRecorded {
                    player_name: nominee.to_string(),
                    voters: voters.iter().map(|v| v.to_string()).collect(),
                    votes,
                },
            ),
        )
    };

    // Below threshold: nothing happens.
    let state = nominate(&state, "Alice", &["Bob", "Carol"], 2);
    assert_eq!(
        state.nominations_today.last().unwrap().outcome,
        NominationOutcome::BlockUnchanged
    );
    assert!(state.chopping_block.is_none());

    // Meets threshold on an empty block: on the block.
    let state = nominate(&state, "Bob", &["Alice", "Carol", "Dave"], 3);
    assert_eq!(
        state.nominations_today.last().unwrap().outcome,
        NominationOutcome::OnTheBlock
    );
    assert_eq!(state.chopping_block.as_ref().unwrap().player_name, "Bob");
    assert_eq!(state.chopping_block.as_ref().unwrap().votes, Some(3));

    // Fewer votes than the block: unchanged.
    let state = nominate(&state, "Carol", &["Alice", "Bob", "Erin"], 3 - 1);
    assert_eq!(
        state.nominations_today.last().unwrap().outcome,
        NominationOutcome::BlockUnchanged
    );
    assert_eq!(state.chopping_block.as_ref().unwrap().player_name, "Bob");

    // Tie: the block empties (no one is executed on a tie).
    let state = nominate(&state, "Dave", &["Alice", "Bob", "Erin"], 3);
    assert_eq!(
        state.nominations_today.last().unwrap().outcome,
        NominationOutcome::TieBlockEmptied
    );
    assert!(state.chopping_block.is_none());

    // Beat: takes the (re-)block.
    let state = nominate(&state, "Erin", &["Alice", "Bob", "Carol", "Dave"], 4);
    assert_eq!(state.chopping_block.as_ref().unwrap().player_name, "Erin");

    // Night clears the day's nomination record and the block.
    let night = apply(&state, &evt(&state, EventPayload::NightBegan));
    assert!(night.nominations_today.is_empty());
    assert!(night.chopping_block.is_none());
}

#[test]
fn nominations_spend_dead_votes_and_beat_uncounted_blocks() {
    let state = game_with_roles(&["Imp", "Chef", "Empath"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");
    let state = add_player(&state, "Carol", "Empath", "good");
    let state = set_alive(&state, "Bob", false);
    let state = apply(&state, &evt(&state, EventPayload::DayBegan));

    // A manually-set block with no recorded votes is replaced by any
    // threshold-meeting vote (threshold: ceil(2/2) = 1).
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::ChoppingBlockSet {
                player_name: "Carol".to_string(),
                votes: None,
            },
        ),
    );
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::NominationRecorded {
                player_name: "Alice".to_string(),
                voters: vec!["Bob".to_string(), "Carol".to_string()],
                votes: 2,
            },
        ),
    );
    assert_eq!(state.chopping_block.as_ref().unwrap().player_name, "Alice");

    // Dead Bob voted: his one dead vote is spent. Living Carol's isn't.
    assert!(state.get_player("Bob").unwrap().has_used_dead_vote);
    assert!(!state.get_player("Carol").unwrap().has_used_dead_vote);
    assert_eq!(state.eligible_voters(), vec!["Alice", "Carol"]);
}

#[test]
fn execution_kills_publicly_and_clears_the_block() {
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
    let state = apply(&state, &evt(&state, EventPayload::DayBegan));
    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::ChoppingBlockSet {
                player_name: "Pat".to_string(),
                votes: Some(2),
            },
        ),
    );

    let state = apply(
        &state,
        &evt(
            &state,
            EventPayload::PlayerExecuted {
                player_name: "Pat".to_string(),
                cleared_effects: vec![("Cara".to_string(), "Poisoned".to_string())],
            },
        ),
    );
    assert!(!state.get_player("Pat").unwrap().is_alive);
    assert!(state.get_player("Cara").unwrap().status_effects.is_empty());
    assert!(state.chopping_block.is_none());
    // Executions are public: nothing queues for announcement.
    assert!(state.deaths_to_announce.is_empty());
}

#[test]
fn game_over_hint_and_recorded_winner() {
    // No demon in play: no hint, whatever the counts.
    let state = game_with_roles(&["Chef", "Empath"]);
    let state = add_player(&state, "Bob", "Chef", "good");
    assert_eq!(state.game_over_hint(), None);

    let state = game_with_roles(&["Imp", "Chef", "Empath"]);
    let state = add_player(&state, "Alice", "Imp", "evil");
    let state = add_player(&state, "Bob", "Chef", "good");
    let state = add_player(&state, "Carol", "Empath", "good");
    assert_eq!(state.game_over_hint(), None);

    // Demon dead -> good may have won.
    let dead_demon = set_alive(&state, "Alice", false);
    assert!(dead_demon
        .game_over_hint()
        .unwrap()
        .contains("demon is dead"));

    // Two living players with the demon standing -> evil may have won.
    let two_left = set_alive(&state, "Bob", false);
    assert!(two_left
        .game_over_hint()
        .unwrap()
        .contains("evil may have won"));

    // Recording the winner stores it and silences the hint.
    let ended = apply(
        &two_left,
        &evt(
            &two_left,
            EventPayload::GameEnded {
                winner: "evil".to_string(),
            },
        ),
    );
    assert_eq!(ended.winner.as_deref(), Some("evil"));
    assert_eq!(ended.game_over_hint(), None);
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
fn store_get_events_up_to_sequence_is_inclusive() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    for (i, name) in ["Imp", "Chef", "Mayor"].into_iter().enumerate() {
        let e = GameEvent::new(
            game_id,
            i as i64,
            EventPayload::RoleIncluded {
                name: name.to_string(),
            },
        );
        store.append(&e).unwrap();
    }

    let events = store.get_events(game_id, Some(1)).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events.last().unwrap().sequence, 1);
}

/// The UNIQUE(game_id, sequence) constraint is the store's defense against two
/// writers corrupting a game's event log; violating it must surface as an Err.
#[test]
fn store_rejects_duplicate_sequence_for_same_game() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    let payload = EventPayload::GameCreated {
        script_name: "trouble_brewing".to_string(),
    };
    store
        .append(&GameEvent::new(game_id, 0, payload.clone()))
        .unwrap();
    assert!(store.append(&GameEvent::new(game_id, 0, payload)).is_err());
    assert_eq!(store.get_events(game_id, None).unwrap().len(), 1);
}

#[test]
fn store_round_trips_event_ids_and_timestamps() {
    let store = EventStore::in_memory().unwrap();
    let game_id = Uuid::new_v4();
    let event = GameEvent::new(
        game_id,
        0,
        EventPayload::GameCreated {
            script_name: "trouble_brewing".to_string(),
        },
    );
    store.append(&event).unwrap();

    let loaded = &store.get_events(game_id, None).unwrap()[0];
    assert_eq!(loaded.id, event.id);
    assert_eq!(loaded.game_id, event.game_id);
    // RFC3339 storage keeps sub-second precision, so history timestamps survive.
    assert_eq!(loaded.timestamp, event.timestamp);
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
