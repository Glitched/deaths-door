from fastapi import APIRouter
from fastapi.exceptions import HTTPException

from ..script import ScriptName
from ..scripts.registry import get_script_by_name

router = APIRouter(prefix="/scripts")


@router.get("/list")
async def read_scripts():
    """Return a list of available scripts."""
    return {x: str(x) for x in list(ScriptName)}


@router.get("/{script_name}/role")
async def read_roles(script_name: str):
    """List the roles for the given script."""
    script = get_script_by_name(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return [char.to_out() for char in script.characters]


# TODO: Can we consolidate this into the method above?
# I don't have internet so I can't check the docs.
@router.get("/{script_name}/role/{name}")
async def read_role(script_name: str, role_name: str):
    """Get a given role for a script."""
    script = get_script_by_name(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    char = script.get_character(role_name)

    if char is None:
        raise HTTPException(status_code=404, detail="Role not found")

    return char.to_out()
