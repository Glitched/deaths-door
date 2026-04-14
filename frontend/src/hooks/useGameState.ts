import { useEffect, useRef, useState } from "react";
import { apiFetch } from "@/api/client";
import type { GameState } from "@/api/types";

export function useGameState(intervalMs = 3000): GameState | null {
  const [state, setState] = useState<GameState | null>(null);
  const lastGood = useRef<GameState | null>(null);

  useEffect(() => {
    let active = true;

    const poll = async () => {
      try {
        const data = await apiFetch<GameState>("/game/state");
        if (active) {
          lastGood.current = data;
          setState(data);
        }
      } catch {
        if (active && lastGood.current) {
          setState(lastGood.current);
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

  return state;
}
