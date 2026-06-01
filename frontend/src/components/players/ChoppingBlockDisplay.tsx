import type { ChoppingBlock } from "@/api/types";

interface ChoppingBlockDisplayProps {
  block: ChoppingBlock;
  executionThreshold: number;
}

// Shown in place of the player list while someone is up for execution.
export function ChoppingBlockDisplay({
  block,
  executionThreshold,
}: ChoppingBlockDisplayProps) {
  // If the storyteller recorded the vote tally, show the tie/beat numbers: a
  // later nomination that ties the count clears the block (nobody dies), and
  // one more vote steals it. Otherwise fall back to the base threshold.
  const subtitle =
    block.votes != null
      ? `${block.votes} to tie · ${block.votes + 1} to beat`
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
        chopping block:
      </div>
      <div className="text-[120px] leading-tight tracking-wide bg-gradient-to-b from-[#FF0000] to-[#690000] bg-clip-text text-transparent">
        {block.player_name}
      </div>
      <div className="text-red-100/50 text-[22px] tracking-widest uppercase">
        {subtitle}
      </div>
    </div>
  );
}
