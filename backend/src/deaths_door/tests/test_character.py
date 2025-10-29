import pytest

from deaths_door.alignment import Alignment
from deaths_door.character_type import CharacterType
from deaths_door.characters.trouble_brewing.chef import Chef
from deaths_door.characters.trouble_brewing.fortune_teller import FortuneTeller
from deaths_door.characters.trouble_brewing.imp import Imp
from deaths_door.travelers.trouble_brewing.beggar import Beggar


@pytest.mark.anyio
async def test_character_basic_properties():
    """Test basic character properties are set correctly."""
    imp = Imp()

    assert imp.name == "Imp"
    assert imp.category == CharacterType.DEMON
    assert imp.alignment == Alignment.EVIL
    assert imp.description is not None
    assert len(imp.description) > 0


@pytest.mark.anyio
async def test_character_name_normalization():
    """Test character name normalization for comparison."""
    chef = Chef()

    # Test exact match
    assert chef.is_named("Chef") is True

    # Test case insensitive
    assert chef.is_named("chef") is True
    assert chef.is_named("CHEF") is True

    # Test with whitespace
    assert chef.is_named(" Chef ") is True
    assert chef.is_named("  chef  ") is True

    # Test non-match
    assert chef.is_named("Imp") is False
    assert chef.is_named("NotACharacter") is False


@pytest.mark.anyio
async def test_character_icon_path():
    """Test character icon path generation."""
    chef = Chef()
    fortune_teller = FortuneTeller()

    # Simple name
    assert chef.get_icon_path() == "chef.png"

    # Name with space should be converted
    assert fortune_teller.get_icon_path() == "fortuneteller.png"


@pytest.mark.anyio
async def test_character_status_effects():
    """Test character status effects functionality."""
    imp = Imp()
    fortune_teller = FortuneTeller()

    # Imp should have status effects (Dead and IsTheDrunk)
    assert len(imp.status_effects) > 0

    # Fortune Teller should have Red Herring status effect
    assert len(fortune_teller.status_effects) > 0

    # Test status effects output conversion
    imp_effects = imp.get_status_effects_out()
    assert len(imp_effects) > 0

    for effect in imp_effects:
        assert effect.character_name == "Imp"
        assert effect.name is not None


@pytest.mark.anyio
async def test_character_serialization():
    """Test character serialization to output format."""
    chef = Chef()
    chef_out = chef.to_out()

    assert chef_out.name == "Chef"
    assert chef_out.description == chef.description
    assert chef_out.icon_path == "chef.png"
    assert chef_out.alignment == Alignment.GOOD
    assert chef_out.category == CharacterType.TOWNSFOLK


@pytest.mark.anyio
async def test_character_string_representations():
    """Test character string representations."""
    imp = Imp()

    # Test __str__ method
    str_repr = str(imp)
    assert "Imp" in str_repr
    assert "demon" in str_repr.lower()

    # Test __repr__ method
    repr_str = repr(imp)
    assert "Character(name='Imp'" in repr_str
    assert "category=" in repr_str
    assert "alignment=" in repr_str


@pytest.mark.anyio
async def test_different_character_types():
    """Test characters of different types have correct properties."""
    # Townsfolk
    chef = Chef()
    assert chef.category == CharacterType.TOWNSFOLK
    assert chef.alignment == Alignment.GOOD

    # Demon
    imp = Imp()
    assert imp.category == CharacterType.DEMON
    assert imp.alignment == Alignment.EVIL

    # Traveler
    beggar = Beggar()
    assert beggar.category == CharacterType.TRAVELER
    assert beggar.alignment == Alignment.UNKNOWN


@pytest.mark.anyio
async def test_character_equality():
    """Test character equality comparison."""
    imp1 = Imp()
    imp2 = Imp()
    chef = Chef()

    # Same character type should be equal based on their properties
    # (though they're different instances, they represent the same character)
    assert imp1.name == imp2.name
    assert imp1.category == imp2.category
    assert imp1.alignment == imp2.alignment

    # Different characters should have different properties
    assert imp1.name != chef.name
    assert imp1.category != chef.category
    assert imp1.alignment != chef.alignment


@pytest.mark.anyio
async def test_character_descriptions_are_meaningful():
    """Test that character descriptions contain game-relevant information."""
    imp = Imp()
    chef = Chef()
    fortune_teller = FortuneTeller()

    # Imp description should mention night killing
    assert "night" in imp.description.lower()
    assert "die" in imp.description.lower()

    # Chef description should mention learning information
    assert "learn" in chef.description.lower() or "know" in chef.description.lower()

    # Fortune Teller should mention demons and false positives
    assert "demon" in fortune_teller.description.lower()
    assert (
        "false" in fortune_teller.description.lower()
        or "register" in fortune_teller.description.lower()
    )


@pytest.mark.anyio
async def test_character_creation_consistency():
    """Test that creating multiple instances of same character is consistent."""
    # Create multiple instances of same character
    chefs = [Chef() for _ in range(3)]

    # All should have identical properties
    for chef in chefs:
        assert chef.name == "Chef"
        assert chef.category == CharacterType.TOWNSFOLK
        assert chef.alignment == Alignment.GOOD
        assert chef.description == chefs[0].description


@pytest.mark.anyio
async def test_special_character_properties():
    """Test special properties of specific characters."""
    # Fortune Teller has Red Herring
    fortune_teller = FortuneTeller()
    status_effects = fortune_teller.get_status_effects_out()
    effect_names = [effect.name for effect in status_effects]
    assert any("Red Herring" in name for name in effect_names)

    # Imp has multiple status effects including Dead and IsTheDrunk
    imp = Imp()
    imp_effects = imp.get_status_effects_out()
    imp_effect_names = [effect.name for effect in imp_effects]
    assert "Dead" in imp_effect_names
    assert "Is the Drunk" in imp_effect_names


@pytest.mark.anyio
async def test_character_game_integration():
    """Test characters work correctly in game scenarios."""
    # Create characters that would appear in a typical game
    characters = [Chef(), Imp(), FortuneTeller(), Beggar()]

    # Each should have unique name
    names = [char.name for char in characters]
    assert len(set(names)) == len(names)  # All unique

    # Should have mix of alignments
    alignments = [char.alignment for char in characters]
    assert Alignment.GOOD in alignments
    assert Alignment.EVIL in alignments
    assert Alignment.UNKNOWN in alignments

    # Should have mix of categories
    categories = [char.category for char in characters]
    assert CharacterType.TOWNSFOLK in categories
    assert CharacterType.DEMON in categories
    assert CharacterType.TRAVELER in categories


@pytest.mark.anyio
async def test_character_name_edge_cases():
    """Test character name matching with edge cases."""
    fortune_teller = FortuneTeller()

    # Test various forms of the name
    assert fortune_teller.is_named("Fortune Teller") is True
    assert fortune_teller.is_named("fortune teller") is True
    assert fortune_teller.is_named("FORTUNE TELLER") is True
    assert fortune_teller.is_named("  Fortune Teller  ") is True

    # Test partial matches (should fail)
    assert fortune_teller.is_named("Fortune") is False
    assert fortune_teller.is_named("Teller") is False

    # Test completely wrong names
    assert fortune_teller.is_named("") is False
    assert fortune_teller.is_named("Not A Character") is False
