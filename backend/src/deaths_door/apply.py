"""Pure functions to apply events to game state."""

from __future__ import annotations

from functools import reduce

from .events import (
    CharactersSwapped,
    DeadVoteUsedSet,
    FirstNightSet,
    GameCreated,
    GameEvent,
    NightStepSet,
    PlayerAdded,
    PlayerAlignmentSet,
    PlayerAliveSet,
    PlayerRemoved,
    PlayerRenamed,
    RoleIncluded,
    RoleRemoved,
    RolesIncluded,
    RoleVisibilitySet,
    StatusEffectAdded,
    StatusEffectRemoved,
    TravelerAdded,
)
from .game_state import GameState, PlayerState


def apply(state: GameState, event: GameEvent) -> GameState:
    """Apply a single event to produce a new game state. Pure function."""
    payload = event.payload
    match payload:
        case GameCreated(script_name=script_name):
            return GameState(
                game_id=event.game_id,
                script_name=script_name,
                version=state.version + 1,
            )

        case NightStepSet(step=step):
            return state.model_copy(
                update={
                    "current_night_step": step,
                    "version": state.version + 1,
                }
            )

        case FirstNightSet(is_first_night=is_first_night):
            return state.model_copy(
                update={
                    "is_first_night": is_first_night,
                    "current_night_step": "Dusk",
                    "version": state.version + 1,
                }
            )

        case RoleVisibilitySet(should_reveal_roles=should_reveal_roles):
            return state.model_copy(
                update={
                    "should_reveal_roles": should_reveal_roles,
                    "version": state.version + 1,
                }
            )

        case RoleIncluded(name=name):
            return state.model_copy(
                update={
                    "included_role_names": state.included_role_names + (name,),
                    "version": state.version + 1,
                }
            )

        case RolesIncluded(names=names):
            return state.model_copy(
                update={
                    "included_role_names": state.included_role_names + tuple(names),
                    "version": state.version + 1,
                }
            )

        case RoleRemoved(name=name):
            roles = list(state.included_role_names)
            normalized = name.lower().strip()
            # Remove first matching role (case-insensitive)
            for i, role in enumerate(roles):
                if role.lower().strip() == normalized:
                    roles.pop(i)
                    break
            return state.model_copy(
                update={
                    "included_role_names": tuple(roles),
                    "version": state.version + 1,
                }
            )

        case PlayerAdded(player_name=player_name, character_name=character_name, alignment=alignment):
            new_player = PlayerState(name=player_name, character_name=character_name, alignment=alignment)
            # Remove the assigned character from the pool
            roles = list(state.included_role_names)
            normalized = character_name.lower().strip()
            for i, role in enumerate(roles):
                if role.lower().strip() == normalized:
                    roles.pop(i)
                    break
            return state.model_copy(
                update={
                    "players": state.players + (new_player,),
                    "included_role_names": tuple(roles),
                    "version": state.version + 1,
                }
            )

        case TravelerAdded(player_name=player_name, traveler_name=traveler_name, alignment=alignment):
            new_player = PlayerState(name=player_name, character_name=traveler_name, alignment=alignment)
            return state.model_copy(
                update={
                    "players": state.players + (new_player,),
                    "version": state.version + 1,
                }
            )

        case PlayerRemoved(player_name=player_name):
            removed = next(p for p in state.players if p.name == player_name)
            remaining = tuple(p for p in state.players if p.name != player_name)
            return state.model_copy(
                update={
                    "players": remaining,
                    "included_role_names": state.included_role_names + (removed.character_name,),
                    "version": state.version + 1,
                }
            )

        case PlayerRenamed(old_name=old_name, new_name=new_name):
            new_state = state.replace_player(old_name, name=new_name)
            return new_state.model_copy(update={"version": state.version + 1})

        case CharactersSwapped(name1=name1, name2=name2):
            p1 = state.get_player(name1)
            p2 = state.get_player(name2)
            if p1 is None or p2 is None:
                raise ValueError(f"Player not found for swap: {name1}, {name2}")
            new_state = state.replace_player(name1, character_name=p2.character_name)
            new_state = new_state.replace_player(name2, character_name=p1.character_name)
            return new_state.model_copy(update={"version": state.version + 1})

        case PlayerAliveSet(player_name=player_name, is_alive=is_alive, cleared_effects=cleared_effects):
            new_state = state.replace_player(player_name, is_alive=is_alive)
            # Apply cascading effect removals
            for target_name, effect in cleared_effects:
                target = new_state.get_player(target_name)
                if target and effect in target.status_effects:
                    new_effects = tuple(e for e in target.status_effects if e != effect)
                    new_state = new_state.replace_player(target_name, status_effects=new_effects)
            return new_state.model_copy(update={"version": state.version + 1})

        case DeadVoteUsedSet(player_name=player_name, has_used_dead_vote=has_used_dead_vote):
            new_state = state.replace_player(player_name, has_used_dead_vote=has_used_dead_vote)
            return new_state.model_copy(update={"version": state.version + 1})

        case PlayerAlignmentSet(player_name=player_name, alignment=alignment):
            new_state = state.replace_player(player_name, alignment=alignment)
            return new_state.model_copy(update={"version": state.version + 1})

        case StatusEffectAdded(player_name=player_name, effect=effect):
            player = state.get_player(player_name)
            if player and effect not in player.status_effects:
                new_effects = player.status_effects + (effect,)
                new_state = state.replace_player(player_name, status_effects=new_effects)
            else:
                new_state = state
            return new_state.model_copy(update={"version": state.version + 1})

        case StatusEffectRemoved(player_name=player_name, effect=effect):
            player = state.get_player(player_name)
            if player and effect in player.status_effects:
                new_effects = tuple(e for e in player.status_effects if e != effect)
                new_state = state.replace_player(player_name, status_effects=new_effects)
            else:
                new_state = state
            return new_state.model_copy(update={"version": state.version + 1})

        case _:
            raise ValueError(f"Unknown event type: {type(payload)}")


def replay(events: list[GameEvent]) -> GameState:
    """Rebuild game state by replaying a sequence of events."""
    if not events:
        raise ValueError("Cannot replay empty event list")

    initial = GameState(game_id=events[0].game_id, script_name="")
    return reduce(apply, events, initial)
