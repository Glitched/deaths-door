//! Event types and payload models for event sourcing.
//!
//! The Python codebase uses a Pydantic discriminated union keyed on a `type`
//! field. The Rust equivalent is an internally-tagged serde enum, which is
//! both more compact and exhaustively checked by the compiler.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persisted game event payload. The `type` tag value matches the Python
/// `EventType` enum string values exactly, for wire/storage compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    GameCreated {
        script_name: String,
    },
    NightStepSet {
        step: String,
    },
    FirstNightSet {
        is_first_night: bool,
    },
    RoleVisibilitySet {
        should_reveal_roles: bool,
    },
    RoleIncluded {
        name: String,
    },
    RolesIncluded {
        names: Vec<String>,
    },
    RoleRemoved {
        name: String,
    },
    PlayerAdded {
        player_name: String,
        character_name: String,
        alignment: String,
    },
    TravelerAdded {
        player_name: String,
        traveler_name: String,
        alignment: String,
    },
    PlayerRemoved {
        player_name: String,
    },
    PlayerRenamed {
        old_name: String,
        new_name: String,
    },
    CharactersSwapped {
        name1: String,
        name2: String,
    },
    PlayerAliveSet {
        player_name: String,
        is_alive: bool,
        #[serde(default)]
        cleared_effects: Vec<(String, String)>,
    },
    DeadVoteUsedSet {
        player_name: String,
        has_used_dead_vote: bool,
    },
    PlayerAlignmentSet {
        player_name: String,
        alignment: String,
    },
    StatusEffectAdded {
        player_name: String,
        effect: String,
    },
    StatusEffectRemoved {
        player_name: String,
        effect: String,
    },
    DemonBluffsSet {
        bluffs: Vec<String>,
    },
}

impl EventPayload {
    /// The machine-readable event type string (matches Python `EventType`).
    pub fn event_type(&self) -> &'static str {
        match self {
            EventPayload::GameCreated { .. } => "game_created",
            EventPayload::NightStepSet { .. } => "night_step_set",
            EventPayload::FirstNightSet { .. } => "first_night_set",
            EventPayload::RoleVisibilitySet { .. } => "role_visibility_set",
            EventPayload::RoleIncluded { .. } => "role_included",
            EventPayload::RolesIncluded { .. } => "roles_included",
            EventPayload::RoleRemoved { .. } => "role_removed",
            EventPayload::PlayerAdded { .. } => "player_added",
            EventPayload::TravelerAdded { .. } => "traveler_added",
            EventPayload::PlayerRemoved { .. } => "player_removed",
            EventPayload::PlayerRenamed { .. } => "player_renamed",
            EventPayload::CharactersSwapped { .. } => "characters_swapped",
            EventPayload::PlayerAliveSet { .. } => "player_alive_set",
            EventPayload::DeadVoteUsedSet { .. } => "dead_vote_used_set",
            EventPayload::PlayerAlignmentSet { .. } => "player_alignment_set",
            EventPayload::StatusEffectAdded { .. } => "status_effect_added",
            EventPayload::StatusEffectRemoved { .. } => "status_effect_removed",
            EventPayload::DemonBluffsSet { .. } => "demon_bluffs_set",
        }
    }
}

/// A persisted event envelope wrapping a typed payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub game_id: Uuid,
    pub sequence: i64,
    pub timestamp: DateTime<Utc>,
    pub payload: EventPayload,
}

impl GameEvent {
    pub fn new(game_id: Uuid, sequence: i64, payload: EventPayload) -> Self {
        GameEvent {
            id: Uuid::new_v4(),
            game_id,
            sequence,
            timestamp: Utc::now(),
            payload,
        }
    }
}

/// Return a human-readable description of an event payload.
pub fn describe_event(payload: &EventPayload) -> String {
    match payload {
        EventPayload::GameCreated { script_name } => {
            format!("Game created with script {script_name}")
        }
        EventPayload::NightStepSet { step } => format!("Night step set to {step}"),
        EventPayload::FirstNightSet { is_first_night } => {
            if *is_first_night {
                "Set to first night".to_string()
            } else {
                "Set to subsequent night".to_string()
            }
        }
        EventPayload::RoleVisibilitySet {
            should_reveal_roles,
        } => {
            if *should_reveal_roles {
                "Roles revealed".to_string()
            } else {
                "Roles hidden".to_string()
            }
        }
        EventPayload::RoleIncluded { name } => format!("Added {name} to role pool"),
        EventPayload::RolesIncluded { names } => {
            format!("Added {} roles: {}", names.len(), names.join(", "))
        }
        EventPayload::RoleRemoved { name } => format!("Removed {name} from role pool"),
        EventPayload::PlayerAdded {
            player_name,
            character_name,
            ..
        } => format!("{player_name} joined as {character_name}"),
        EventPayload::TravelerAdded {
            player_name,
            traveler_name,
            ..
        } => format!("{player_name} joined as traveler {traveler_name}"),
        EventPayload::PlayerRemoved { player_name } => format!("{player_name} removed from game"),
        EventPayload::PlayerRenamed { old_name, new_name } => {
            format!("{old_name} renamed to {new_name}")
        }
        EventPayload::CharactersSwapped { name1, name2 } => {
            format!("{name1} and {name2} swapped characters")
        }
        EventPayload::PlayerAliveSet {
            player_name,
            is_alive,
            cleared_effects,
        } => {
            let action = if *is_alive { "resurrected" } else { "died" };
            let mut desc = format!("{player_name} {action}");
            if !cleared_effects.is_empty() {
                let effects = cleared_effects
                    .iter()
                    .map(|(n, e)| format!("{e} from {n}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                desc.push_str(&format!(" (cleared: {effects})"));
            }
            desc
        }
        EventPayload::DeadVoteUsedSet {
            player_name,
            has_used_dead_vote,
        } => {
            let verb = if *has_used_dead_vote {
                "used"
            } else {
                "recovered"
            };
            format!("{player_name} {verb} their dead vote")
        }
        EventPayload::PlayerAlignmentSet {
            player_name,
            alignment,
        } => {
            format!("{player_name} alignment changed to {alignment}")
        }
        EventPayload::StatusEffectAdded {
            player_name,
            effect,
        } => format!("{player_name} gained {effect}"),
        EventPayload::StatusEffectRemoved {
            player_name,
            effect,
        } => format!("{player_name} lost {effect}"),
        EventPayload::DemonBluffsSet { bluffs } => {
            if bluffs.is_empty() {
                "Demon bluffs cleared".to_string()
            } else {
                format!("Demon bluffs set to {}", bluffs.join(", "))
            }
        }
    }
}
