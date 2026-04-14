import { useEffect, useRef, useState } from "react";
import { apiFetch } from "@/api/client";
import type { TimerState } from "@/api/types";

export function useTimer(intervalMs = 1000): TimerState | null {
  const [timer, setTimer] = useState<TimerState | null>(null);
  const lastGood = useRef<TimerState | null>(null);

  useEffect(() => {
    let active = true;

    const poll = async () => {
      try {
        const data = await apiFetch<TimerState>("/timer/fetch");
        if (active) {
          lastGood.current = data;
          setTimer(data);
        }
      } catch {
        // Hold last known value — overlay must never show errors
        if (active && lastGood.current) {
          setTimer(lastGood.current);
        }
      }
    };

    poll();
    const id = setInterval(poll, intervalMs);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [intervalMs]);

  return timer;
}
