from ..characters.trouble_brewing import (
    Baron,
    Butler,
    Chef,
    Drunk,
    Empath,
    FortuneTeller,
    Imp,
    Investigator,
    Librarian,
    Mayor,
    Monk,
    Poisoner,
    Ravenkeeper,
    Recluse,
    Saint,
    ScarletWoman,
    Slayer,
    Soldier,
    Spy,
    Undertaker,
    Virgin,
    Washerwoman,
)
from ..script import Script, ScriptName


class TroubleBrewing(Script):
    """Script class representing Trouble Brewing."""

    def __init__(self) -> None:
        """Create a new Trouble Brewing script."""
        self.name = ScriptName.TROUBLE_BREWING

        self.characters = [
            Washerwoman(),
            Librarian(),
            Investigator(),
            Chef(),
            Empath(),
            FortuneTeller(),
            Undertaker(),
            Monk(),
            Ravenkeeper(),
            Virgin(),
            Slayer(),
            Soldier(),
            Mayor(),
            Butler(),
            Drunk(),
            Recluse(),
            Saint(),
            Poisoner(),
            Spy(),
            Baron(),
            ScarletWoman(),
            Imp(),
        ]
