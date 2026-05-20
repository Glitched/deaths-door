//! Sound effect playback.
//!
//! The Python version uses pygame; here we use `rodio`. Playback is
//! fire-and-forget on a dedicated thread (rodio's output stream is not `Send`),
//! and degrades gracefully when no audio device is available.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundName {
    // Player death
    Death,
    Wilhelm,
    // Morning
    Rooster,
    Alarm,
    Timer,
    // Goodnight
    MusicBox,
    // Reveal
    Drumroll,
    Drama,
    SadTrumpet,
}

impl SoundName {
    pub fn value(&self) -> &'static str {
        match self {
            SoundName::Death => "death",
            SoundName::Wilhelm => "wilhelm",
            SoundName::Rooster => "rooster",
            SoundName::Alarm => "alarm",
            SoundName::Timer => "timer",
            SoundName::MusicBox => "music_box",
            SoundName::Drumroll => "drumroll",
            SoundName::Drama => "drama",
            SoundName::SadTrumpet => "sad_trumpet",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<SoundName> {
        let lower = name.to_lowercase();
        [
            SoundName::Death,
            SoundName::Wilhelm,
            SoundName::Rooster,
            SoundName::Alarm,
            SoundName::Timer,
            SoundName::MusicBox,
            SoundName::Drumroll,
            SoundName::Drama,
            SoundName::SadTrumpet,
        ]
        .into_iter()
        .find(|s| s.value() == lower)
    }
}

/// Sounds organized by category (matches Python `sounds` dict).
pub fn sounds_by_category() -> Vec<(&'static str, Vec<SoundName>)> {
    vec![
        (
            "morning",
            vec![SoundName::Rooster, SoundName::Alarm, SoundName::Timer],
        ),
        ("goodnight", vec![SoundName::MusicBox]),
        (
            "reveal",
            vec![SoundName::Drumroll, SoundName::Drama, SoundName::SadTrumpet],
        ),
        ("death", vec![SoundName::Death, SoundName::Wilhelm]),
    ]
}

/// Locate the wav file for a sound, checking a few candidate asset locations.
fn sound_path(sound: SoundName) -> Option<PathBuf> {
    let file = format!("{}.wav", sound.value());
    let candidates = [
        // Relative to the current working directory (when run via `cargo run`).
        PathBuf::from("assets/sound_fx").join(&file),
        // Relative to the crate root (when run from elsewhere).
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/sound_fx")
            .join(&file),
    ];
    candidates.into_iter().find(|p| p.exists())
}

#[derive(Clone, Default)]
pub struct SoundFx;

impl SoundFx {
    pub fn new() -> Self {
        SoundFx
    }

    /// Play a sound effect (fire-and-forget). Returns `Err` only if the sound
    /// file can't be located; audio-device failures are logged and ignored.
    pub fn play(&self, sound: SoundName) -> Result<(), String> {
        let Some(path) = sound_path(sound) else {
            return Err(format!("Sound file not found for {}", sound.value()));
        };

        std::thread::spawn(move || {
            let (_stream, handle) = match rodio::OutputStream::try_default() {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("No audio output device available: {e}");
                    return;
                }
            };
            let sink = match rodio::Sink::try_new(&handle) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to create audio sink: {e}");
                    return;
                }
            };
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Failed to open sound file {:?}: {e}", path);
                    return;
                }
            };
            match rodio::Decoder::new(BufReader::new(file)) {
                Ok(source) => {
                    sink.append(source);
                    sink.sleep_until_end();
                }
                Err(e) => tracing::warn!("Failed to decode sound file {:?}: {e}", path),
            }
        });

        Ok(())
    }
}
