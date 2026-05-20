//! A Blood on the Clocktower script (edition): its character pool, travelers,
//! and ordered night steps.

use serde::Deserialize;

use crate::character::Character;
use crate::night_step::NightStep;

#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub characters: Vec<Character>,
    pub travelers: Vec<Character>,
    pub first_night_steps: Vec<NightStep>,
    pub other_night_steps: Vec<NightStep>,
}

impl Script {
    /// Get a character by name (case-insensitive).
    pub fn get_character(&self, name: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.is_named(name))
    }

    /// Whether the named character is in this script.
    pub fn has_character(&self, name: &str) -> bool {
        self.get_character(name).is_some()
    }

    /// Get a traveler by exact name.
    pub fn get_traveler(&self, name: &str) -> Option<&Character> {
        self.travelers.iter().find(|t| t.name == name)
    }
}
