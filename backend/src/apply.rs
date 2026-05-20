//! Pure functions to apply events to game state.

use crate::error::GameError;
use crate::events::{EventPayload, GameEvent};
use crate::game_state::{GameState, PlayerState};

fn normalize(s: &str) -> String {
    s.to_lowercase().trim().to_string()
}

/// Remove the first role matching `name` (case-insensitive) from `roles`.
fn remove_first_role(roles: &mut Vec<String>, name: &str) {
    let normalized = normalize(name);
    if let Some(idx) = roles.iter().position(|r| normalize(r) == normalized) {
        roles.remove(idx);
    }
}

/// Apply a single event to produce a new game state. Pure function.
pub fn apply(state: &GameState, event: &GameEvent) -> GameState {
    match &event.payload {
        EventPayload::GameCreated { script_name } => {
            let mut next = GameState::initial(event.game_id, script_name.clone());
            next.version = state.version + 1;
            next
        }

        EventPayload::NightStepSet { step } => {
            let mut next = state.clone();
            next.current_night_step = step.clone();
            next.version += 1;
            next
        }

        EventPayload::FirstNightSet { is_first_night } => {
            let mut next = state.clone();
            next.is_first_night = *is_first_night;
            next.current_night_step = "Dusk".to_string();
            next.version += 1;
            next
        }

        EventPayload::RoleVisibilitySet {
            should_reveal_roles,
        } => {
            let mut next = state.clone();
            next.should_reveal_roles = *should_reveal_roles;
            next.version += 1;
            next
        }

        EventPayload::RoleIncluded { name } => {
            let mut next = state.clone();
            next.included_role_names.push(name.clone());
            next.version += 1;
            next
        }

        EventPayload::RolesIncluded { names } => {
            let mut next = state.clone();
            next.included_role_names.extend(names.iter().cloned());
            next.version += 1;
            next
        }

        EventPayload::RoleRemoved { name } => {
            let mut next = state.clone();
            remove_first_role(&mut next.included_role_names, name);
            next.version += 1;
            next
        }

        EventPayload::PlayerAdded {
            player_name,
            character_name,
            alignment,
        } => {
            let mut next = state.clone();
            // Remove the assigned character from the pool.
            remove_first_role(&mut next.included_role_names, character_name);
            next.players.push(PlayerState::new(
                player_name.clone(),
                character_name.clone(),
                alignment.clone(),
            ));
            next.version += 1;
            next
        }

        EventPayload::TravelerAdded {
            player_name,
            traveler_name,
            alignment,
        } => {
            let mut next = state.clone();
            next.players.push(PlayerState::new(
                player_name.clone(),
                traveler_name.clone(),
                alignment.clone(),
            ));
            next.version += 1;
            next
        }

        EventPayload::PlayerRemoved { player_name } => {
            let mut next = state.clone();
            if let Some(idx) = next.players.iter().position(|p| &p.name == player_name) {
                let removed = next.players.remove(idx);
                next.included_role_names.push(removed.character_name);
            }
            next.version += 1;
            next
        }

        EventPayload::PlayerRenamed { old_name, new_name } => {
            let mut next = state.replace_player(old_name, |p| p.name = new_name.clone());
            next.version += 1;
            next
        }

        EventPayload::CharactersSwapped { name1, name2 } => {
            let mut next = state.clone();
            let c1 = next.get_player(name1).map(|p| p.character_name.clone());
            let c2 = next.get_player(name2).map(|p| p.character_name.clone());
            if let (Some(c1), Some(c2)) = (c1, c2) {
                if let Some(p1) = next.players.iter_mut().find(|p| &p.name == name1) {
                    p1.character_name = c2;
                }
                if let Some(p2) = next.players.iter_mut().find(|p| &p.name == name2) {
                    p2.character_name = c1;
                }
            }
            next.version += 1;
            next
        }

        EventPayload::PlayerAliveSet {
            player_name,
            is_alive,
            cleared_effects,
        } => {
            let mut next = state.replace_player(player_name, |p| p.is_alive = *is_alive);
            // Apply cascading effect removals.
            for (target_name, effect) in cleared_effects {
                next = next.replace_player(target_name, |p| {
                    p.status_effects.retain(|e| e != effect);
                });
            }
            next.version += 1;
            next
        }

        EventPayload::DeadVoteUsedSet {
            player_name,
            has_used_dead_vote,
        } => {
            let mut next =
                state.replace_player(player_name, |p| p.has_used_dead_vote = *has_used_dead_vote);
            next.version += 1;
            next
        }

        EventPayload::PlayerAlignmentSet {
            player_name,
            alignment,
        } => {
            let mut next = state.replace_player(player_name, |p| p.alignment = alignment.clone());
            next.version += 1;
            next
        }

        EventPayload::StatusEffectAdded {
            player_name,
            effect,
        } => {
            let mut next = state.replace_player(player_name, |p| {
                if !p.status_effects.contains(effect) {
                    p.status_effects.push(effect.clone());
                }
            });
            next.version += 1;
            next
        }

        EventPayload::StatusEffectRemoved {
            player_name,
            effect,
        } => {
            let mut next = state.replace_player(player_name, |p| {
                p.status_effects.retain(|e| e != effect);
            });
            next.version += 1;
            next
        }
    }
}

/// Rebuild game state by replaying a sequence of events.
pub fn replay(events: &[GameEvent]) -> Result<GameState, GameError> {
    let first = events.first().ok_or(GameError::EmptyReplay)?;
    let initial = GameState::initial(first.game_id, "");
    Ok(events
        .iter()
        .fold(initial, |state, event| apply(&state, event)))
}
