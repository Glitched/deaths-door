import { useTimer } from "@/hooks/useTimer";

function formatTimer(totalSeconds: number): string {
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function TimerDisplay() {
  const timer = useTimer();

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
