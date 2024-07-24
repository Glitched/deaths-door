import pytest

from deaths_door.game import Game
from deaths_door.script import ScriptName


@pytest.mark.anyio
async def test_add_role():
    """Asserts that we return a fun message."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.add_player_with_role("Ryan", "imp")
