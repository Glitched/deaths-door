"""Event types and payload models for event sourcing."""

from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Annotated, Literal, Union
from uuid import UUID, uuid4

from pydantic import BaseModel, Field


class EventType(str, Enum):
    """All possible game event types."""

    GAME_CREATED = "game_created"
    NIGHT_STEP_SET = "night_step_set"
    FIRST_NIGHT_SET = "first_night_set"
    ROLE_VISIBILITY_SET = "role_visibility_set"
    ROLE_INCLUDED = "role_included"
    ROLES_INCLUDED = "roles_included"
    ROLE_REMOVED = "role_removed"
    PLAYER_ADDED = "player_added"
    TRAVELER_ADDED = "traveler_added"
    PLAYER_REMOVED = "player_removed"
    PLAYER_RENAMED = "player_renamed"
    CHARACTERS_SWAPPED = "characters_swapped"
    PLAYER_ALIVE_SET = "player_alive_set"
    DEAD_VOTE_USED_SET = "dead_vote_used_set"
    PLAYER_ALIGNMENT_SET = "player_alignment_set"
    STATUS_EFFECT_ADDED = "status_effect_added"
    STATUS_EFFECT_REMOVED = "status_effect_removed"


# --- Event payload models (one per event type) ---


class GameCreated(BaseModel, frozen=True):
    """A new game was created with a script."""

    type: Literal[EventType.GAME_CREATED] = EventType.GAME_CREATED
    script_name: str


class NightStepSet(BaseModel, frozen=True):
    """The current night step bookmark was changed."""

    type: Literal[EventType.NIGHT_STEP_SET] = EventType.NIGHT_STEP_SET
    step: str


class FirstNightSet(BaseModel, frozen=True):
    """The first night flag was toggled (resets step to Dusk)."""

    type: Literal[EventType.FIRST_NIGHT_SET] = EventType.FIRST_NIGHT_SET
    is_first_night: bool


class RoleVisibilitySet(BaseModel, frozen=True):
    """Role visibility was toggled."""

    type: Literal[EventType.ROLE_VISIBILITY_SET] = EventType.ROLE_VISIBILITY_SET
    should_reveal_roles: bool


class RoleIncluded(BaseModel, frozen=True):
    """A single role was added to the game's available pool."""

    type: Literal[EventType.ROLE_INCLUDED] = EventType.ROLE_INCLUDED
    name: str


class RolesIncluded(BaseModel, frozen=True):
    """Multiple roles were added to the game's available pool."""

    type: Literal[EventType.ROLES_INCLUDED] = EventType.ROLES_INCLUDED
    names: tuple[str, ...]


class RoleRemoved(BaseModel, frozen=True):
    """A role was removed from the game's available pool."""

    type: Literal[EventType.ROLE_REMOVED] = EventType.ROLE_REMOVED
    name: str


class PlayerAdded(BaseModel, frozen=True):
    """A player was added with an assigned character (randomness already resolved)."""

    type: Literal[EventType.PLAYER_ADDED] = EventType.PLAYER_ADDED
    player_name: str
    character_name: str
    alignment: str


class TravelerAdded(BaseModel, frozen=True):
    """A player was added as a specific traveler."""

    type: Literal[EventType.TRAVELER_ADDED] = EventType.TRAVELER_ADDED
    player_name: str
    traveler_name: str
    alignment: str


class PlayerRemoved(BaseModel, frozen=True):
    """A player was removed and their character returned to the pool."""

    type: Literal[EventType.PLAYER_REMOVED] = EventType.PLAYER_REMOVED
    player_name: str


class PlayerRenamed(BaseModel, frozen=True):
    """A player's name was changed."""

    type: Literal[EventType.PLAYER_RENAMED] = EventType.PLAYER_RENAMED
    old_name: str
    new_name: str


class CharactersSwapped(BaseModel, frozen=True):
    """Two players' characters were swapped."""

    type: Literal[EventType.CHARACTERS_SWAPPED] = EventType.CHARACTERS_SWAPPED
    name1: str
    name2: str


class PlayerAliveSet(BaseModel, frozen=True):
    """A player's alive status was changed, with any cascading effect removals."""

    type: Literal[EventType.PLAYER_ALIVE_SET] = EventType.PLAYER_ALIVE_SET
    player_name: str
    is_alive: bool
    cleared_effects: tuple[tuple[str, str], ...] = ()  # (player_name, effect_name) pairs


class DeadVoteUsedSet(BaseModel, frozen=True):
    """A dead player's vote usage was recorded."""

    type: Literal[EventType.DEAD_VOTE_USED_SET] = EventType.DEAD_VOTE_USED_SET
    player_name: str
    has_used_dead_vote: bool


class PlayerAlignmentSet(BaseModel, frozen=True):
    """A player's alignment was changed."""

    type: Literal[EventType.PLAYER_ALIGNMENT_SET] = EventType.PLAYER_ALIGNMENT_SET
    player_name: str
    alignment: str


class StatusEffectAdded(BaseModel, frozen=True):
    """A status effect was added to a player."""

    type: Literal[EventType.STATUS_EFFECT_ADDED] = EventType.STATUS_EFFECT_ADDED
    player_name: str
    effect: str


class StatusEffectRemoved(BaseModel, frozen=True):
    """A status effect was removed from a player."""

    type: Literal[EventType.STATUS_EFFECT_REMOVED] = EventType.STATUS_EFFECT_REMOVED
    player_name: str
    effect: str


EventPayload = Annotated[
    Union[
        GameCreated,
        NightStepSet,
        FirstNightSet,
        RoleVisibilitySet,
        RoleIncluded,
        RolesIncluded,
        RoleRemoved,
        PlayerAdded,
        TravelerAdded,
        PlayerRemoved,
        PlayerRenamed,
        CharactersSwapped,
        PlayerAliveSet,
        DeadVoteUsedSet,
        PlayerAlignmentSet,
        StatusEffectAdded,
        StatusEffectRemoved,
    ],
    Field(discriminator="type"),
]


class GameEvent(BaseModel, frozen=True):
    """A persisted event envelope wrapping a typed payload."""

    id: UUID = Field(default_factory=uuid4)
    game_id: UUID
    sequence: int
    timestamp: datetime
    payload: EventPayload
