//! Immutable game state models for event sourcing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::alignment::Alignment;
use crate::character::{Character, CharacterOut};
use crate::night_step::NightStep;
use crate::player::PlayerOut;
use crate::script::Script;
use crate::scripts::get_script_by_name;
use crate::status_effect::StatusEffectOut;

/// Day/night phase of the game. Nights are for character abilities (and
/// secret deaths); days are for announcements, nominations, and executions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Day,
    Night,
}

/// What a resolved nomination did to the chopping block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NominationOutcome {
    /// Met the execution threshold and beat the current block: the nominee is
    /// now on the block.
    OnTheBlock,
    /// Exactly tied the votes of the player on the block: nobody is on the
    /// block now (a tie means no one is executed).
    TieBlockEmptied,
    /// Not enough votes to change anything.
    BlockUnchanged,
}

/// A nomination resolved today. The list clears when night begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Nomination {
    /// The nominated player.
    pub player_name: String,
    /// Final vote count (usually `voters.len()`, but the storyteller may
    /// record a different total, e.g. a Bureaucrat's triple vote).
    pub votes: u32,
    /// Everyone who voted.
    pub voters: Vec<String>,
    pub outcome: NominationOutcome,
}

/// The player currently up for execution (the "chopping block").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChoppingBlock {
    pub player_name: String,
    /// Vote count that put the player on the block, if the storyteller chose to
    /// record it.
    #[serde(default)]
    pub votes: Option<u32>,
}

/// Immutable snapshot of a player's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub name: String,
    pub character_name: String,
    pub alignment: String,
    pub is_alive: bool,
    pub has_used_dead_vote: bool,
    pub status_effects: Vec<String>,
}

impl PlayerState {
    pub fn new(name: String, character_name: String, alignment: String) -> Self {
        PlayerState {
            name,
            character_name,
            alignment,
            is_alive: true,
            has_used_dead_vote: false,
            status_effects: Vec::new(),
        }
    }
}

/// Immutable snapshot of the entire game state, derived from events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub game_id: Uuid,
    pub script_name: String,
    pub included_role_names: Vec<String>,
    pub players: Vec<PlayerState>,
    pub should_reveal_roles: bool,
    pub current_night_step: String,
    pub is_first_night: bool,
    /// Up to 3 character names recorded as the Demon's bluffs (good characters
    /// shown as "not in play"). Stored as names; resolved via [`Self::get_demon_bluffs`].
    #[serde(default)]
    pub demon_bluffs: Vec<String>,
    /// The player currently on the chopping block, if any. Cleared automatically
    /// when that player dies or is removed, or when night begins.
    #[serde(default)]
    pub chopping_block: Option<ChoppingBlock>,
    /// Whether it is currently day or night. Kept in sync with the night-step
    /// bookmark ("Dawn" = day), and driven by `DayBegan`/`NightBegan` events.
    #[serde(default = "default_phase")]
    pub phase: Phase,
    /// How many days have begun. 0 during setup and the first night.
    #[serde(default)]
    pub day_number: u32,
    /// Players who died during the night and haven't been announced yet.
    /// Filled by night deaths; drained by `DeathAnnounced` (or revival/removal).
    #[serde(default)]
    pub deaths_to_announce: Vec<String>,
    /// Nominations resolved today. Cleared when night begins.
    #[serde(default)]
    pub nominations_today: Vec<Nomination>,
    /// The vote count at which nominations tied, emptying the chopping block.
    /// A tie stands for the rest of the day: later nominations must beat this
    /// count, and matching it merely re-ties. Mutually exclusive with an
    /// occupied block; cleared on day/night transitions and manual block changes.
    #[serde(default)]
    pub tied_votes: Option<u32>,
    /// Which team won ("good"/"evil"), once the storyteller has ended the game
    /// via `POST /game/end`. `None` while the game is running.
    #[serde(default)]
    pub winner: Option<String>,
    pub version: i64,
}

fn default_phase() -> Phase {
    Phase::Night
}

impl GameState {
    /// A fresh, empty game state (before `GameCreated` is applied).
    pub fn initial(game_id: Uuid, script_name: impl Into<String>) -> Self {
        GameState {
            game_id,
            script_name: script_name.into(),
            included_role_names: Vec::new(),
            players: Vec::new(),
            should_reveal_roles: false,
            current_night_step: "Dusk".to_string(),
            is_first_night: true,
            demon_bluffs: Vec::new(),
            chopping_block: None,
            // Games open on the first night (setup happens before day 1).
            phase: Phase::Night,
            day_number: 0,
            deaths_to_announce: Vec::new(),
            nominations_today: Vec::new(),
            tied_votes: None,
            winner: None,
            version: 0,
        }
    }

    pub fn is_day(&self) -> bool {
        self.phase == Phase::Day
    }

    // --- Script lookup ---

    /// Get the [`Script`] for this game, if its name is known.
    pub fn get_script(&self) -> Option<&'static Script> {
        get_script_by_name(&self.script_name)
    }

    pub fn get_character(&self, name: &str) -> Option<&'static Character> {
        self.get_script().and_then(|s| s.get_character(name))
    }

    // --- Player lookups ---

    pub fn get_player(&self, name: &str) -> Option<&PlayerState> {
        self.players.iter().find(|p| p.name == name)
    }

    fn normalize(s: &str) -> String {
        s.to_lowercase().trim().to_string()
    }

    pub fn has_living_character_named(&self, character_name: &str) -> bool {
        let normalized = Self::normalize(character_name);
        self.players
            .iter()
            .any(|p| Self::normalize(&p.character_name) == normalized && p.is_alive)
    }

    pub fn has_dead_character_named(&self, character_name: &str) -> bool {
        let normalized = Self::normalize(character_name);
        self.players
            .iter()
            .any(|p| Self::normalize(&p.character_name) == normalized && !p.is_alive)
    }

    // --- Derived vote info ---

    pub fn living_player_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_alive).count()
    }

    /// Votes needed to execute (>= 50% of living players, rounded up).
    pub fn execution_threshold(&self) -> usize {
        self.living_player_count().div_ceil(2)
    }

    pub fn get_dead_players_with_vote(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|p| !p.is_alive && !p.has_used_dead_vote)
            .map(|p| p.name.clone())
            .collect()
    }

    /// Everyone currently allowed to vote: living players, plus dead players
    /// who still hold their one dead vote.
    pub fn eligible_voters(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|p| p.is_alive || !p.has_used_dead_vote)
            .map(|p| p.name.clone())
            .collect()
    }

    /// Whether a player has already been nominated today (each player may be
    /// nominated at most once per day).
    pub fn was_nominated_today(&self, name: &str) -> bool {
        self.nominations_today.iter().any(|n| n.player_name == name)
    }

    /// Votes a new nomination needs to put its nominee on the block: the base
    /// execution threshold, raised to one more than the standing count — the
    /// current block's votes, or the count earlier nominations tied at.
    pub fn votes_to_take_block(&self) -> u32 {
        let threshold = self.execution_threshold() as u32;
        let standing = self
            .chopping_block
            .as_ref()
            .and_then(|b| b.votes)
            .or(self.tied_votes);
        match standing {
            Some(votes) => threshold.max(votes + 1),
            None => threshold,
        }
    }

    /// A heads-up that the game may be over. The storyteller confirms with
    /// `POST /game/end` — abilities like the Scarlet Woman can keep a game
    /// going, so this is a hint, never a ruling.
    pub fn game_over_hint(&self) -> Option<String> {
        if self.winner.is_some() {
            return None;
        }
        let demons: Vec<&PlayerState> = self
            .players
            .iter()
            .filter(|p| {
                self.get_character(&p.character_name)
                    .is_some_and(|c| c.category == crate::character_type::CharacterType::Demon)
            })
            .collect();
        if demons.is_empty() {
            return None; // no demon in play (yet)
        }
        if demons.iter().all(|p| !p.is_alive) {
            Some("The demon is dead — good may have won (unless a Scarlet Woman inherits).".into())
        } else if self.living_player_count() <= 2 {
            Some("Only two players live and the demon stands — evil may have won.".into())
        } else {
            None
        }
    }

    // --- Night steps ---

    pub fn get_night_steps(&self) -> Vec<NightStep> {
        let Some(script) = self.get_script() else {
            return Vec::new();
        };
        let steps = if self.is_first_night {
            &script.first_night_steps
        } else {
            &script.other_night_steps
        };
        self.filter_active_night_steps(steps)
    }

    fn filter_active_night_steps(&self, steps: &[NightStep]) -> Vec<NightStep> {
        let mut result = Vec::new();
        for step in steps {
            let show = step.always_show
                || (step.show_when_dead && self.has_dead_character_named(&step.name))
                || self.has_living_character_named(&step.name);
            if show {
                result.push(step.clone());
            }
        }
        result
    }

    // --- Status effects ---

    pub fn get_status_effects(&self) -> Vec<StatusEffectOut> {
        let Some(script) = self.get_script() else {
            return Vec::new();
        };
        let mut effects: Vec<StatusEffectOut> = Vec::new();
        for player in &self.players {
            if let Some(character) = script.get_character(&player.character_name) {
                effects.extend(character.get_status_effects_out());
            }
        }
        effects.sort_by(|a, b| a.character_name.cmp(&b.character_name));
        effects
    }

    // --- Unclaimed travelers ---

    pub fn get_unclaimed_travelers(&self) -> Vec<&'static Character> {
        let Some(script) = self.get_script() else {
            return Vec::new();
        };
        let claimed: Vec<&str> = self
            .players
            .iter()
            .map(|p| p.character_name.as_str())
            .collect();
        script
            .travelers
            .iter()
            .filter(|t| !claimed.contains(&t.name.as_str()))
            .collect()
    }

    // --- Included roles as Character objects ---

    pub fn get_included_roles(&self) -> Vec<&'static Character> {
        let Some(script) = self.get_script() else {
            return Vec::new();
        };
        self.included_role_names
            .iter()
            .filter_map(|name| script.get_character(name))
            .collect()
    }

    // --- Demon bluffs ---

    /// The recorded demon bluffs as [`Character`] objects, resolved against the
    /// script. Names that don't resolve (e.g. unknown script) are dropped.
    pub fn get_demon_bluffs(&self) -> Vec<&'static Character> {
        let Some(script) = self.get_script() else {
            return Vec::new();
        };
        self.demon_bluffs
            .iter()
            .filter_map(|name| script.get_character(name))
            .collect()
    }

    // --- Immutable update helper ---

    /// Return a new state with one player's fields updated via the closure.
    pub fn replace_player(
        &self,
        player_name: &str,
        update: impl Fn(&mut PlayerState),
    ) -> GameState {
        let mut next = self.clone();
        if let Some(p) = next.players.iter_mut().find(|p| p.name == player_name) {
            update(p);
        }
        next
    }
}

/// Persistent status effects to clear when a character dies (death cleanup).
fn character_persistent_effects(character_name: &str) -> &'static [&'static str] {
    match character_name {
        "Poisoner" => &["Poisoned"],
        "Monk" => &["Safe"],
        "Butler" => &["Butler's Master"],
        _ => &[],
    }
}

/// Cascading status-effect removals when a player dies.
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

/// Convert a [`PlayerState`] to the API response model.
pub fn player_state_to_out(player: &PlayerState, script: &Script) -> PlayerOut {
    // Search both characters and travelers.
    let character = script.get_character(&player.character_name).or_else(|| {
        script
            .travelers
            .iter()
            .find(|t| t.name == player.character_name)
    });

    let character_out = match character {
        Some(c) => c.to_out(),
        // Invariant: a player's character is always in the script. Fall back to
        // a minimal model rather than panicking inside a request handler.
        None => CharacterOut {
            name: player.character_name.clone(),
            description: String::new(),
            icon_path: format!(
                "{}.png",
                player.character_name.to_lowercase().replace(' ', "")
            ),
            alignment: Alignment::from_str(&player.alignment).unwrap_or(Alignment::Unknown),
            category: crate::character_type::CharacterType::Townsfolk,
        },
    };

    PlayerOut {
        name: player.name.clone(),
        character: character_out,
        alignment: Alignment::from_str(&player.alignment).unwrap_or(Alignment::Unknown),
        is_alive: player.is_alive,
        has_used_dead_vote: player.has_used_dead_vote,
        status_effects: player.status_effects.clone(),
    }
}

/// Convert included role names to [`CharacterOut`] API models.
pub fn game_state_to_included_role_outs(state: &GameState) -> Vec<CharacterOut> {
    state
        .get_included_roles()
        .iter()
        .map(|c| c.to_out())
        .collect()
}
