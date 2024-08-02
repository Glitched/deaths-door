from pydantic import BaseModel


class NightStep(BaseModel):
    """A step in the night."""

    name: str
    description: str
    always_show: bool = False
