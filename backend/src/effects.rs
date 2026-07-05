//! Scene effects: coordinated lighting sequences, sound, and overlay cues.
//!
//! Triggering a scene plays its paired sound, runs a timed lighting cue
//! sequence on the DMX fixtures, and surfaces an [`ActiveEffect`] in SSE frames
//! so the projector overlay can run a matching visual. The effect's length
//! follows the paired sound's audio duration (a 1.5s death sting gives a 1.5s
//! effect; the 13s music box gives a long goodnight fade).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use utoipa::ToSchema;

use crate::game_manager::GameManager;
use crate::lighting::{colors, LightingManager, LightingScene};
use crate::lock::LockExt;
use crate::sound::{self, SoundFx, SoundName};

/// Sequence length when a scene has no paired sound (or its file is missing).
const DEFAULT_DURATION: Duration = Duration::from_secs(3);

/// The sound that accompanies each scene, if any.
pub fn paired_sound(scene: LightingScene) -> Option<SoundName> {
    match scene {
        LightingScene::Death => Some(SoundName::Death),
        LightingScene::Drama => Some(SoundName::Drama),
        LightingScene::Goodnight => Some(SoundName::MusicBox),
        LightingScene::Morning => Some(SoundName::Rooster),
        LightingScene::Reveal => Some(SoundName::Drumroll),
        LightingScene::Blackout | LightingScene::Spotlight | LightingScene::Fog => None,
    }
}

/// A scene effect currently playing, included in `/game/state` and SSE frames.
/// The overlay replays its visual whenever `id` changes.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ActiveEffect {
    /// Monotonically increasing trigger id.
    pub id: u64,
    /// Scene name, e.g. "death".
    pub scene: String,
    /// Total effect length in milliseconds.
    pub duration_ms: u64,
}

/// One step of a scene's lighting sequence.
struct Cue {
    /// Offset from the start of the effect.
    at: Duration,
    action: CueAction,
}

enum CueAction {
    /// (light1 color, light2 color), dimmer, strobe for both moving heads.
    Heads((i64, i64), i64, i64),
    /// Fog machine output (0-255).
    Fog(i64),
    Blackout,
}

/// Build the cue list for a scene, scaled to the effect's total duration.
fn build_cues(scene: LightingScene, total: Duration) -> Vec<Cue> {
    // Cue at a fraction of the total duration.
    let at = |frac: f64, action: CueAction| Cue {
        at: total.mul_f64(frac),
        action,
    };
    let both = |color: i64| (color, color);

    match scene {
        // Hard red strobe burst with a fog puff, settling into a deep red wash.
        LightingScene::Death => vec![
            at(0.0, CueAction::Heads(both(colors::RED), 255, 200)),
            at(0.0, CueAction::Fog(200)),
            at(0.5, CueAction::Heads(both(colors::RED), 255, 0)),
            at(0.5, CueAction::Fog(0)),
        ],
        // Alternating color pulses with strobe, settling bright.
        LightingScene::Drama => vec![
            at(
                0.0,
                CueAction::Heads((colors::DRAMA_A, colors::DRAMA_B), 200, 50),
            ),
            at(
                0.25,
                CueAction::Heads((colors::DRAMA_B, colors::DRAMA_A), 230, 80),
            ),
            at(
                0.5,
                CueAction::Heads((colors::DRAMA_A, colors::DRAMA_B), 200, 50),
            ),
            at(
                0.75,
                CueAction::Heads((colors::DRAMA_B, colors::DRAMA_A), 230, 80),
            ),
            at(1.0, CueAction::Heads(both(colors::DRAMA_B), 220, 0)),
        ],
        // Slow fade down to a dim blue night-light.
        LightingScene::Goodnight => vec![
            at(0.0, CueAction::Heads(both(colors::BLUE), 200, 0)),
            at(0.25, CueAction::Heads(both(colors::BLUE), 160, 0)),
            at(0.5, CueAction::Heads(both(colors::BLUE), 120, 0)),
            at(0.75, CueAction::Heads(both(colors::BLUE), 80, 0)),
            at(1.0, CueAction::Heads(both(colors::BLUE), 50, 0)),
        ],
        // Sunrise: fade up from dim warm light to full white.
        LightingScene::Morning => vec![
            at(0.0, CueAction::Heads(both(colors::WHITE), 40, 0)),
            at(0.33, CueAction::Heads(both(colors::WHITE), 110, 0)),
            at(0.66, CueAction::Heads(both(colors::WHITE), 190, 0)),
            at(1.0, CueAction::Heads(both(colors::WHITE), 255, 0)),
        ],
        // Building tension flicker timed to the drumroll, climax at the end.
        LightingScene::Reveal => vec![
            at(0.0, CueAction::Heads(both(colors::RED), 60, 0)),
            at(0.3, CueAction::Heads(both(colors::RED), 100, 60)),
            at(0.6, CueAction::Heads(both(colors::RED), 140, 120)),
            at(0.85, CueAction::Heads(both(colors::RED), 180, 200)),
            at(0.85, CueAction::Fog(255)),
            at(1.0, CueAction::Heads(both(colors::AUTO_CYCLE), 255, 0)),
            at(1.0, CueAction::Fog(0)),
        ],
        // A burst from the fog machine, then off.
        LightingScene::Fog => vec![at(0.0, CueAction::Fog(255)), at(1.0, CueAction::Fog(0))],
        LightingScene::Blackout => vec![at(0.0, CueAction::Blackout)],
        // Spotlight is positional; it has its own endpoint and no scene sequence.
        LightingScene::Spotlight => vec![],
    }
}

pub struct EffectsEngine {
    lighting: Arc<LightingManager>,
    manager: Arc<GameManager>,
    sound: SoundFx,
    /// The currently-playing effect and when it started.
    active: Mutex<Option<(ActiveEffect, Instant)>>,
    /// Trigger counter; doubles as a cancellation token for running cue tasks.
    generation: AtomicU64,
}

impl EffectsEngine {
    pub fn new(lighting: Arc<LightingManager>, manager: Arc<GameManager>) -> Self {
        EffectsEngine {
            lighting,
            manager,
            sound: SoundFx::new(),
            active: Mutex::new(None),
            generation: AtomicU64::new(0),
        }
    }

    /// The effect currently playing, or `None` once its duration has elapsed.
    pub fn active(&self) -> Option<ActiveEffect> {
        let active = self.active.lock_recover();
        active.as_ref().and_then(|(effect, started)| {
            let still_playing = (started.elapsed().as_millis() as u64) < effect.duration_ms;
            still_playing.then(|| effect.clone())
        })
    }

    /// Trigger a scene: paired sound (unless `silent`), lighting cue sequence,
    /// and an SSE notification so the overlay can run its visual. Any effect
    /// already playing is superseded.
    pub async fn trigger(self: &Arc<Self>, scene: LightingScene, silent: bool) -> ActiveEffect {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // A superseded scene's fog-off cue never runs, so a mid-scene trigger
        // could otherwise leave the fog machine going indefinitely. Every new
        // trigger starts from fog-off; scenes that want fog cue it themselves.
        self.lighting.set_fog(0);

        // The paired sound's audio length sets the effect length (even when
        // silenced, so the effect looks the same); instant scenes have none.
        let paired = paired_sound(scene);
        let duration = match scene {
            LightingScene::Blackout => Duration::ZERO,
            _ => paired.and_then(sound::duration).unwrap_or(DEFAULT_DURATION),
        };
        let sound = if silent { None } else { paired };

        let effect = ActiveEffect {
            id: generation,
            scene: scene.value().to_string(),
            duration_ms: duration.as_millis() as u64,
        };
        *self.active.lock_recover() = Some((effect.clone(), Instant::now()));

        // Sound is fire-and-forget, off the async worker (rodio setup blocks).
        if let Some(sound) = sound {
            let fx = self.sound.clone();
            tokio::task::spawn_blocking(move || {
                let _ = fx.play(sound);
            });
        }

        // Run the lighting cues in the background; a newer trigger cancels.
        let this = Arc::clone(self);
        tokio::spawn(async move {
            this.run_cues(scene, duration, generation).await;
        });

        // Push a frame so overlay/console subscribers see the effect start now.
        self.manager.notify_current().await;

        effect
    }

    async fn run_cues(&self, scene: LightingScene, total: Duration, generation: u64) {
        let start = Instant::now();
        for cue in build_cues(scene, total) {
            let elapsed = start.elapsed();
            if cue.at > elapsed {
                tokio::time::sleep(cue.at - elapsed).await;
            }
            // Superseded by a newer trigger; its cues own the fixtures now.
            if self.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            self.apply(&cue.action);
        }

        // Hold until the full duration has elapsed (cue lists may end early),
        // then clear the active effect and let subscribers know.
        let elapsed = start.elapsed();
        if total > elapsed {
            tokio::time::sleep(total - elapsed).await;
        }
        {
            let mut active = self.active.lock_recover();
            if active.as_ref().is_some_and(|(e, _)| e.id == generation) {
                *active = None;
            } else {
                return; // a newer effect took over; don't notify
            }
        }
        self.manager.notify_current().await;
    }

    fn apply(&self, action: &CueAction) {
        match action {
            CueAction::Heads(colors, dimmer, strobe) => {
                self.lighting.set_heads(*colors, *dimmer, *strobe)
            }
            CueAction::Fog(intensity) => self.lighting.set_fog(*intensity),
            CueAction::Blackout => self.lighting.blackout(),
        }
    }
}
