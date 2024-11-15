from enum import Enum


class Alignment(str, Enum):
    """The alignment of the role."""

    GOOD = "good"
    EVIL = "evil"
    UNKNOWN = "unknown"
