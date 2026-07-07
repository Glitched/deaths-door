export interface TimerState {
  is_running: boolean;
  seconds: number;
}

export interface CharacterOut {
  name: string;
  description: string;
  icon_path: string;
  alignment: string;
}

export interface PlayerOut {
  name: string;
  character: CharacterOut;
  alignment: string;
  is_alive: boolean;
  has_used_dead_vote: boolean;
  status_effects: string[];
}

export interface ChoppingBlock {
  player_name: string;
  votes: number | null;
}

/** A scene effect (lights + sound) currently playing on the physical rig. */
export interface ActiveEffect {
  /** Increments on every trigger; replay the visual when this changes. */
  id: number;
  scene: string;
  duration_ms: number;
}

/** A nomination vote being tallied live (hands going around the circle). */
export interface VoteInProgress {
  player_name: string;
  voters: string[];
}

export interface GameState {
  script_name: string;
  players: PlayerOut[];
  living_player_count: number;
  execution_threshold: number;
  dead_players_with_vote: string[];
  current_night_step: string;
  is_first_night: boolean;
  phase?: "day" | "night";
  chopping_block: ChoppingBlock | null;
  vote_in_progress?: VoteInProgress | null;
  winner?: string | null;
  game_over_hint?: string | null;
  active_effect: ActiveEffect | null;
  timer: TimerState;
}

export function isNightPhase(state: GameState): boolean {
  // Prefer the first-class phase; fall back to the bookmark for old backends.
  if (state.phase) return state.phase === "night";
  const step = state.current_night_step;
  return step !== "Dawn" && step !== "";
}
