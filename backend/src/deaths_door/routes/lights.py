"""API routes for DMX lighting control."""

from typing import Any

from fastapi import APIRouter, HTTPException, Path
from pydantic import BaseModel, Field

from ..lighting import LightingManager, LightingScene
from ..sound_fx import SoundFX, SoundName

router = APIRouter(prefix="/lights", tags=["Lighting"])


@router.get("/status")
async def get_status() -> dict[str, Any]:
    """Get DMX connection status and detected devices."""
    manager = LightingManager()
    
    # Get serial port info if connected
    port_info = None
    if manager.controller and manager.controller.serial_port:
        port_info = {
            "port": manager.controller.serial_port.port,
            "is_open": manager.controller.serial_port.is_open,
        }
    
    return {
        "connected": manager.connected,
        "serial_port": port_info,
        "fixtures": {
            "light1": manager.light1 is not None,
            "light2": manager.light2 is not None,
            "fog": manager.fog is not None,
        },
        "calibrated_positions": len(manager.positions),
    }


# Response Models
class OperationResponse(BaseModel):
    """Response for successful operations."""

    status: str = Field(..., description="Operation status", examples=["success"])
    message: str = Field(
        ..., description="Operation message", examples=["Lights updated"]
    )


class SceneListResponse(BaseModel):
    """Response with list of available scenes."""

    scenes: list[str] = Field(..., description="List of available scene names")


class PositionResponse(BaseModel):
    """Response with player position information."""

    player_num: int = Field(..., description="Player number")
    pan: int = Field(..., description="Pan value (0-255)")
    tilt: int = Field(..., description="Tilt value (0-255)")


class PositionsListResponse(BaseModel):
    """Response with all saved positions."""

    positions: dict[int, dict[str, int]] = Field(
        ..., description="Dictionary of player positions"
    )


# Request Models
class ChannelSetRequest(BaseModel):
    """Request to set a channel value."""

    value: int = Field(..., description="Channel value (0-255)", ge=0, le=255)


class PositionSetRequest(BaseModel):
    """Request to set pan/tilt position."""

    pan: int = Field(..., description="Pan value (0-255)", ge=0, le=255)
    tilt: int = Field(..., description="Tilt value (0-255)", ge=0, le=255)
    fine: bool = Field(True, description="Use fine adjustment")


class CalibrationSaveRequest(BaseModel):
    """Request to save calibrated position."""

    pan: int = Field(..., description="Pan value (0-255)", ge=0, le=255)
    tilt: int = Field(..., description="Tilt value (0-255)", ge=0, le=255)


class SpotlightRequest(BaseModel):
    """Request to spotlight a player."""

    brightness: int = Field(
        255, description="Brightness level (0-255)", ge=0, le=255
    )
    fixture_id: int = Field(1, description="Fixture ID to use (1 or 2)", ge=1, le=2)


# Scene Control Endpoints
@router.get("/scenes/list")
async def list_scenes() -> SceneListResponse:
    """List all available lighting scenes."""
    manager = LightingManager()
    return SceneListResponse(scenes=manager.list_scenes())


@router.post(
    "/scene/{name}",
    responses={
        404: {"description": "Scene not found"},
        500: {"description": "Failed to trigger scene"},
    },
)
async def trigger_scene(
    name: str = Path(..., description="Name of the scene to trigger", examples=["death"])
) -> OperationResponse:
    """
    Trigger a lighting scene.

    This endpoint triggers only the lighting portion of a scene.
    For integrated light+sound scenes, use the /scenes/integrated/{name} endpoint.
    """
    scene = LightingScene.from_str(name)
    if scene is None:
        raise HTTPException(status_code=404, detail=f"Scene '{name}' not found")

    try:
        manager = LightingManager()
        manager.trigger_scene(name)
        return OperationResponse(
            status="success", message=f"Scene '{name}' triggered successfully"
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to trigger scene: {str(e)}"
        ) from e


@router.post(
    "/scene/integrated/{name}",
    responses={
        404: {"description": "Scene not found"},
        500: {"description": "Failed to trigger scene"},
    },
)
async def trigger_integrated_scene(
    name: str = Path(
        ...,
        description="Name of the integrated scene to trigger",
        examples=["death"],
    )
) -> OperationResponse:
    """
    Trigger an integrated light+sound scene.

    This endpoint triggers both lighting and sound effects simultaneously
    for a coordinated audio-visual experience.
    """
    # Check if scene exists for lighting
    lighting_scene = LightingScene.from_str(name)
    if lighting_scene is None:
        raise HTTPException(status_code=404, detail=f"Scene '{name}' not found")

    # Check if sound exists
    sound_name = SoundName.from_str(name)

    try:
        # Trigger lighting
        lighting_manager = LightingManager()
        lighting_manager.trigger_scene(name)

        # Trigger sound if available
        if sound_name is not None:
            sound_fx = SoundFX()
            sound_fx.play(sound_name)

        return OperationResponse(
            status="success",
            message=f"Integrated scene '{name}' triggered successfully",
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to trigger scene: {str(e)}"
        ) from e


# Granular Control Endpoints
@router.post("/fixture/{fixture_id}/channel/{channel}")
async def set_channel(
    fixture_id: int = Path(..., description="Fixture ID (1=Light1, 2=Light2, 3=Fog)", ge=1, le=3),
    channel: int = Path(..., description="Channel number (1-11)", ge=1, le=11),
    request: ChannelSetRequest = ...,
) -> OperationResponse:
    """
    Set a specific DMX channel value on a fixture.

    This provides granular control for manual adjustments and testing.

    Channel mapping for XPCLEOYZ Moving Head (11-channel mode):
    - 1: Pan Running
    - 2: Pan Fine
    - 3: Tilt Running
    - 4: Tilt Fine
    - 5: Color
    - 6: Gobo
    - 7: Fixed Gobo/Pattern
    - 8: Strobe
    - 9: Dimming
    - 10: Auto Mode
    - 11: Pan/Tilt Running/Reset
    """
    try:
        manager = LightingManager()
        manager.set_channel(fixture_id, channel, request.value)
        return OperationResponse(
            status="success",
            message=f"Fixture {fixture_id} channel {channel} set to {request.value}",
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to set channel: {str(e)}"
        ) from e


@router.post("/fixture/{fixture_id}/position")
async def set_position(
    fixture_id: int = Path(..., description="Fixture ID (1 or 2)", ge=1, le=2),
    request: PositionSetRequest = ...,
) -> OperationResponse:
    """
    Set pan/tilt position of a moving head fixture.

    This is a convenience endpoint for positioning lights without
    dealing with individual channels.
    """
    try:
        manager = LightingManager()
        manager.set_position(fixture_id, request.pan, request.tilt, request.fine)
        return OperationResponse(
            status="success",
            message=f"Fixture {fixture_id} positioned to pan={request.pan}, tilt={request.tilt}",
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to set position: {str(e)}"
        ) from e


@router.post("/blackout")
async def blackout() -> OperationResponse:
    """
    Turn off all lights immediately.

    This is an emergency blackout function that sets all dimmer channels to 0.
    """
    try:
        manager = LightingManager()
        manager.blackout()
        return OperationResponse(status="success", message="All lights blacked out")
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to blackout: {str(e)}"
        ) from e


# Calibration Endpoints
@router.post("/calibrate/player/{player_num}/save")
async def save_player_position(
    player_num: int = Path(..., description="Player number (chair position)", ge=1),
    request: CalibrationSaveRequest = ...,
) -> OperationResponse:
    """
    Save a calibrated position for a specific player.

    Use this after manually adjusting lights to point at a player's chair.
    The position can then be recalled quickly using the spotlight endpoint.

    Workflow:
    1. Use /fixture/{id}/position to manually adjust lights
    2. Once positioned correctly, call this endpoint to save the position
    3. Use /lights/spotlight/player/{num} to recall the saved position
    """
    try:
        manager = LightingManager()
        manager.save_player_position(player_num, request.pan, request.tilt)
        return OperationResponse(
            status="success",
            message=f"Position saved for player {player_num}",
        )
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to save position: {str(e)}"
        ) from e


@router.get("/calibrate/positions")
async def get_all_positions() -> PositionsListResponse:
    """
    Get all saved player positions.

    Returns a dictionary mapping player numbers to their calibrated pan/tilt positions.
    """
    try:
        manager = LightingManager()
        positions = manager.get_all_positions()
        return PositionsListResponse(positions=positions)
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to get positions: {str(e)}"
        ) from e


@router.post("/spotlight/player/{player_num}")
async def spotlight_player(
    player_num: int = Path(..., description="Player number to spotlight", ge=1),
    request: SpotlightRequest = SpotlightRequest(),
) -> OperationResponse:
    """
    Spotlight a specific player using their saved calibrated position.

    This will move the specified fixture to the saved position for the player
    and activate spotlight mode (white light, full brightness by default).

    The player position must be calibrated first using the calibration endpoints.
    """
    try:
        manager = LightingManager()

        # Check if position exists
        if player_num not in manager.positions:
            raise HTTPException(
                status_code=404,
                detail=f"No calibrated position found for player {player_num}. "
                "Please calibrate the position first.",
            )

        manager.spotlight_player(
            player_num, request.brightness, request.fixture_id
        )
        return OperationResponse(
            status="success",
            message=f"Spotlighting player {player_num} with fixture {request.fixture_id}",
        )
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(
            status_code=500, detail=f"Failed to spotlight player: {str(e)}"
        ) from e

