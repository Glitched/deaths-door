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

/// Step interval for interpolated dimmer fades (~30 updates/second; the DMX
/// transmitter streams at ~40Hz, so finer steps would be wasted).
const FADE_STEP: Duration = Duration::from_millis(33);

/// One step of a scene's lighting sequence.
struct Cue {
    /// Offset from the start of the effect.
    at: Duration,
    /// End of the cue's window. Equal to `at` for instant cues; later for
    /// fades, which interpolate across the window.
    until: Duration,
    action: CueAction,
}

enum CueAction {
    /// (light1 color, light2 color), dimmer, strobe for both moving heads.
    Heads((i64, i64), i64, i64),
    /// Smoothly ramp both heads' dimmer between two levels across the cue's
    /// window. Color/strobe are left alone (the color wheel is a physical
    /// wheel — "fading" it would sweep through every color in between).
    FadeDimmer {
        from: i64,
        to: i64,
    },
    /// Fog machine output (0-255).
    Fog(i64),
    Blackout,
}

/// Build the cue list for a scene, scaled to the effect's total duration.
fn build_cues(scene: LightingScene, total: Duration) -> Vec<Cue> {
    // Instant cue at a fraction of the total duration.
    let at = |frac: f64, action: CueAction| Cue {
        at: total.mul_f64(frac),
        until: total.mul_f64(frac),
        action,
    };
    // Dimmer fade spanning a fraction window of the total duration.
    let fade = |from_frac: f64, until_frac: f64, from: i64, to: i64| Cue {
        at: total.mul_f64(from_frac),
        until: total.mul_f64(until_frac),
        action: CueAction::FadeDimmer { from, to },
    };
    let both = |color: i64| (color, color);

    match scene {
        // Hard red strobe burst with a fog puff, then the strobe cuts and the
        // wash decays to a dim blood-red afterglow (rather than parking the
        // room at full-bright red).
        LightingScene::Death => vec![
            at(0.0, CueAction::Heads(both(colors::RED), 255, 200)),
            at(0.0, CueAction::Fog(200)),
            at(0.5, CueAction::Heads(both(colors::RED), 255, 0)),
            at(0.5, CueAction::Fog(0)),
            fade(0.5, 1.0, 255, 140),
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
        // A slow, continuous fade down to a dim blue night-light, tracking the
        // ~13s music box all the way to the last note.
        LightingScene::Goodnight => vec![
            at(0.0, CueAction::Heads(both(colors::BLUE), 200, 0)),
            fade(0.0, 1.0, 200, 50),
        ],
        // Sunrise: one smooth ramp from dim warm light to full white.
        LightingScene::Morning => vec![
            at(0.0, CueAction::Heads(both(colors::WHITE), 40, 0)),
            fade(0.0, 1.0, 40, 255),
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
        self.trigger_with_sound(scene, None, silent).await
    }

    /// Like [`Self::trigger`], but with the scene's paired sound replaced by
    /// `sound_override` (e.g. a Wilhelm-scream death). The effect length
    /// follows the sound that's actually chosen.
    pub async fn trigger_with_sound(
        self: &Arc<Self>,
        scene: LightingScene,
        sound_override: Option<SoundName>,
        silent: bool,
    ) -> ActiveEffect {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // A superseded scene's fog-off cue never runs, so a mid-scene trigger
        // could otherwise leave the fog machine going indefinitely. Every new
        // trigger starts from fog-off; scenes that want fog cue it themselves.
        self.lighting.set_fog(0);

        // The chosen sound's audio length sets the effect length (even when
        // silenced, so the effect looks the same); instant scenes have none.
        let paired = sound_override.or_else(|| paired_sound(scene));
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
            match &cue.action {
                CueAction::FadeDimmer { from, to } => {
                    if !self.fade_dimmer(&cue, *from, *to, start, generation).await {
                        return; // superseded mid-fade
                    }
                }
                action => self.apply(action),
            }
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

    /// Step both heads' dimmer linearly across the cue's window. Returns false
    /// if a newer trigger superseded this one mid-fade.
    async fn fade_dimmer(
        &self,
        cue: &Cue,
        from: i64,
        to: i64,
        start: Instant,
        generation: u64,
    ) -> bool {
        let window = cue.until.saturating_sub(cue.at);
        loop {
            if self.generation.load(Ordering::SeqCst) != generation {
                return false;
            }
            let progress = if window.is_zero() {
                1.0
            } else {
                let into = start.elapsed().saturating_sub(cue.at);
                (into.as_secs_f64() / window.as_secs_f64()).min(1.0)
            };
            let level = from + ((to - from) as f64 * progress).round() as i64;
            self.lighting.set_head_dimmers(level);
            if progress >= 1.0 {
                return true;
            }
            tokio::time::sleep(FADE_STEP).await;
        }
    }

    fn apply(&self, action: &CueAction) {
        match action {
            CueAction::Heads(colors, dimmer, strobe) => {
                self.lighting.set_heads(*colors, *dimmer, *strobe)
            }
            CueAction::FadeDimmer { .. } => unreachable!("fades are handled by run_cues"),
            CueAction::Fog(intensity) => self.lighting.set_fog(*intensity),
            CueAction::Blackout => self.lighting.blackout(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scene's cue list must stay inside the effect window, in order,
    /// with DMX-range fade levels — whatever duration the sound dictates.
    #[test]
    fn cue_lists_are_well_formed_for_every_scene() {
        for total in [
            Duration::ZERO,
            Duration::from_millis(1500),
            Duration::from_secs(13),
        ] {
            for scene in LightingScene::ALL {
                let mut last_start = Duration::ZERO;
                for cue in build_cues(scene, total) {
                    assert!(cue.at <= cue.until, "{scene:?}: inverted cue window");
                    assert!(cue.until <= total, "{scene:?}: cue past effect end");
                    assert!(cue.at >= last_start, "{scene:?}: cues out of order");
                    last_start = cue.at;
                    if let CueAction::FadeDimmer { from, to } = cue.action {
                        assert!(
                            (0..=255).contains(&from) && (0..=255).contains(&to),
                            "{scene:?}: fade level out of DMX range"
                        );
                    }
                }
            }
        }
    }
}
