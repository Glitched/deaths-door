import type { PlayerOut } from "@/api/types";

interface PlayerListProps {
  players: PlayerOut[];
}

function PlayerName({ player }: { player: PlayerOut }) {
  const isDead = !player.is_alive;

  return (
    <div className="flex items-center gap-2 py-1">
      <span
        className={`text-[28px] tracking-wide transition-all duration-500 ${
          isDead
            ? "line-through text-red-900 decoration-red-500 decoration-2"
            : "text-red-100/90"
        }`}
      style={{ fontFamily: "'Palatino Linotype', Palatino, 'Book Antiqua', Georgia, serif" }}
      >
        {player.name}
      </span>
    </div>
  );
}

export function PlayerList({ players }: PlayerListProps) {
  if (players.length === 0) return null;

  const midpoint = Math.ceil(players.length / 2);
  const leftColumn = players.slice(0, midpoint);
  const rightColumn = players.slice(midpoint);

  return (
    <div className="grid grid-cols-2 gap-x-16 gap-y-0 px-12 mt-8 max-w-4xl mx-auto">
      <div className="flex flex-col items-end">
        {leftColumn.map((p) => (
          <PlayerName key={p.name} player={p} />
        ))}
      </div>
      <div className="flex flex-col items-start">
        {rightColumn.map((p) => (
          <PlayerName key={p.name} player={p} />
        ))}
      </div>
    </div>
  );
}
