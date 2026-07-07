//! Pure functions to apply events to game state.

use crate::error::GameError;
use crate::events::{EventPayload, GameEvent};
use crate::game_state::{
    ChoppingBlock, GameState, Nomination, NominationOutcome, Phase, PlayerState,
};

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
            // The phase follows the bookmark ("Dawn" is the day marker), so
            // clients driving raw steps keep the phase coherent. The chopping
            // block only exists during the day; entering night resolves it.
            if step.as_str() == "Dawn" {
                next.phase = Phase::Day;
            } else {
                next.phase = Phase::Night;
                next.chopping_block = None;
            }
            next.version += 1;
            next
        }

        EventPayload::FirstNightSet { is_first_night } => {
            let mut next = state.clone();
            next.is_first_night = *is_first_night;
            next.current_night_step = "Dusk".to_string();
            next.phase = Phase::Night;
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
            next.deaths_to_announce.retain(|n| n != player_name);
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
            for name in next.deaths_to_announce.iter_mut() {
                if name == old_name {
                    *name = new_name.clone();
                }
            }
            for nomination in next.nominations_today.iter_mut() {
                if &nomination.player_name == old_name {
                    nomination.player_name = new_name.clone();
                }
                for voter in nomination.voters.iter_mut() {
                    if voter == old_name {
                        *voter = new_name.clone();
                    }
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
            let player_exists = state.get_player(player_name).is_some();
            let mut next = state.replace_player(player_name, |p| p.is_alive = *is_alive);
            // Apply cascading effect removals.
            for (target_name, effect) in cleared_effects {
                next = next.replace_player(target_name, |p| {
                    p.status_effects.retain(|e| e != effect);
                });
            }
            if !*is_alive {
                // A dead player can't be executed; their block resolves.
                clear_chopping_block_for(&mut next, player_name);
                // Night deaths are secret until announced at dawn (day deaths
                // are public as they happen).
                if player_exists
                    && next.phase == Phase::Night
                    && !next.deaths_to_announce.iter().any(|n| n == player_name)
                {
                    next.deaths_to_announce.push(player_name.clone());
                }
            } else {
                // A revived player has no death to announce.
                next.deaths_to_announce.retain(|n| n != player_name);
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

        EventPayload::DayBegan => {
            let mut next = state.clone();
            next.phase = Phase::Day;
            next.current_night_step = "Dawn".to_string();
            next.day_number += 1;
            next.version += 1;
            next
        }

        EventPayload::NightBegan => {
            let mut next = state.clone();
            next.phase = Phase::Night;
            next.current_night_step = "Dusk".to_string();
            // The first night is the one the game starts in; a night begun
            // after any day is a later night.
            next.is_first_night = next.day_number == 0;
            next.chopping_block = None;
            next.nominations_today.clear();
            next.version += 1;
            next
        }

        EventPayload::DeathAnnounced { player_name } => {
            let mut next = state.clone();
            next.deaths_to_announce.retain(|n| n != player_name);
            next.version += 1;
            next
        }

        EventPayload::NominationRecorded {
            player_name,
            voters,
            votes,
        } => {
            let mut next = state.clone();
            // Dead voters spend their one dead vote.
            for voter in voters {
                next = next.replace_player(voter, |p| {
                    if !p.is_alive {
                        p.has_used_dead_vote = true;
                    }
                });
            }
            let outcome = resolve_nomination(&mut next, player_name, *votes);
            next.nominations_today.push(Nomination {
                player_name: player_name.clone(),
                votes: *votes,
                voters: voters.clone(),
                outcome,
            });
            next.version += 1;
            next
        }

        EventPayload::PlayerExecuted {
            player_name,
            cleared_effects,
        } => {
            let mut next = state.replace_player(player_name, |p| p.is_alive = false);
            for (target_name, effect) in cleared_effects {
                next = next.replace_player(target_name, |p| {
                    p.status_effects.retain(|e| e != effect);
                });
            }
            // Executions are public: the block resolves and there is nothing
            // to announce at dawn.
            clear_chopping_block_for(&mut next, player_name);
            next.version += 1;
            next
        }

        EventPayload::GameEnded { winner } => {
            let mut next = state.clone();
            next.winner = Some(winner.clone());
            next.version += 1;
            next
        }
    }
}

/// Apply a confirmed nomination's vote count to the chopping block.
///
/// Meeting the execution threshold AND beating the current block takes the
/// block; exactly tying the current block's votes empties it (a tie means no
/// one is executed); anything less changes nothing. A block whose vote count
/// was never recorded (set manually without votes) is replaced by any
/// threshold-meeting vote and can't be tied.
fn resolve_nomination(state: &mut GameState, nominee: &str, votes: u32) -> NominationOutcome {
    let threshold = state.execution_threshold() as u32;
    let block_votes = state.chopping_block.as_ref().map(|b| b.votes);
    let beats_block = match block_votes {
        None | Some(None) => true,
        Some(Some(current)) => votes > current,
    };
    if votes >= threshold && beats_block {
        state.chopping_block = Some(ChoppingBlock {
            player_name: nominee.to_string(),
            votes: Some(votes),
        });
        NominationOutcome::OnTheBlock
    } else if matches!(block_votes, Some(Some(current)) if current == votes) {
        state.chopping_block = None;
        NominationOutcome::TieBlockEmptied
    } else {
        NominationOutcome::BlockUnchanged
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
