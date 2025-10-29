"""Tests for status effect removal when characters die."""

import pytest

from deaths_door.game import Game
from deaths_door.script import ScriptName


@pytest.mark.anyio
async def test_poisoner_death_removes_poisoned_status():
    """Test that when Poisoner dies, Poisoned status is removed from all players."""
    # Create game with Poisoner and other roles
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Poisoner")
    game.include_role("Chef")
    game.include_role("Empath")

    # Add players
    _poisoner_player = game.add_player_with_role("Alice", "Poisoner")
    chef_player = game.add_player_with_role("Bob", "Chef")
    empath_player = game.add_player_with_role("Charlie", "Empath")

    # Poisoner poisons both other players
    chef_player.add_status_effect("Poisoned")
    empath_player.add_status_effect("Poisoned")

    # Verify they're poisoned
    assert "Poisoned" in chef_player.status_effects
    assert "Poisoned" in empath_player.status_effects

    # Poisoner dies
    game.set_player_alive_status("Alice", False)

    # Verify Poisoned status was removed from all players
    assert "Poisoned" not in chef_player.status_effects
    assert "Poisoned" not in empath_player.status_effects


@pytest.mark.anyio
async def test_monk_death_removes_safe_status():
    """Test that when Monk dies, Safe status is removed from all players."""
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Monk")
    game.include_role("Chef")
    game.include_role("Empath")

    _monk_player = game.add_player_with_role("Alice", "Monk")
    chef_player = game.add_player_with_role("Bob", "Chef")
    empath_player = game.add_player_with_role("Charlie", "Empath")

    # Monk protects both other players
    chef_player.add_status_effect("Safe")
    empath_player.add_status_effect("Safe")

    # Verify they're safe
    assert "Safe" in chef_player.status_effects
    assert "Safe" in empath_player.status_effects

    # Monk dies
    game.set_player_alive_status("Alice", False)

    # Verify Safe status was removed
    assert "Safe" not in chef_player.status_effects
    assert "Safe" not in empath_player.status_effects


@pytest.mark.anyio
async def test_butler_death_removes_master_status():
    """Test that when Butler dies, Butler's Master status is removed from all players."""
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Butler")
    game.include_role("Chef")
    game.include_role("Empath")

    _butler_player = game.add_player_with_role("Alice", "Butler")
    chef_player = game.add_player_with_role("Bob", "Chef")
    _empath_player = game.add_player_with_role("Charlie", "Empath")

    # Mark Chef as Butler's Master
    chef_player.add_status_effect("Butler's Master")

    # Verify status exists
    assert "Butler's Master" in chef_player.status_effects

    # Butler dies
    game.set_player_alive_status("Alice", False)

    # Verify Butler's Master status was removed
    assert "Butler's Master" not in chef_player.status_effects


@pytest.mark.anyio
async def test_non_persistent_character_death_leaves_other_effects():
    """Test that when a non-persistent character dies, other status effects remain."""
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Imp")
    game.include_role("Chef")
    game.include_role("Empath")

    _imp_player = game.add_player_with_role("Alice", "Imp")
    chef_player = game.add_player_with_role("Bob", "Chef")
    empath_player = game.add_player_with_role("Charlie", "Empath")

    # Add some status effects that shouldn't be removed
    chef_player.add_status_effect("Poisoned")
    empath_player.add_status_effect("Safe")

    # Imp dies (Imp doesn't have persistent status effects on others)
    game.set_player_alive_status("Alice", False)

    # Verify status effects remain
    assert "Poisoned" in chef_player.status_effects
    assert "Safe" in empath_player.status_effects


@pytest.mark.anyio
async def test_resurrection_does_not_trigger_cleanup():
    """Test that resurrecting a player doesn't trigger status effect cleanup."""
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Poisoner")
    game.include_role("Chef")

    _poisoner_player = game.add_player_with_role("Alice", "Poisoner")
    chef_player = game.add_player_with_role("Bob", "Chef")

    # Add poisoned status
    chef_player.add_status_effect("Poisoned")

    # Kill and resurrect Poisoner
    game.set_player_alive_status("Alice", False)
    assert "Poisoned" not in chef_player.status_effects  # Removed on death

    # Resurrect Poisoner
    chef_player.add_status_effect("Poisoned")  # Re-poison after resurrection
    game.set_player_alive_status("Alice", True)

    # Status should still be there (resurrection doesn't trigger cleanup)
    assert "Poisoned" in chef_player.status_effects


@pytest.mark.anyio
async def test_multiple_status_effects_partial_removal():
    """Test that death only removes relevant status effects, keeping others."""
    game = Game(ScriptName.TROUBLE_BREWING)
    game.include_role("Poisoner")
    game.include_role("Chef")

    _poisoner_player = game.add_player_with_role("Alice", "Poisoner")
    chef_player = game.add_player_with_role("Bob", "Chef")

    # Add multiple status effects
    chef_player.add_status_effect("Poisoned")
    chef_player.add_status_effect("Safe")
    chef_player.add_status_effect("Dead")

    # Poisoner dies
    game.set_player_alive_status("Alice", False)

    # Only Poisoned should be removed
    assert "Poisoned" not in chef_player.status_effects
    assert "Safe" in chef_player.status_effects
    assert "Dead" in chef_player.status_effects
