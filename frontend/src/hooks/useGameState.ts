import { useEffect, useRef, useState } from "react";
import type { GameState } from "@/api/types";

const STREAM_URL = import.meta.env.DEV ? "/api/game/stream" : "/game/stream";

export type ConnectionStatus = "connecting" | "connected" | "disconnected";

export interface GameStateResult {
  state: GameState | null;
  status: ConnectionStatus;
}

export function useGameState(): GameStateResult {
  const [state, setState] = useState<GameState | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>("connecting");
  const lastGood = useRef<GameState | null>(null);

  useEffect(() => {
    let active = true;
    let eventSource: EventSource | null = null;

    function connect() {
      eventSource = new EventSource(STREAM_URL);

      eventSource.onopen = () => {
        if (active) setStatus("connected");
      };

      eventSource.onmessage = (event) => {
        if (!active) return;
        try {
          const data = JSON.parse(event.data) as GameState;
          lastGood.current = data;
          setState(data);
          setStatus("connected");
        } catch {
          // Ignore malformed messages
        }
      };

      eventSource.onerror = () => {
        if (!active) return;
        setStatus("disconnected");
        if (lastGood.current) {
          setState(lastGood.current);
        }
      };
    }

    connect();

    return () => {
      active = false;
      eventSource?.close();
    };
  }, []);

  return { state, status };
}
