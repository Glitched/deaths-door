import pytest

from deaths_door.alignment import Alignment
from deaths_door.game import Game
from deaths_door.script import ScriptName

from .helpers import GameTestCase


@pytest.mark.anyio
async def test_add_player_with_specific_role():
    """Test adding a player with a specific role."""
    test_case = GameTestCase()

    player = test_case.game.add_player_with_role("Ryan", "Imp")

    assert player.name == "Ryan"
    assert player.character.name == "Imp"
    assert player.alignment == Alignment.EVIL
    assert player.is_alive is True
    assert len(test_case.game.players) == 1
    assert len(test_case.game.included_roles) < len(
        test_case.game.script.characters
    )  # Role was removed


@pytest.mark.anyio
async def test_add_player_with_nonexistent_role():
    """Test that adding a player with non-existent role raises error."""
    test_case = GameTestCase()

    with pytest.raises(
        ValueError, match="Role 'NonExistentRole' not found in included roles"
    ):
        test_case.game.add_player_with_role("Alice", "NonExistentRole")


@pytest.mark.anyio
async def test_add_player_with_random_role():
    """Test adding a player with a random role assignment."""
    test_case = GameTestCase()
    original_role_count = len(test_case.game.included_roles)

    player = test_case.game.add_player_with_random_role("Bob")

    assert player.name == "Bob"
    assert player.character.name is not None
    assert player.is_alive is True
    assert len(test_case.game.players) == 1
    assert len(test_case.game.included_roles) == original_role_count - 1


@pytest.mark.anyio
async def test_add_traveler():
    """Test adding a traveler character."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Add a traveler (they don't need to be in included_roles)
    player = game.add_player_as_traveler("Traveler_Alice", "Beggar")

    assert player.name == "Traveler_Alice"
    assert player.character.name == "Beggar"
    assert player.alignment == Alignment.UNKNOWN  # Travelers have unknown alignment
    assert len(game.players) == 1


@pytest.mark.anyio
async def test_add_nonexistent_traveler():
    """Test that adding non-existent traveler raises error."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    with pytest.raises(ValueError, match="Traveler not found or in game: FakeTraveler"):
        game.add_player_as_traveler("Alice", "FakeTraveler")


@pytest.mark.anyio
async def test_add_duplicate_traveler():
    """Test that adding the same traveler twice raises error."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # First traveler succeeds
    game.add_player_as_traveler("Alice", "Beggar")

    # Second player with same traveler should fail
    with pytest.raises(ValueError, match="Traveler not found or in game: Beggar"):
        game.add_player_as_traveler("Bob", "Beggar")


@pytest.mark.anyio
async def test_has_living_character_named():
    """Test checking for living characters by name."""
    test_case = GameTestCase()

    # Add an Imp player
    imp_player = test_case.game.add_player_with_role("Evil_Alice", "Imp")

    # Should find living Imp
    assert test_case.game.has_living_character_named("Imp") is True
    assert test_case.game.has_living_character_named("Chef") is False

    # Kill the Imp
    imp_player.is_alive = False

    # Should no longer find living Imp
    assert test_case.game.has_living_character_named("Imp") is False


@pytest.mark.anyio
async def test_get_unclaimed_travelers():
    """Test getting travelers that haven't been claimed yet."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Initially all travelers are unclaimed
    all_travelers = game.script.travelers
    unclaimed = game.get_unclaimed_travelers()
    assert len(unclaimed) == len(all_travelers)

    # Claim one traveler
    game.add_player_as_traveler("Alice", "Beggar")

    # Should have one less unclaimed traveler
    unclaimed_after = game.get_unclaimed_travelers()
    assert len(unclaimed_after) == len(all_travelers) - 1
    assert not any(t.name == "Beggar" for t in unclaimed_after)


@pytest.mark.anyio
async def test_night_steps_filtering():
    """Test that night steps are filtered based on living characters."""
    test_case = GameTestCase()

    # Add some specific characters
    test_case.game.add_player_with_role("Chef_Player", "Chef")
    test_case.game.add_player_with_role("Imp_Player", "Imp")

    first_night_steps = list(test_case.game.get_first_night_steps())
    other_night_steps = list(test_case.game.get_other_night_steps())

    # Should include steps for living characters
    chef_steps = [step for step in first_night_steps if step.name == "Chef"]
    imp_steps = [step for step in other_night_steps if step.name == "Imp"]

    assert len(chef_steps) > 0  # Chef has first night ability
    assert len(imp_steps) > 0  # Imp has night ability

    # Kill the Chef
    chef_player = test_case.game.get_player_by_name("Chef_Player")
    assert chef_player is not None
    chef_player.is_alive = False

    # Chef steps should no longer appear in subsequent nights
    # (but may still appear in first night since it's a different method)
    updated_other_nights = list(test_case.game.get_other_night_steps())
    chef_other_steps = [step for step in updated_other_nights if step.name == "Chef"]
    assert len(chef_other_steps) == 0  # Dead Chef shouldn't have night actions


@pytest.mark.anyio
async def test_role_reveal_visibility():
    """Test role reveal visibility toggle."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Initially roles should not be revealed
    assert game.should_reveal_roles is False

    # Toggle role reveal
    game.should_reveal_roles = True
    assert game.should_reveal_roles is True


@pytest.mark.anyio
async def test_player_removal():
    """Test removing players from the game."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.included_roles = game.script.characters.copy()

    # Add two players
    player1 = game.add_player_with_role("Alice", "Imp")
    game.add_player_with_role("Bob", "Chef")

    assert len(game.players) == 2
    original_roles_count = len(game.included_roles)

    # Remove one player
    game.remove_player_by_name(player1.name)

    assert len(game.players) == 1
    assert game.get_player_by_name("Alice") is None
    assert game.get_player_by_name("Bob") is not None
    # Role should be returned to available pool
    assert len(game.included_roles) == original_roles_count + 1


@pytest.mark.anyio
async def test_player_removal_by_name():
    """Test removing players by name."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.included_roles = game.script.characters.copy()

    game.add_player_with_role("Alice", "Imp")
    game.add_player_with_role("Bob", "Chef")

    # Remove by name
    game.remove_player_by_name("Alice")
    assert len(game.players) == 1
    assert game.get_player_by_name("Alice") is None


@pytest.mark.anyio
async def test_remove_nonexistent_player():
    """Test that removing non-existent player raises appropriate error."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    with pytest.raises(ValueError, match="Player not found: NonExistentPlayer"):
        game.remove_player_by_name("NonExistentPlayer")


@pytest.mark.anyio
async def test_night_phase_defaults():
    """Test that night phase fields have correct default values."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Check defaults
    assert game.current_night_step == "Dusk"
    assert game.is_first_night is True


@pytest.mark.anyio
async def test_night_phase_modification():
    """Test modifying night phase fields."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Modify current_night_step
    game.current_night_step = "Poisoner"
    assert game.current_night_step == "Poisoner"

    # Modify is_first_night
    game.is_first_night = False
    assert game.is_first_night is False

    # Change back
    game.current_night_step = "Dusk"
    game.is_first_night = True
    assert game.current_night_step == "Dusk"
    assert game.is_first_night is True
