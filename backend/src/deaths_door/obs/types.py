from pydantic import BaseModel


class VideoSettings(BaseModel):
    """The settings for the video."""

    baseHeight: int
    baseWidth: int
    fpsDenominator: int
    fpsNumerator: int
    outputHeight: int
    outputWidth: int


class Scene(BaseModel):
    """A scene in OBS."""

    sceneIndex: int
    sceneName: str
    sceneUuid: str


class Input(BaseModel):
    """An input in a scene."""

    inputUuid: str
    sceneItemId: int


class SceneItemTransform(BaseModel):
    """The transform of a scene item."""

    alignment: int
    boundsAlignment: int
    boundsHeight: float
    boundsType: str
    boundsWidth: float
    cropBottom: int
    cropLeft: int
    cropRight: int
    cropTop: int
    height: float
    positionX: float
    positionY: float
    rotation: float
    scaleX: float
    scaleY: float
    sourceHeight: float
    sourceWidth: float
    width: float
