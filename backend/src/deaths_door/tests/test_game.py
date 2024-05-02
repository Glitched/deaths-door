import pytest

from deaths_door.game import Game, RoleDistribution
from deaths_door.script import ScriptName


@pytest.mark.anyio
async def test_add_role():
    """Asserts that we return a fun message."""
    game = Game(player_count=7, script_name=ScriptName.TROUBLE_BREWING)
    game.add_role("imp")
    assert game.get_current_role_counts() == RoleDistribution(
        townsfolk=0, outsiders=0, minions=0, demons=1
    )


@pytest.mark.anyio
async def test_roles_remaining():
    """Asserts that we return a fun message."""
    game = Game(player_count=7, script_name=ScriptName.TROUBLE_BREWING)
    assert game.get_open_slots() == RoleDistribution(
        townsfolk=5, outsiders=0, minions=1, demons=1
    )
    game.add_role("imp")
    assert game.get_open_slots() == RoleDistribution(
        townsfolk=5, outsiders=0, minions=1, demons=0
    )


@pytest.mark.anyio
async def test_role_count_change():
    """Asserts that we return a fun message."""
    game = Game(player_count=7, script_name=ScriptName.TROUBLE_BREWING)
    assert game.get_open_slots() == RoleDistribution(
        townsfolk=5, outsiders=0, minions=1, demons=1
    )
    game.add_role("baron")
    assert game.get_open_slots() == RoleDistribution(
        townsfolk=3, outsiders=2, minions=0, demons=1
    )
