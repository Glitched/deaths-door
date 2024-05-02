from fastapi import APIRouter
from fastapi.exceptions import HTTPException

from ..script import Script, ScriptName

router = APIRouter()


@router.get("/scripts/list")
async def read_scripts():
    """Return a list of available scripts."""
    return {x: str(x) for x in list(ScriptName)}


@router.get("/scripts/{script_name}/role")
async def read_roles(script_name: str):
    """List the roles for the given script."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.roles


# TODO: Can we consolidate this into the method above?
# I don't have internet so I can't check the docs.
@router.get("/scripts/{script}/role/{name}")
async def read_role(script_name: str, role_name: str):
    """Get a given role for a script."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.get_role(role_name)
