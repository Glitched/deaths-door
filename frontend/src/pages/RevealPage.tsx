import { useCallback, useEffect, useState } from "react";

import { apiFetch } from "@/api/client";
import type { PlayerOut } from "@/api/types";

type RevealStep = "select" | "confirm" | "waiting" | "revealed";

const TITLES = {
  initial: "SEAL YOUR FATE",
  retry: "Don't fail me again.",
};

export function RevealPage() {
  useEffect(() => { document.title = "Seal Your Fate"; }, []);
  const [step, setStep] = useState<RevealStep>("select");
  const [names, setNames] = useState<string[]>([]);
  const [selectedName, setSelectedName] = useState("");
  const [title, setTitle] = useState(TITLES.initial);
  const [player, setPlayer] = useState<PlayerOut | null>(null);

  // Poll for player names
  useEffect(() => {
    const poll = async () => {
      try {
        const data = await apiFetch<string[]>("/players/names");
        setNames(data);
      } catch {
        // Silently retry
      }
    };
    poll();
    const id = setInterval(poll, 10000);
    return () => clearInterval(id);
  }, []);

  // Fetch role (waits for reveal on server side)
  const fetchRole = useCallback(async (name: string) => {
    setStep("waiting");
    const attempt = async (): Promise<void> => {
      try {
        const data = await apiFetch<PlayerOut>(`/players/name/${name}`);
        setPlayer(data);
        setStep("revealed");
      } catch {
        // Retry — server returns 408 on timeout, keep trying
        setTimeout(attempt, 500);
      }
    };
    await attempt();
  }, []);

  return (
    <div className="min-h-screen bg-black flex items-center justify-center">
      <div className="text-center max-w-md mx-auto px-6">
        {step === "select" && (
          <div className="animate-in fade-in duration-500">
            <h1
              className="text-4xl text-white mb-8 tracking-wider"
              style={{ fontFamily: "var(--font-timer)" }}
            >
              {title}
            </h1>
            <select
              className="w-full bg-zinc-900 border border-zinc-700 text-white text-lg px-4 py-3 rounded-lg appearance-none cursor-pointer focus:outline-none focus:border-red-700"
              value={selectedName}
              onChange={(e) => {
                setSelectedName(e.target.value);
                if (e.target.value) setStep("confirm");
              }}
            >
              <option value="">Identify yourself</option>
              {names.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </div>
        )}

        {step === "confirm" && (
          <div className="animate-in fade-in duration-500">
            <h1
              className="text-4xl text-white mb-8 tracking-wider"
              style={{ fontFamily: "var(--font-timer)" }}
            >
              {selectedName}
            </h1>
            <p className="text-zinc-400 mb-6 text-lg" style={{ fontFamily: "'Palatino Linotype', Palatino, Georgia, serif" }}>
              Confirm your identity.
            </p>
            <div className="flex gap-4 justify-center">
              <button
                className="px-6 py-3 bg-zinc-800 border border-zinc-600 text-white rounded-lg hover:bg-zinc-700 transition-colors"
                onClick={() => fetchRole(selectedName)}
              >
                Confirm
              </button>
              <button
                className="px-6 py-3 bg-zinc-900 border border-zinc-700 text-zinc-400 rounded-lg hover:bg-zinc-800 transition-colors"
                onClick={() => {
                  setStep("select");
                  setSelectedName("");
                  setTitle(TITLES.retry);
                }}
              >
                Go Back
              </button>
            </div>
          </div>
        )}

        {step === "waiting" && (
          <div>
            <h1
              className="text-3xl animate-[colorShift_2s_ease-in-out_infinite_alternate]"
              style={{ fontFamily: "var(--font-timer)" }}
            >
              Your role is...
            </h1>
          </div>
        )}

        {step === "revealed" && player && (
          <div className="animate-in fade-in duration-1000">
            {player.character.icon_path && (
              <img
                src={`/api/static/icons/${player.character.icon_path}`}
                alt=""
                className="w-48 h-48 mx-auto mb-4 grayscale"
              />
            )}
            <h1
              className="text-5xl text-white mb-4"
              style={{ fontFamily: "var(--font-timer)" }}
            >
              {player.character.name}
            </h1>
            <p
              className="text-zinc-400 text-lg leading-relaxed"
              style={{ fontFamily: "'Palatino Linotype', Palatino, Georgia, serif" }}
            >
              <span className="text-zinc-500">Team: </span>
              <span className="text-zinc-300 capitalize">{player.alignment}</span>
            </p>
            <p
              className="text-zinc-500 text-base mt-4 leading-relaxed"
              style={{ fontFamily: "'Palatino Linotype', Palatino, Georgia, serif" }}
            >
              {player.character.description}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
