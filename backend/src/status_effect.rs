//! Status effects applied to characters, and the death-cleanup rules for them.

use serde::{Deserialize, Serialize};

use crate::game_state::GameState;

/// A single status effect applied to a character (API output model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusEffectOut {
    pub name: String,
    pub character_name: String,
}

/// Persistent status effects that stop applying when their source character
/// dies (death cleanup).
pub fn character_persistent_effects(character_name: &str) -> &'static [&'static str] {
    match character_name {
        "Poisoner" => &["Poisoned"],
        "Monk" => &["Safe"],
        "Butler" => &["Butler's Master"],
        _ => &[],
    }
}

/// Cascading status-effect removals when a player dies: every instance of the
/// dying character's persistent effects, on any player, as `(player, effect)`
/// pairs. Baked into the `PlayerAliveSet` event so the log records exactly
/// what was cleared.
pub fn compute_death_cleared_effects(
    state: &GameState,
    player_name: &str,
) -> Vec<(String, String)> {
    let Some(player) = state.get_player(player_name) else {
        return Vec::new();
    };
    let effects_to_remove = character_persistent_effects(&player.character_name);
    if effects_to_remove.is_empty() {
        return Vec::new();
    }
    let mut cleared = Vec::new();
    for p in &state.players {
        for effect in effects_to_remove {
            if p.status_effects.iter().any(|e| e == effect) {
                cleared.push((p.name.clone(), effect.to_string()));
            }
        }
    }
    cleared
}
