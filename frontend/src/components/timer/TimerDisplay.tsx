import type { TimerState } from "@/api/types";

function formatTimer(totalSeconds: number): string {
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

// Timer now arrives on the game-state SSE stream (pushed on every change), so
// it's passed in from the page rather than separately polled.
export function TimerDisplay({ timer }: { timer: TimerState | null }) {
  if (!timer) return null;

  return (
    <div className="flex justify-center pt-2">
      <span
        className="text-[240px] leading-none bg-gradient-to-b from-[#FF0000] to-[#690000] bg-clip-text text-transparent"
        style={{ fontFamily: "var(--font-timer)" }}
      >
        {formatTimer(timer.seconds)}
      </span>
    </div>
  );
}
