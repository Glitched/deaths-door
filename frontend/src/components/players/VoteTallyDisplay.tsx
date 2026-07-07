import type { ChoppingBlock, VoteInProgress } from "@/api/types";

interface VoteTallyDisplayProps {
  vote: VoteInProgress;
  executionThreshold: number;
  block: ChoppingBlock | null;
}

// Shown while the storyteller tallies a vote: the nominee, the count climbing
// as hands go up, and the number to beat. Takes over from the player list /
// chopping block until the vote is confirmed or cancelled.
export function VoteTallyDisplay({
  vote,
  executionThreshold,
  block,
}: VoteTallyDisplayProps) {
  const count = vote.voters.length;
  // The number that matters: the current block's votes (tie clears, one more
  // steals) when someone is already on the block, else the base threshold.
  const target =
    block && block.votes != null
      ? Math.max(executionThreshold, block.votes + 1)
      : executionThreshold;
  const reached = count >= target;
  const subtitle =
    block && block.votes != null
      ? `${block.votes} ties · ${Math.max(executionThreshold, block.votes + 1)} takes the block`
      : `${executionThreshold} to convict`;

  return (
    <div
      className="flex flex-col items-center text-center mt-4 animate-in fade-in duration-500"
      style={{
        fontFamily:
          "'Palatino Linotype', Palatino, 'Book Antiqua', Georgia, serif",
      }}
    >
      <div className="text-red-100/50 text-[22px] tracking-widest uppercase">
        on trial:
      </div>
      <div className="text-[100px] leading-tight tracking-wide bg-gradient-to-b from-[#FF0000] to-[#690000] bg-clip-text text-transparent">
        {vote.player_name}
      </div>
      <div
        // Remount per count so the number pops on every raised hand.
        key={count}
        className={`text-[160px] leading-none tracking-wide animate-in zoom-in-75 duration-300 ${
          reached ? "text-red-500" : "text-red-100/80"
        }`}
      >
        {count}
      </div>
      <div className="text-red-100/50 text-[22px] tracking-widest uppercase mt-2">
        {subtitle}
      </div>
    </div>
  );
}
