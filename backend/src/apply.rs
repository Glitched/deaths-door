//! Pure functions to validate and apply events to game state.

use crate::error::GameError;
use crate::events::{EventPayload, GameEvent};
use crate::game_state::{ChoppingBlock, GameState, PlayerState};

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

fn ensure_player_exists(state: &GameState, name: &str) -> Result<(), GameError> {
    if state.get_player(name).is_none() {
        return Err(GameError::NotFound(format!("Player not found: {name}")));
    }
    Ok(())
}

fn ensure_player_absent(state: &GameState, name: &str) -> Result<(), GameError> {
    if state.get_player(name).is_some() {
        return Err(GameError::Conflict(format!(
            "Player with name {name} already exists."
        )));
    }
    Ok(())
}

/// Check `name` against the script's characters, or pass when the script is
/// unknown (there is nothing to validate against).
fn ensure_role_in_script(
    state: &GameState,
    name: &str,
    missing: impl FnOnce() -> GameError,
) -> Result<(), GameError> {
    match state.get_script() {
        None => Ok(()),
        Some(script) if script.get_character(name).is_some() => Ok(()),
        Some(_) => Err(missing()),
    }
}

/// Validate an event payload against the state it would be applied to.
///
/// Called by `GameManager::dispatch` while holding the state lock, so these
/// checks are race-free: they see exactly the state the event will mutate.
/// Replay intentionally skips validation — persisted events are facts.
pub fn validate(state: &GameState, payload: &EventPayload) -> Result<(), GameError> {
    match payload {
        EventPayload::PlayerAdded {
            player_name,
            character_name,
            ..
        } => {
            ensure_player_absent(state, player_name)?;
            let normalized = normalize(character_name);
            let in_pool = state
                .included_role_names
                .iter()
                .any(|r| normalize(r) == normalized);
            if !in_pool {
                return Err(GameError::InvalidInput(format!(
                    "Role not in pool: {character_name}"
                )));
            }
            Ok(())
        }

        EventPayload::TravelerAdded {
            player_name,
            traveler_name,
            ..
        } => {
            ensure_player_absent(state, player_name)?;
            let unclaimed = state
                .get_unclaimed_travelers()
                .iter()
                .any(|t| t.is_named(traveler_name));
            if !unclaimed {
                return Err(GameError::NotFound(format!(
                    "Traveler not found or in game: {traveler_name}"
                )));
            }
            Ok(())
        }

        EventPayload::PlayerRemoved { player_name }
        | EventPayload::PlayerAliveSet { player_name, .. }
        | EventPayload::DeadVoteUsedSet { player_name, .. }
        | EventPayload::PlayerAlignmentSet { player_name, .. }
        | EventPayload::StatusEffectRemoved { player_name, .. } => {
            ensure_player_exists(state, player_name)
        }

        EventPayload::StatusEffectAdded {
            player_name,
            effect,
        } => {
            ensure_player_exists(state, player_name)?;
            if effect.trim().is_empty() {
                return Err(GameError::InvalidInput(
                    "Status effect name cannot be empty".to_string(),
                ));
            }
            Ok(())
        }

        EventPayload::PlayerRenamed { old_name, new_name } => {
            ensure_player_exists(state, old_name)?;
            if new_name != old_name {
                ensure_player_absent(state, new_name)?;
            }
            Ok(())
        }

        EventPayload::CharactersSwapped { name1, name2 } => {
            ensure_player_exists(state, name1)?;
            ensure_player_exists(state, name2)
        }

        EventPayload::ChoppingBlockSet { player_name, .. } => {
            let player = state
                .get_player(player_name)
                .ok_or_else(|| GameError::NotFound(format!("Player not found: {player_name}")))?;
            if !player.is_alive {
                return Err(GameError::InvalidInput(format!(
                    "{player_name} is dead and cannot be executed"
                )));
            }
            Ok(())
        }

        EventPayload::RoleIncluded { name } => ensure_role_in_script(state, name, || {
            GameError::InvalidInput(format!("Role not found: {name}"))
        }),

        EventPayload::RolesIncluded { names } => names.iter().try_for_each(|name| {
            ensure_role_in_script(state, name, || {
                GameError::InvalidInput(format!("Role not found: {name}"))
            })
        }),

        EventPayload::RoleRemoved { name } => {
            let normalized = normalize(name);
            let present = state
                .included_role_names
                .iter()
                .any(|r| normalize(r) == normalized);
            if !present {
                return Err(GameError::NotFound(format!("Role not in game: {name}")));
            }
            Ok(())
        }

        EventPayload::DemonBluffsSet { bluffs } => {
            if bluffs.len() > 3 {
                return Err(GameError::InvalidInput(format!(
                    "At most 3 demon bluffs allowed, got {}",
                    bluffs.len()
                )));
            }
            bluffs.iter().try_for_each(|name| {
                ensure_role_in_script(state, name, || {
                    GameError::NotFound(format!("Role '{name}' not found in script"))
                })
            })
        }

        EventPayload::GameCreated { .. }
        | EventPayload::NightStepSet { .. }
        | EventPayload::FirstNightSet { .. }
        | EventPayload::RoleVisibilitySet { .. }
        | EventPayload::ChoppingBlockCleared => Ok(()),
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
            // The chopping block only exists during the day; entering night
            // (any step other than "Dawn") resolves it.
            if step.as_str() != "Dawn" {
                next.chopping_block = None;
            }
            next.version += 1;
            next
        }

        EventPayload::FirstNightSet { is_first_night } => {
            let mut next = state.clone();
            next.is_first_night = *is_first_night;
            next.current_night_step = "Dusk".to_string();
            next.chopping_block = None;
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
            clear_chopping_block_for(&mut next, player_name);
            next.version += 1;
            next
        }

        EventPayload::PlayerRenamed { old_name, new_name } => {
            let mut next = state.replace_player(old_name, |p| p.name = new_name.clone());
            if let Some(block) = &mut next.chopping_block {
                if &block.player_name == old_name {
                    block.player_name = new_name.clone();
                }
            }
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
            // A dead player can't be executed; their block resolves.
            if !*is_alive {
                clear_chopping_block_for(&mut next, player_name);
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

        EventPayload::DemonBluffsSet { bluffs } => {
            let mut next = state.clone();
            next.demon_bluffs = bluffs.clone();
            next.version += 1;
            next
        }

        EventPayload::ChoppingBlockSet { player_name, votes } => {
            let mut next = state.clone();
            next.chopping_block = Some(ChoppingBlock {
                player_name: player_name.clone(),
                votes: *votes,
            });
            next.version += 1;
            next
        }

        EventPayload::ChoppingBlockCleared => {
            let mut next = state.clone();
            next.chopping_block = None;
            next.version += 1;
            next
        }
    }
}

/// Clear the chopping block if `player_name` is the player on it.
fn clear_chopping_block_for(state: &mut GameState, player_name: &str) {
    if state
        .chopping_block
        .as_ref()
        .is_some_and(|b| b.player_name == player_name)
    {
        state.chopping_block = None;
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
