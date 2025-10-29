from fastapi import APIRouter, Path
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..character import CharacterOut
from ..script import ScriptName
from ..scripts.registry import get_script_by_name


class ScriptListResponse(BaseModel):
    """Available scripts/editions with their display names."""

    scripts: dict[str, str] = Field(
        ...,
        description="Mapping of script IDs to their display names",
        examples=[
            {
                "trouble_brewing": "Trouble Brewing",
                "bad_moon_rising": "Bad Moon Rising",
                "sects_and_violets": "Sects & Violets",
            }
        ],
    )

router = APIRouter(prefix="/scripts", tags=["Scripts"])


@router.get("/list")
async def read_scripts() -> ScriptListResponse:
    """Return a list of available scripts."""
    return ScriptListResponse(scripts={x.value: str(x) for x in list(ScriptName)})


@router.get("/{script_name}/role", responses={404: {"description": "Script not found"}})
async def read_roles(
    script_name: str = Path(
        ...,
        description="Name of the script/edition",
        examples=["trouble_brewing"],
    )
) -> list[CharacterOut]:
    """List all character roles available in the given script."""
    script = get_script_by_name(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return [char.to_out() for char in script.characters]


@router.get("/{script_name}/travelers", responses={404: {"description": "Script not found"}})
async def read_travelers(
    script_name: str = Path(
        ...,
        description="Name of the script/edition",
        examples=["trouble_brewing"],
    )
) -> list[CharacterOut]:
    """List all traveler characters available in the given script."""
    script = get_script_by_name(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return [char.to_out() for char in script.travelers]


# TODO: Can we consolidate this into the method above?
# I don't have internet so I can't check the docs.
@router.get(
    "/{script_name}/role/{name}",
    responses={404: {"description": "Script or role not found"}},
)
async def read_role(
    script_name: str = Path(
        ...,
        description="Name of the script/edition",
        examples=["trouble_brewing"],
    ),
    role_name: str = Path(
        ...,
        description="Name of the character/role",
        examples=["Imp"],
        alias="name",
    ),
) -> CharacterOut:
    """Get details for a specific character role in a script."""
    script = get_script_by_name(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    char = script.get_character(role_name)

    if char is None:
        raise HTTPException(status_code=404, detail="Role not found")

    return char.to_out()
