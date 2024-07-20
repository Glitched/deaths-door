from pydantic import BaseModel


class Changes(BaseModel):
    """Represents the changes to the number of roles in the Script."""

    townsfolk: None | int = None
    outsider: None | int = None
    minion: None | int = None
    demon: None | int = None
