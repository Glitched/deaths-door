from pydantic import BaseModel, Field


class StatusEffectOut(BaseModel):
    """A single status effect applied to a character."""

    name: str = Field(
        ...,
        description="Name of the status effect",
        examples=["Poisoned", "Safe", "Is the Drunk", "Butler's Master"],
    )
    character_name: str = Field(
        ...,
        description="Name of the character this status effect is attached to",
        examples=["Imp", "Chef", "Butler"],
    )


class StatusEffect:
    """A status effect that can be applied to a character."""

    name: str

    def to_out(self, character_name: str) -> StatusEffectOut:
        """Convert the status effect to an output model."""
        return StatusEffectOut(name=self.name, character_name=character_name)


class IsTheDrunk(StatusEffect):
    """The Is the Drunk status effect."""

    name = "Is the Drunk"


class Poisoned(StatusEffect):
    """The Poisoned status effect."""

    name = "Poisoned"


class Safe(StatusEffect):
    """The Safe status effect."""

    name = "Safe"


class ButlersMaster(StatusEffect):
    """The Butler's Master status effect."""

    name = "Butler's Master"


class Dead(StatusEffect):
    """The Dead status effect."""

    name = "Dead"


class DiedToday(StatusEffect):
    """The Died Today status effect."""

    name = "Died Today"


class InvestigatorMinion(StatusEffect):
    """The Investigator Minion status effect."""

    name = "Investigator's Minion"


class InvestigatorWrong(StatusEffect):
    """The Investigator Wrong status effect."""

    name = "Investigator's Wrong"


class IsTheDemon(StatusEffect):
    """The Is the Demon status effect."""

    name = "Is the Demon"


class NoAbility(StatusEffect):
    """The No Ability status effect."""

    name = "No Ability"


class RedHerring(StatusEffect):
    """The Red Herring status effect."""

    name = "Fortune Teller's Red Herring"


class LibrarianOutsider(StatusEffect):
    """The Librarian Outsider status effect."""

    name = "Librarian's Outsider"


class LibrarianWrong(StatusEffect):
    """The Librarian Wrong status effect."""

    name = "Librarian's Wrong"


class WasherwomanTownsfolk(StatusEffect):
    """The Washerwoman Townsfolk status effect."""

    name = "Washerwoman's Townsfolk"


class WasherwomanWrong(StatusEffect):
    """The Washerwoman Wrong status effect."""

    name = "Washerwoman's Wrong"
