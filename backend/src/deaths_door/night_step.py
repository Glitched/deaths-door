from pydantic import BaseModel, Field


class NightStep(BaseModel):
    """A step in the night phase for the storyteller to follow."""

    name: str = Field(
        ...,
        description="Character or role name for this night step",
        examples=["Poisoner", "Monk", "Fortune Teller", "Imp"],
    )
    description: str = Field(
        ...,
        description="Instructions for the storyteller during this night step",
        examples=[
            "The Poisoner points to a player: they are poisoned",
            "The Monk points to a player (not themselves): they are safe tonight",
        ],
    )
    always_show: bool = Field(
        False,
        description="If true, show this step even if the character is dead or not in play",
        examples=[False, True],
    )
