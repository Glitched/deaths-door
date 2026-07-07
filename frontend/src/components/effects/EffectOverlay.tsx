import type { ActiveEffect } from "@/api/types";

// Full-screen visual for each scene; keyframes live in index.css. The
// animation duration is set inline from the effect so visuals end with their
// sound (a 1.5s death sting flashes fast; the 13s music box fades slowly).
const SCENE_CLASSES: Record<string, string> = {
  death: "effect-death",
  drama: "effect-drama",
  goodnight: "effect-goodnight",
  morning: "effect-morning",
  reveal: "effect-reveal",
  alarm: "effect-alarm",
  sad_trumpet: "effect-sad-trumpet",
  wilhelm: "effect-wilhelm",
  fog: "effect-fog",
};

// Purely derived from the SSE stream: the backend clears active_effect (and
// pushes a frame) when the effect's duration elapses, which unmounts this.
// The keyframes also end at opacity 0, so a missed frame can't leave a
// lingering wash on the projector.
export function EffectOverlay({ effect }: { effect: ActiveEffect | null }) {
  if (!effect || !(effect.scene in SCENE_CLASSES)) return null;

  return (
    <div
      // Remount on every trigger so repeating a scene restarts its animation.
      key={effect.id}
      className={`fixed inset-0 pointer-events-none z-50 ${SCENE_CLASSES[effect.scene]}`}
      style={{ animationDuration: `${effect.duration_ms}ms` }}
    />
  );
}
