"""DMX lighting control for moving head lights and fog machine."""

from __future__ import annotations

import asyncio
import json
import logging
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any, Callable

logger = logging.getLogger(__name__)

# Try to import serial for DMX control
try:
    import serial
    import serial.tools.list_ports
    SERIAL_AVAILABLE = True
except ImportError:
    logger.warning("pyserial not available. DMX support will be disabled.")
    SERIAL_AVAILABLE = False


class LightingScene(str, Enum):
    """Available lighting scenes."""

    # Game events
    DEATH = "death"
    DRAMA = "drama"
    GOODNIGHT = "goodnight"
    MORNING = "morning"
    REVEAL = "reveal"

    # Effects
    BLACKOUT = "blackout"
    SPOTLIGHT = "spotlight"
    FOG = "fog"

    @classmethod
    def from_str(cls, name: str) -> LightingScene | None:
        """Return the LightingScene for a given string if present, else return None."""
        for scene in cls:
            if scene.value == name.lower():
                return scene
        return None


class OpenDMXController:
    """
    Simple OpenDMX/FTDI USB to DMX controller.
    
    Communicates directly with FTDI-based OpenDMX interfaces using pyserial.
    """

    def __init__(self, port: str | None = None, baudrate: int = 250000):
        """
        Initialize the OpenDMX controller.

        Args:
            port: Serial port (e.g., 'COM3' on Windows, '/dev/ttyUSB0' on Linux)
                  If None, will attempt to auto-detect FTDI device
            baudrate: Baud rate for DMX (default 250000 for OpenDMX)
        """
        self.dmx_data = [0] * 512  # DMX universe: 512 channels
        self.serial_port = None
        
        if not SERIAL_AVAILABLE:
            raise RuntimeError("pyserial not available")
        
        # Auto-detect FTDI device if no port specified
        if port is None:
            port = self._find_ftdi_port()
        
        if port is None:
            raise RuntimeError("No FTDI/OpenDMX device found")
        
        # Open serial connection
        try:
            self.serial_port = serial.Serial(
                port=port,
                baudrate=baudrate,
                bytesize=serial.EIGHTBITS,
                parity=serial.PARITY_NONE,
                stopbits=serial.STOPBITS_TWO,
                timeout=1
            )
            logger.info(f"Successfully opened OpenDMX on port {port}")
        except Exception as e:
            raise RuntimeError(f"Failed to open serial port {port}: {e}") from e

    def _find_ftdi_port(self) -> str | None:
        """
        Auto-detect FTDI device.

        Returns:
            Port name if found, None otherwise
        """
        if not SERIAL_AVAILABLE:
            return None
            
        ports = serial.tools.list_ports.comports()
        for port in ports:
            # Look for FTDI devices in description, manufacturer, or VID
            # FTDI VID is 0403
            is_ftdi = (
                "FTDI" in (port.description or "").upper() or
                "FT232" in (port.description or "").upper() or
                "FTDI" in (port.manufacturer or "").upper() or
                "VID:PID=0403:" in (port.hwid or "").upper()
            )
            
            if is_ftdi:
                logger.info(f"Found FTDI device: {port.device} - {port.description} (Manufacturer: {port.manufacturer})")
                return port.device
        
        logger.warning("No FTDI device found")
        return None

    def set_channel(self, channel: int, value: int) -> None:
        """
        Set a DMX channel value.

        Args:
            channel: DMX channel number (1-512)
            value: Channel value (0-255)
        """
        if 1 <= channel <= 512:
            self.dmx_data[channel - 1] = max(0, min(255, value))
        else:
            logger.warning(f"Invalid DMX channel: {channel}")

    def render(self) -> None:
        """Send the current DMX data to the device."""
        if self.serial_port and self.serial_port.is_open:
            try:
                # OpenDMX protocol: send all 512 channels
                self.serial_port.write(bytes(self.dmx_data))
            except Exception as e:
                logger.error(f"Failed to send DMX data: {e}")

    def close(self) -> None:
        """Close the serial connection."""
        if self.serial_port and self.serial_port.is_open:
            self.serial_port.close()
            logger.info("Closed OpenDMX connection")


class XPCLEOYZMovingHead:
    """
    XPCLEOYZ 60W Moving Head Light fixture.

    11-channel DMX configuration from manual.
    """

    def __init__(self, controller: OpenDMXController, start_channel: int, name: str = ""):
        """
        Initialize the moving head fixture.

        Args:
            controller: OpenDMX controller instance
            start_channel: Starting DMX channel (1-512)
            name: Fixture name
        """
        self.controller = controller
        self.start_channel = start_channel
        self.name = name

    def set_channel(self, offset: int, value: int) -> None:
        """
        Set a channel relative to the fixture's start channel.

        Args:
            offset: Channel offset (0-10 for 11 channels)
            value: Channel value (0-255)
        """
        channel = self.start_channel + offset
        self.controller.set_channel(channel, value)
        self.controller.render()

    def set_position(self, pan: int, tilt: int, fine: bool = True) -> None:
        """
        Set pan and tilt position.

        Args:
            pan: Pan value (0-255)
            tilt: Tilt value (0-255)
            fine: Whether to use fine adjustment (default True)
        """
        self.set_channel(0, pan)  # Pan
        self.set_channel(2, tilt)  # Tilt
        if fine:
            self.set_channel(1, pan)  # Pan Fine
            self.set_channel(3, tilt)  # Tilt Fine

    def set_color(self, color: int) -> None:
        """
        Set color wheel position.

        Args:
            color: Color value (10-139: specific colors, 140-255: auto change)
        """
        self.set_channel(4, color)

    def set_dimmer(self, brightness: int) -> None:
        """
        Set dimmer/brightness.

        Args:
            brightness: Brightness value (0-255)
        """
        self.set_channel(8, brightness)

    def set_strobe(self, speed: int) -> None:
        """
        Set strobe effect.

        Args:
            speed: Strobe speed (0-255, 0 = off)
        """
        self.set_channel(7, speed)

    def blackout(self) -> None:
        """Turn off the light."""
        self.set_dimmer(0)

    def spotlight(self, brightness: int = 255) -> None:
        """Set to spotlight mode with white color."""
        self.set_dimmer(brightness)
        self.set_color(10)  # First fixed color (typically white)
        self.set_strobe(0)  # No strobe


class FogMachineStub:
    """Stub fixture for fog machine - to be configured later."""

    def __init__(self, controller: OpenDMXController, start_channel: int, name: str = ""):
        """
        Initialize the fog machine stub.

        Args:
            controller: OpenDMX controller instance
            start_channel: Starting DMX channel
            name: Fixture name
        """
        self.controller = controller
        self.start_channel = start_channel
        self.name = name

    def set_intensity(self, value: int) -> None:
        """Set fog intensity."""
        self.controller.set_channel(self.start_channel, value)
        self.controller.render()


class PlayerPosition:
    """Represents a calibrated position for spotlighting a player."""

    def __init__(self, player_num: int, pan: int, tilt: int):
        """
        Initialize a player position.

        Args:
            player_num: Player number (chair position)
            pan: Pan value (0-255)
            tilt: Tilt value (0-255)
        """
        self.player_num = player_num
        self.pan = pan
        self.tilt = tilt

    def to_dict(self) -> dict[str, int]:
        """Convert to dictionary for JSON serialization."""
        return {"player_num": self.player_num, "pan": self.pan, "tilt": self.tilt}

    @classmethod
    def from_dict(cls, data: dict[str, int]) -> PlayerPosition:
        """Create from dictionary."""
        return cls(
            player_num=data["player_num"], pan=data["pan"], tilt=data["tilt"]
        )


@dataclass
class LightingCue:
    """
    Represents a single lighting cue in a timeline.

    A cue defines what should happen at a specific time during a sequence.
    """

    timestamp: float  # Time in seconds from start of sequence
    action: Callable[[], None]  # Function to execute at this timestamp
    description: str  # Human-readable description of the cue

    def __lt__(self, other: LightingCue) -> bool:
        """Compare cues by timestamp for sorting."""
        return self.timestamp < other.timestamp


class LightingSequence:
    """
    Represents a timeline-based sequence of lighting changes.

    This enables advanced effects like:
    - Gradual color transitions
    - Timed strobe patterns
    - Coordinated movements synchronized with audio
    - Multi-step dramatic reveals
    """

    def __init__(self, name: str):
        """
        Initialize a lighting sequence.

        Args:
            name: Name of the sequence
        """
        self.name = name
        self.cues: list[LightingCue] = []
        self.is_running = False
        self._task: asyncio.Task[None] | None = None

    def add_cue(
        self, timestamp: float, action: Callable[[], None], description: str = ""
    ) -> None:
        """
        Add a cue to the sequence.

        Args:
            timestamp: Time in seconds from start
            action: Function to execute
            description: Description of the cue
        """
        cue = LightingCue(timestamp, action, description)
        self.cues.append(cue)
        # Keep cues sorted by timestamp
        self.cues.sort()

    def clear_cues(self) -> None:
        """Clear all cues from the sequence."""
        self.cues = []

    async def play(self) -> None:
        """
        Play the sequence asynchronously.

        Executes each cue at its scheduled time.
        """
        if self.is_running:
            logger.warning(f"Sequence {self.name} is already running")
            return

        self.is_running = True
        start_time = asyncio.get_event_loop().time()

        try:
            for cue in self.cues:
                if not self.is_running:
                    break

                # Calculate how long to wait until this cue
                current_time = asyncio.get_event_loop().time()
                elapsed = current_time - start_time
                wait_time = cue.timestamp - elapsed

                if wait_time > 0:
                    await asyncio.sleep(wait_time)

                # Execute the cue
                logger.debug(f"Executing cue: {cue.description} at {cue.timestamp}s")
                try:
                    cue.action()
                except Exception as e:
                    logger.error(f"Error executing cue {cue.description}: {e}")

        finally:
            self.is_running = False

    def stop(self) -> None:
        """Stop the sequence if it's running."""
        self.is_running = False
        if self._task and not self._task.done():
            self._task.cancel()

    def start(self) -> asyncio.Task[None]:
        """
        Start playing the sequence in the background.

        Returns:
            The asyncio task running the sequence
        """
        self._task = asyncio.create_task(self.play())
        return self._task


class LightingManager:
    """
    Singleton manager for DMX lighting control.

    Manages moving head lights and fog machine with graceful fallback if hardware unavailable.
    """

    _instance: None | LightingManager = None

    controller: OpenDMXController | None
    light1: XPCLEOYZMovingHead | None
    light2: XPCLEOYZMovingHead | None
    fog: FogMachineStub | None
    connected: bool
    positions: dict[int, PlayerPosition]
    positions_file: Path
    sequences: dict[str, LightingSequence]
    active_sequence: LightingSequence | None

    def __new__(cls) -> LightingManager:
        """Return the existing instance of the class, if present."""
        if cls._instance is None:
            cls._instance = super(LightingManager, cls).__new__(cls)
        return cls._instance

    def __init__(self) -> None:
        """Initialize the DMX controller and fixtures."""
        # Only initialize once
        if hasattr(self, "connected"):
            return

        self.connected = False
        self.controller = None
        self.light1 = None
        self.light2 = None
        self.fog = None
        self.positions = {}
        self.positions_file = Path("src/assets/lighting_positions.json")
        self.sequences = {}
        self.active_sequence = None

        # Try to initialize DMX controller
        try:
            if not SERIAL_AVAILABLE:
                raise RuntimeError("pyserial library not available")
            
            self.controller = OpenDMXController()
            self.connected = True
            logger.info("Successfully initialized DMX controller")

            # Add fixtures at specific DMX addresses
            # Light 1: channels 1-11
            self.light1 = XPCLEOYZMovingHead(
                controller=self.controller,
                start_channel=1,
                name="Light1"
            )

            # Light 2: channels 12-22
            self.light2 = XPCLEOYZMovingHead(
                controller=self.controller,
                start_channel=12,
                name="Light2"
            )

            # Fog machine stub: channel 23+ (to be configured later)
            self.fog = FogMachineStub(
                controller=self.controller,
                start_channel=23,
                name="Fog"
            )

            logger.info("Successfully added lighting fixtures")
        except Exception as e:
            logger.warning(
                f"Failed to initialize DMX controller: {e}. "
                "Continuing without lighting support."
            )
            self.connected = False

        # Load calibrated positions
        self._load_positions()

    def _load_positions(self) -> None:
        """Load calibrated player positions from file."""
        if self.positions_file.exists():
            try:
                with open(self.positions_file) as f:
                    data = json.load(f)
                    self.positions = {
                        int(k): PlayerPosition.from_dict(v)
                        for k, v in data.items()
                    }
                logger.info(f"Loaded {len(self.positions)} player positions")
            except Exception as e:
                logger.warning(f"Failed to load player positions: {e}")
                self.positions = {}
        else:
            logger.info("No player positions file found, starting with empty positions")

    def _save_positions(self) -> None:
        """Save calibrated player positions to file."""
        try:
            # Ensure directory exists
            self.positions_file.parent.mkdir(parents=True, exist_ok=True)

            with open(self.positions_file, "w") as f:
                data = {str(k): v.to_dict() for k, v in self.positions.items()}
                json.dump(data, f, indent=2)
            logger.info(f"Saved {len(self.positions)} player positions")
        except Exception as e:
            logger.error(f"Failed to save player positions: {e}")

    def set_channel(self, fixture_id: int, channel: int, value: int) -> None:
        """
        Set a specific channel value on a fixture.

        Args:
            fixture_id: Fixture ID (1 or 2 for lights, 3 for fog)
            channel: Channel number (1-11 for moving heads)
            value: Channel value (0-255)
        """
        if not self.connected:
            logger.debug("DMX not connected, skipping channel set")
            return

        fixture = self._get_fixture(fixture_id)
        if fixture is None:
            logger.warning(f"Invalid fixture ID: {fixture_id}")
            return

        if isinstance(fixture, XPCLEOYZMovingHead):
            # Channel 1-11 maps to offset 0-10
            fixture.set_channel(channel - 1, value)
        else:
            logger.warning(f"Invalid fixture type for channel control: {fixture_id}")

    def set_position(
        self, fixture_id: int, pan: int, tilt: int, fine: bool = True
    ) -> None:
        """
        Set pan/tilt position of a fixture.

        Args:
            fixture_id: Fixture ID (1 or 2)
            pan: Pan value (0-255)
            tilt: Tilt value (0-255)
            fine: Whether to use fine adjustment
        """
        if not self.connected:
            logger.debug("DMX not connected, skipping position set")
            return

        fixture = self._get_fixture(fixture_id)
        if fixture and isinstance(fixture, XPCLEOYZMovingHead):
            fixture.set_position(pan, tilt, fine)
        else:
            logger.warning(f"Invalid fixture ID for position: {fixture_id}")

    def blackout(self) -> None:
        """Turn off all lights."""
        if not self.connected:
            return

        if self.light1:
            self.light1.blackout()
        if self.light2:
            self.light2.blackout()

    def save_player_position(self, player_num: int, pan: int, tilt: int) -> None:
        """
        Save a calibrated position for a player.

        Args:
            player_num: Player number (chair position)
            pan: Pan value (0-255)
            tilt: Tilt value (0-255)
        """
        self.positions[player_num] = PlayerPosition(player_num, pan, tilt)
        self._save_positions()
        logger.info(f"Saved position for player {player_num}")

    def spotlight_player(
        self, player_num: int, brightness: int = 255, fixture_id: int = 1
    ) -> None:
        """
        Spotlight a specific player using saved position.

        Args:
            player_num: Player number to spotlight
            brightness: Brightness level (0-255)
            fixture_id: Which light to use (1 or 2)
        """
        if not self.connected:
            logger.debug("DMX not connected, skipping spotlight")
            return

        if player_num not in self.positions:
            logger.warning(f"No saved position for player {player_num}")
            return

        position = self.positions[player_num]
        fixture = self._get_fixture(fixture_id)

        if fixture and isinstance(fixture, XPCLEOYZMovingHead):
            fixture.set_position(position.pan, position.tilt)
            fixture.spotlight(brightness)
            logger.info(f"Spotlighting player {player_num} with fixture {fixture_id}")
        else:
            logger.warning(f"Invalid fixture ID: {fixture_id}")

    def get_all_positions(self) -> dict[int, dict[str, int]]:
        """Get all saved player positions."""
        return {k: v.to_dict() for k, v in self.positions.items()}

    def trigger_scene(self, scene_name: str) -> None:
        """
        Trigger a predefined lighting scene.

        Args:
            scene_name: Name of the scene to trigger
        """
        if not self.connected:
            logger.debug("DMX not connected, skipping scene trigger")
            return

        scene = LightingScene.from_str(scene_name)
        if scene is None:
            logger.warning(f"Unknown scene: {scene_name}")
            return

        logger.info(f"Triggering scene: {scene_name}")

        if scene == LightingScene.BLACKOUT:
            self.blackout()
        elif scene == LightingScene.DEATH:
            self._scene_death()
        elif scene == LightingScene.DRAMA:
            self._scene_drama()
        elif scene == LightingScene.GOODNIGHT:
            self._scene_goodnight()
        elif scene == LightingScene.MORNING:
            self._scene_morning()
        elif scene == LightingScene.REVEAL:
            self._scene_reveal()

    def _scene_death(self) -> None:
        """Death scene: Red color, dramatic lighting."""
        if self.light1:
            self.light1.set_color(20)  # Red color
            self.light1.set_dimmer(255)
            self.light1.set_strobe(0)

        if self.light2:
            self.light2.set_color(20)
            self.light2.set_dimmer(255)
            self.light2.set_strobe(0)

    def _scene_drama(self) -> None:
        """Drama scene: Dramatic colors with slow strobe."""
        if self.light1:
            self.light1.set_color(30)  # Dramatic color
            self.light1.set_dimmer(200)
            self.light1.set_strobe(50)  # Slow strobe

        if self.light2:
            self.light2.set_color(40)
            self.light2.set_dimmer(200)
            self.light2.set_strobe(50)

    def _scene_goodnight(self) -> None:
        """Goodnight scene: Dim blue, calm atmosphere."""
        if self.light1:
            self.light1.set_color(60)  # Blue color
            self.light1.set_dimmer(100)
            self.light1.set_strobe(0)

        if self.light2:
            self.light2.set_color(60)
            self.light2.set_dimmer(100)
            self.light2.set_strobe(0)

    def _scene_morning(self) -> None:
        """Morning scene: Bright white, energetic."""
        if self.light1:
            self.light1.set_color(10)  # White
            self.light1.set_dimmer(255)
            self.light1.set_strobe(0)

        if self.light2:
            self.light2.set_color(10)
            self.light2.set_dimmer(255)
            self.light2.set_strobe(0)

    def _scene_reveal(self) -> None:
        """Reveal scene: Dramatic spotlight effect."""
        if self.light1:
            self.light1.set_color(140)  # Auto color change
            self.light1.set_dimmer(255)
            self.light1.set_strobe(0)

        if self.light2:
            self.light2.set_color(140)
            self.light2.set_dimmer(255)
            self.light2.set_strobe(0)

    def _get_fixture(self, fixture_id: int) -> XPCLEOYZMovingHead | FogMachineStub | None:
        """Get fixture by ID."""
        if fixture_id == 1:
            return self.light1
        elif fixture_id == 2:
            return self.light2
        elif fixture_id == 3:
            return self.fog
        return None

    def list_scenes(self) -> list[str]:
        """List all available scenes."""
        return [scene.value for scene in LightingScene]

    # Timeline/Sequence Methods
    def create_sequence(self, name: str) -> LightingSequence:
        """
        Create a new lighting sequence.

        Args:
            name: Name of the sequence

        Returns:
            The created sequence
        """
        sequence = LightingSequence(name)
        self.sequences[name] = sequence
        return sequence

    def get_sequence(self, name: str) -> LightingSequence | None:
        """
        Get a sequence by name.

        Args:
            name: Name of the sequence

        Returns:
            The sequence if found, None otherwise
        """
        return self.sequences.get(name)

    def play_sequence(self, name: str) -> asyncio.Task[None] | None:
        """
        Play a named sequence.

        Args:
            name: Name of the sequence to play

        Returns:
            The task running the sequence, or None if sequence not found
        """
        sequence = self.get_sequence(name)
        if sequence is None:
            logger.warning(f"Sequence '{name}' not found")
            return None

        # Stop any currently active sequence
        if self.active_sequence and self.active_sequence.is_running:
            self.active_sequence.stop()

        self.active_sequence = sequence
        return sequence.start()

    def stop_sequence(self) -> None:
        """Stop the currently active sequence."""
        if self.active_sequence:
            self.active_sequence.stop()
            self.active_sequence = None

    def create_example_sequence(self) -> None:
        """
        Create an example dramatic reveal sequence.

        This demonstrates how to use the timeline system for complex effects.
        Can be used as a template for creating custom sequences.
        """
        if not self.connected:
            return

        seq = self.create_sequence("dramatic_reveal")

        # 0.0s: Start with blackout
        seq.add_cue(0.0, lambda: self.blackout(), "Blackout")

        # 1.0s: Dim red glow
        def dim_red() -> None:
            if self.light1:
                self.light1.set_color(20)
                self.light1.set_dimmer(50)
            if self.light2:
                self.light2.set_color(20)
                self.light2.set_dimmer(50)

        seq.add_cue(1.0, dim_red, "Dim red glow")

        # 2.5s: Increase brightness
        def brighter() -> None:
            if self.light1:
                self.light1.set_dimmer(150)
            if self.light2:
                self.light2.set_dimmer(150)

        seq.add_cue(2.5, brighter, "Increase brightness")

        # 4.0s: Add strobe effect
        def strobe() -> None:
            if self.light1:
                self.light1.set_strobe(100)
            if self.light2:
                self.light2.set_strobe(100)

        seq.add_cue(4.0, strobe, "Add strobe")

        # 5.0s: Full brightness, color change
        def reveal() -> None:
            if self.light1:
                self.light1.set_color(140)  # Auto color
                self.light1.set_dimmer(255)
                self.light1.set_strobe(0)
            if self.light2:
                self.light2.set_color(140)
                self.light2.set_dimmer(255)
                self.light2.set_strobe(0)

        seq.add_cue(5.0, reveal, "Full reveal")

        logger.info("Created example 'dramatic_reveal' sequence")
