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
from ..night_step import NightStep
from ..script import Script
from ..script_name import ScriptName
from ..travelers.trouble_brewing import (
    Beggar,
    Bureaucrat,
    Gunslinger,
    Scapegoat,
    Thief,
)


class TroubleBrewing(Script):
    """Script class representing Trouble Brewing."""

    name = ScriptName.TROUBLE_BREWING

    characters = [
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

    travelers = [
        Thief(),
        Bureaucrat(),
        Gunslinger(),
        Scapegoat(),
        Beggar(),
    ]

    first_night_steps = [
        NightStep(
            name="Dusk",
            description="Check that all eyes are closed. Some Travellers & Fabled act.",
            always_show=True,
        ),
        NightStep(
            name="Minion Info",
            description="If there are 7 or more players, wake all Minions: "
            + "Show the THIS IS THE DEMON token. Point to the Demon.",
            always_show=True,
        ),
        NightStep(
            name="Demon Info",
            description="If there are 7 or more players, wake the Demon: "
            + "Show the THESE ARE YOUR MINIONS token. Point to all Minions. "
            + "Show the THESE CHARACTERS ARE NOT IN PLAY token. "
            + "Show 3 not-in-play good character tokens.",
            always_show=True,
        ),
        NightStep(name="Poisoner", description="The Poisoner chooses a player.", always_show=False),
        NightStep(
            name="Spy",
            description="Show the Grimoire for as long as the Spy needs.",
            always_show=False,
        ),
        NightStep(
            name="Washerwoman",
            description="Show the Townsfolk character token. "
            + "Point to both the TOWNSFOLK and WRONG players.",
            always_show=False,
        ),
        NightStep(
            name="Librarian",
            description="Show the Outsider character token. "
            + "Point to both the OUTSIDER and WRONG players.",
            always_show=False,
        ),
        NightStep(
            name="Investigator",
            description="Show the Minion character token. "
            + "Point to both the MINION and WRONG players.",
            always_show=False,
        ),
        NightStep(name="Chef", description="Give a finger signal.", always_show=False),
        NightStep(name="Empath", description="Give a finger signal.", always_show=False),
        NightStep(
            name="Fortune Teller",
            description="The Fortune Teller chooses 2 players. "
            + "Nod if either is the Demon (or the RED HERRING).",
            always_show=False,
        ),
        NightStep(name="Butler", description="The Butler chooses a player.", always_show=False),
        NightStep(
            name="Dawn",
            description="Wait a few seconds. Call for eyes open.",
            always_show=True,
        ),
    ]

    other_night_steps = [
        NightStep(
            name="Dusk",
            description="Check that all eyes are closed. Some Travellers & Fabled act.",
            always_show=True,
        ),
        NightStep(name="Poisoner", description="The Poisoner chooses a player.", always_show=False),
        NightStep(name="Monk", description="The Monk chooses a player.", always_show=False),
        NightStep(
            name="Spy", description="Show the Grimoire for as long as the Spy needs.", always_show=False
        ),
        NightStep(
            name="Scarlet Woman",
            description="If the Scarlet Woman became the Imp today, "
            + "show them the YOU ARE token, then the Imp token.",
            always_show=False,
        ),
        NightStep(
            name="Imp",
            description=(
                "The Imp chooses a player. If the Imp chose themselves:\n"
                "Replace 1 alive Minion token with a spare Imp token.\n"
                "Put the old Imp to sleep. Wake the new Imp.\n"
                "Show the YOU ARE token, then show the Imp token."
            ),
            always_show=False,
        ),
        NightStep(
            name="Ravenkeeper",
            description="If the Ravenkeeper died tonight, "
            + "the Ravenkeeper chooses a player. "
            + "Show that player's character token.",
            always_show=False,
            show_when_dead=True,
        ),
        NightStep(
            name="Undertaker",
            description="If a player was executed today, show their character token.",
            always_show=False,
        ),
        NightStep(name="Empath", description="Give a finger signal.", always_show=False),
        NightStep(
            name="Fortune Teller",
            description="The Fortune Teller chooses 2 players. "
            + "Nod if either is the Demon (or the RED HERRING).",
            always_show=False,
        ),
        NightStep(name="Butler", description="The Butler chooses a player.", always_show=False),
        NightStep(
            name="Dawn",
            description="Wait a few seconds. "
            + "Call for eyes open & immediately say who died.",
            always_show=True,
        ),
    ]
