"""Tests for PlayerState and player_state_to_out conversion."""

import pytest
from pydantic import ValidationError

from deaths_door.alignment import Alignment
from deaths_door.game_state import PlayerState, player_state_to_out
from deaths_door.scripts.registry import get_script_by_name


@pytest.mark.anyio
async def test_player_state_defaults():
    """Test PlayerState default values."""
    player = PlayerState(name="Alice", character_name="Imp", alignment="evil")

    assert player.name == "Alice"
    assert player.character_name == "Imp"
    assert player.alignment == "evil"
    assert player.is_alive is True
    assert player.has_used_dead_vote is False
    assert player.status_effects == ()


@pytest.mark.anyio
async def test_player_state_with_all_fields():
    """Test PlayerState with all fields specified."""
    player = PlayerState(
        name="Bob",
        character_name="Chef",
        alignment="good",
        is_alive=False,
        has_used_dead_vote=True,
        status_effects=("Poisoned", "Safe"),
    )

    assert player.is_alive is False
    assert player.has_used_dead_vote is True
    assert "Poisoned" in player.status_effects
    assert "Safe" in player.status_effects


@pytest.mark.anyio
async def test_player_state_is_frozen():
    """Test that PlayerState is immutable."""
    player = PlayerState(name="Alice", character_name="Imp", alignment="evil")

    with pytest.raises(ValidationError):
        player.name = "Bob"  # type: ignore[reportAttributeAccessIssue]


@pytest.mark.anyio
async def test_player_state_model_copy():
    """Test updating PlayerState via model_copy."""
    player = PlayerState(name="Alice", character_name="Imp", alignment="evil")

    dead = player.model_copy(update={"is_alive": False})
    assert dead.is_alive is False
    assert player.is_alive is True  # Original unchanged

    renamed = player.model_copy(update={"name": "Alicia"})
    assert renamed.name == "Alicia"
    assert player.name == "Alice"


@pytest.mark.anyio
async def test_player_state_status_effects_immutable():
    """Test status effect operations via tuple replacement."""
    player = PlayerState(name="Alice", character_name="Chef", alignment="good")

    # Add effect
    with_effect = player.model_copy(update={"status_effects": player.status_effects + ("Poisoned",)})
    assert "Poisoned" in with_effect.status_effects
    assert player.status_effects == ()  # Original unchanged

    # Add duplicate — tuples allow it, apply() prevents it
    with_dup = with_effect.model_copy(update={"status_effects": with_effect.status_effects + ("Poisoned",)})
    assert with_dup.status_effects == ("Poisoned", "Poisoned")

    # Remove effect
    without = with_effect.model_copy(
        update={"status_effects": tuple(e for e in with_effect.status_effects if e != "Poisoned")}
    )
    assert without.status_effects == ()


@pytest.mark.anyio
async def test_player_state_to_out():
    """Test converting PlayerState to API output format."""
    player = PlayerState(
        name="TestPlayer",
        character_name="Chef",
        alignment="good",
        is_alive=False,
        status_effects=("Poisoned",),
    )
    script = get_script_by_name("trouble_brewing")
    assert script is not None

    out = player_state_to_out(player, script)

    assert out.name == "TestPlayer"
    assert out.character.name == "Chef"
    assert out.alignment == Alignment.GOOD
    assert out.is_alive is False
    assert out.has_used_dead_vote is False
    assert "Poisoned" in out.status_effects


@pytest.mark.anyio
async def test_player_state_to_out_traveler():
    """Test converting a traveler PlayerState to API output."""
    player = PlayerState(name="Traveler", character_name="Beggar", alignment="unknown")
    script = get_script_by_name("trouble_brewing")
    assert script is not None

    out = player_state_to_out(player, script)
    assert out.character.name == "Beggar"
    assert out.alignment == Alignment.UNKNOWN


@pytest.mark.anyio
async def test_player_state_to_out_unknown_character():
    """Test that converting with unknown character raises ValueError."""
    player = PlayerState(name="Alice", character_name="FakeCharacter", alignment="good")
    script = get_script_by_name("trouble_brewing")
    assert script is not None

    with pytest.raises(ValueError, match="Character not found: FakeCharacter"):
        player_state_to_out(player, script)


@pytest.mark.anyio
async def test_player_state_game_scenarios():
    """Test realistic game scenarios with PlayerState."""
    # Dead player with used vote
    dead = PlayerState(
        name="DeadPlayer",
        character_name="Chef",
        alignment="good",
        is_alive=False,
        has_used_dead_vote=True,
    )
    assert dead.is_alive is False
    assert dead.has_used_dead_vote is True

    # Evil player with multiple effects
    evil = PlayerState(
        name="EvilPlayer",
        character_name="Imp",
        alignment="evil",
        status_effects=("Poisoned", "Safe"),
    )
    assert len(evil.status_effects) == 2
    assert evil.alignment == "evil"
