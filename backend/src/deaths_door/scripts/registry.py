from ..script import Script
from ..script_name import ScriptName
from .trouble_brewing import TroubleBrewing


def get_script_by_name(name: str) -> Script | None:
    """Return the Script for a given string if present, else return none."""
    match ScriptName.from_str(name):
        case ScriptName.TROUBLE_BREWING:
            return TroubleBrewing()
        case _:
            return None
