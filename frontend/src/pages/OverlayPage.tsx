import { TimerDisplay } from "@/components/timer/TimerDisplay";
import { PlayerList } from "@/components/players/PlayerList";
import { useGameState } from "@/hooks/useGameState";
import { isNightPhase } from "@/api/types";

export function OverlayPage() {
  const gameState = useGameState();
  const night = gameState ? isNightPhase(gameState) : false;

  return (
    <div className="min-h-screen bg-black">
      <TimerDisplay />
      {gameState && !night && (
        <>
          <div
            className="text-center text-red-100/50 text-[22px] tracking-widest uppercase mt-4"
            style={{ fontFamily: "'Palatino Linotype', Palatino, 'Book Antiqua', Georgia, serif" }}
          >
            {gameState.execution_threshold} to convict
          </div>
          <PlayerList players={gameState.players} />
        </>
      )}
    </div>
  );
}
