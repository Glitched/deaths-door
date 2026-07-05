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

/// Duration of a sound's audio file, if it exists and can be decoded. Used as
/// the length hint for the lighting/overlay effect that accompanies it.
pub fn duration(sound: SoundName) -> Option<std::time::Duration> {
    use rodio::Source;
    let path = sound_path(sound)?;
    let file = File::open(&path).ok()?;
    let decoder = rodio::Decoder::new(BufReader::new(file)).ok()?;
    decoder.total_duration()
}

/// Whether server-wide audio mute is on (checked at each play, so tests and
/// long-running servers can toggle it via the environment).
fn muted() -> bool {
    std::env::var("DEATHS_DOOR_MUTE")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
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

    /// Play a sound effect. Blocks only until playback has *started* (file
    /// opened, decoded, audio device acquired), then plays out asynchronously.
    ///
    /// Returns `Err` if the sound can't be started — file missing, undecodable
    /// (e.g. an unsupported WAV variant), or no audio output device — so callers
    /// can surface the failure instead of silently dropping it. Because it waits
    /// for the start signal, call it from a blocking context (`spawn_blocking`).
    ///
    /// `DEATHS_DOOR_MUTE=1|true` mutes the whole server: playback is skipped
    /// (successfully) while scene lighting, overlay visuals, and effect
    /// durations stay exactly the same. Used by tests, and handy for running
    /// lights-only setups.
    pub fn play(&self, sound: SoundName) -> Result<(), String> {
        if muted() {
            return Ok(());
        }
        let Some(path) = sound_path(sound) else {
            return Err(format!("sound file not found for '{}'", sound.value()));
        };

        // rodio's output stream is `!Send`, so setup + playback must live on one
        // thread. Set up there, report whether playback started, then play out.
        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            let started = (|| -> Result<(rodio::OutputStream, rodio::Sink), String> {
                let (stream, handle) = rodio::OutputStream::try_default()
                    .map_err(|e| format!("no audio output device: {e}"))?;
                let sink = rodio::Sink::try_new(&handle)
                    .map_err(|e| format!("could not create audio sink: {e}"))?;
                let file = File::open(&path)
                    .map_err(|e| format!("could not open {}: {e}", path.display()))?;
                let source = rodio::Decoder::new(BufReader::new(file))
                    .map_err(|e| format!("could not decode {}: {e}", path.display()))?;
                sink.append(source);
                Ok((stream, sink))
            })();

            match started {
                Ok((stream, sink)) => {
                    let _ = tx.send(Ok(()));
                    sink.sleep_until_end();
                    drop(stream); // keep the output stream alive until playback ends
                }
                Err(e) => {
                    tracing::warn!("sound playback failed: {e}");
                    let _ = tx.send(Err(e));
                }
            }
        });

        // Wait for the start result (decoding a short clip takes milliseconds).
        rx.recv()
            .unwrap_or_else(|_| Err("audio thread terminated before reporting".to_string()))
    }
}
