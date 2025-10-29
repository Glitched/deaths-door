import pytest

from deaths_door.alignment import Alignment
from deaths_door.characters.trouble_brewing.chef import Chef
from deaths_door.characters.trouble_brewing.imp import Imp
from deaths_door.player import Player


@pytest.mark.anyio
async def test_player_creation():
    """Test basic player creation with character assignment."""
    imp_character = Imp()
    player = Player("Alice", imp_character)

    assert player.name == "Alice"
    assert player.character.name == "Imp"
    assert player.alignment == Alignment.EVIL  # Inherited from Imp
    assert player.is_alive is True
    assert player.has_used_dead_vote is False
    assert player.status_effects == []


@pytest.mark.anyio
async def test_player_alignment_inheritance():
    """Test that players inherit alignment from their characters."""
    # Evil character
    imp = Imp()
    evil_player = Player("EvilPlayer", imp)
    assert evil_player.alignment == Alignment.EVIL

    # Good character
    chef = Chef()
    good_player = Player("GoodPlayer", chef)
    assert good_player.alignment == Alignment.GOOD


@pytest.mark.anyio
async def test_player_death_and_resurrection():
    """Test player death/resurrection mechanics."""
    player = Player("Bob", Imp())

    # Player starts alive
    assert player.is_alive is True

    # Kill player
    player.is_alive = False
    assert player.is_alive is False

    # Resurrect player
    player.is_alive = True
    assert player.is_alive is True


@pytest.mark.anyio
async def test_dead_vote_usage():
    """Test dead vote tracking."""
    player = Player("Charlie", Chef())

    # Initially hasn't used dead vote
    assert player.has_used_dead_vote is False

    # Use dead vote
    player.has_used_dead_vote = True
    assert player.has_used_dead_vote is True


@pytest.mark.anyio
async def test_player_name_update():
    """Test updating player name."""
    player = Player("OldName", Chef())
    original_character = player.character

    player.set_name("NewName")

    assert player.name == "NewName"
    assert player.character is original_character  # Character unchanged


@pytest.mark.anyio
async def test_player_status_effects():
    """Test player status effect management."""
    player = Player("Alice", Chef())

    # Initially no status effects
    assert len(player.status_effects) == 0

    # Add status effect
    player.add_status_effect("Poisoned")
    assert "Poisoned" in player.status_effects
    assert len(player.status_effects) == 1

    # Add duplicate - should still only have one
    player.add_status_effect("Poisoned")
    assert len(player.status_effects) == 1

    # Add different status effect
    player.add_status_effect("Safe")
    assert "Safe" in player.status_effects
    assert len(player.status_effects) == 2

    # Remove status effect
    player.remove_status_effect("Poisoned")
    assert "Poisoned" not in player.status_effects
    assert "Safe" in player.status_effects
    assert len(player.status_effects) == 1


@pytest.mark.anyio
async def test_remove_nonexistent_status_effect():
    """Test that removing non-existent status effect raises appropriate error."""
    player = Player("Alice", Chef())

    # Try to remove non-existent status effect - should not raise error (safe removal)
    player.remove_status_effect("NonExistentEffect")  # Should not raise


@pytest.mark.anyio
async def test_player_character_swap():
    """Test swapping a player's character."""
    player = Player("Alice", Chef())
    original_alignment = player.alignment

    # Swap to different character
    new_character = Imp()
    player.set_character(new_character)
    player.set_alignment(new_character.alignment)  # Manually update alignment

    assert player.character.name == "Imp"
    assert player.alignment != original_alignment  # Should change with character
    assert player.alignment == Alignment.EVIL
    assert player.name == "Alice"  # Name stays the same


@pytest.mark.anyio
async def test_player_serialization():
    """Test converting player to output format."""
    player = Player("TestPlayer", Chef())
    player.add_status_effect("Poisoned")
    player.is_alive = False

    player_out = player.to_out()

    assert player_out.name == "TestPlayer"
    assert player_out.character.name == "Chef"
    assert player_out.alignment == Alignment.GOOD
    assert player_out.is_alive is False
    assert player_out.has_used_dead_vote is False
    assert "Poisoned" in player_out.status_effects


@pytest.mark.anyio
async def test_player_string_representation():
    """Test player string representations."""
    alive_player = Player("Alice", Chef())
    dead_player = Player("Bob", Imp())
    dead_player.is_alive = False

    # Test __str__ method
    assert "Alice as Chef (alive)" in str(alive_player)
    assert "Bob as Imp (dead)" in str(dead_player)

    # Test __repr__ method
    alive_repr = repr(alive_player)
    assert "Player(name='Alice'" in alive_repr
    assert "character='Chef'" in alive_repr
    assert "is_alive=True" in alive_repr

    dead_repr = repr(dead_player)
    assert "is_alive=False" in dead_repr


@pytest.mark.anyio
async def test_player_game_integration_scenarios():
    """Test realistic game scenarios with players."""
    # Scenario: Drunk thinks they're the Chef but is actually the Drunk
    drunk_player = Player("DrunkPlayer", Chef())  # Thinks they're Chef
    drunk_player.alignment = Alignment.GOOD  # But actually good (Drunk is Outsider)

    # The Drunk's apparent character doesn't match their true alignment in this case
    # This is a legitimate game mechanic
    assert drunk_player.character.name == "Chef"
    assert drunk_player.alignment == Alignment.GOOD

    # Scenario: Player dies and uses their dead vote
    dead_player = Player("DeadPlayer", Chef())
    dead_player.is_alive = False
    dead_player.has_used_dead_vote = True

    assert dead_player.is_alive is False
    assert dead_player.has_used_dead_vote is True

    # Scenario: Evil player with multiple status effects
    evil_player = Player("EvilPlayer", Imp())
    evil_player.add_status_effect("Poisoned")
    evil_player.add_status_effect("Safe")  # Contradictory but possible

    assert len(evil_player.status_effects) == 2
    assert evil_player.alignment == Alignment.EVIL
