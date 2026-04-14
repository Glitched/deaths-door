import { TimerDisplay } from "@/components/timer/TimerDisplay";
import { PlayerList } from "@/components/players/PlayerList";
import { useGameState } from "@/hooks/useGameState";
import { isNightPhase } from "@/api/types";

export function OverlayPage() {
  const { state: gameState, status } = useGameState();
  const night = gameState ? isNightPhase(gameState) : false;

  return (
    <div className="min-h-screen bg-black relative">
      {status === "disconnected" && (
        <div className="absolute top-4 right-4 w-6 h-6 rounded-full bg-red-500 animate-pulse blur-md shadow-[0_0_20px_8px_rgba(239,68,68,0.5)]" />
      )}
      <TimerDisplay />
      {gameState && !night && (
        <>
          <div
            className="text-center text-red-100/50 text-[22px] tracking-widest uppercase mt-4"
            style={{
              fontFamily:
                "'Palatino Linotype', Palatino, 'Book Antiqua', Georgia, serif",
            }}
          >
            {gameState.execution_threshold} to convict
          </div>
          <PlayerList players={gameState.players} />
        </>
      )}
    </div>
  );
}
