//! DMX lighting control for moving head lights and fog machine.
//!
//! Talks to an FTDI-based OpenDMX interface over serial (via `serialport`).
//! OpenDMX cables are dumb — there is no DMX chip in them — so the host must
//! generate the DMX512 protocol itself: each frame is a BREAK (line held low),
//! a mark-after-break, then a 0x00 start code followed by the 512 channel
//! values, and frames must repeat continuously (fixtures treat signal loss as
//! an error and may blank or hold). To do that, public methods here only write
//! into a shadow copy of the universe; a dedicated transmitter thread owns the
//! serial port and streams the shadow universe ~40 times per second.
//!
//! Degrades gracefully to a no-op when no interface is connected: the shadow
//! universe is still updated (which keeps this testable), but no thread runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serialport::{DataBits, Parity, SerialPort, SerialPortType, StopBits};

use crate::lock::LockExt;

const DMX_BAUD: u32 = 250_000;
const FTDI_VID: u16 = 0x0403;
const UNIVERSE_SIZE: usize = 512;

/// Refresh interval for the transmitter thread (~40 frames/second; DMX
/// receivers expect roughly 20-44 Hz). One 513-byte frame at 250kbaud takes
/// ~23ms on the wire, so this mostly just paces the loop.
const FRAME_INTERVAL: Duration = Duration::from_millis(25);
/// BREAK length. The spec minimum is 88µs; longer is always legal, and OS
/// sleep granularity means we may well overshoot — that's fine.
const BREAK_DURATION: Duration = Duration::from_micros(200);
/// Mark-after-break length (spec minimum 8µs, may be up to 1s).
const MAB_DURATION: Duration = Duration::from_micros(20);

// XPCLEOYZ 60W moving head channel offsets (11-channel mode).
const OFF_PAN: usize = 0;
const OFF_PAN_FINE: usize = 1;
const OFF_TILT: usize = 2;
const OFF_TILT_FINE: usize = 3;
const OFF_COLOR: usize = 4;
const OFF_STROBE: usize = 7;
const OFF_DIMMER: usize = 8;

const LIGHT1_START: usize = 1;
const LIGHT2_START: usize = 12;
const FOG_START: usize = 23;

/// Color-wheel positions for the moving heads (from the fixture manual via the
/// original Python implementation; 10-139 are fixed colors, 140+ auto-cycles).
pub mod colors {
    pub const WHITE: i64 = 10;
    pub const RED: i64 = 20;
    /// Unverified wheel positions used by the drama scene since the Python days.
    pub const DRAMA_A: i64 = 30;
    pub const DRAMA_B: i64 = 40;
    pub const BLUE: i64 = 60;
    pub const AUTO_CYCLE: i64 = 140;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingScene {
    Death,
    Drama,
    Goodnight,
    Morning,
    Reveal,
    Blackout,
    Spotlight,
    Fog,
}

impl LightingScene {
    pub fn value(&self) -> &'static str {
        match self {
            LightingScene::Death => "death",
            LightingScene::Drama => "drama",
            LightingScene::Goodnight => "goodnight",
            LightingScene::Morning => "morning",
            LightingScene::Reveal => "reveal",
            LightingScene::Blackout => "blackout",
            LightingScene::Spotlight => "spotlight",
            LightingScene::Fog => "fog",
        }
    }

    pub const ALL: [LightingScene; 8] = [
        LightingScene::Death,
        LightingScene::Drama,
        LightingScene::Goodnight,
        LightingScene::Morning,
        LightingScene::Reveal,
        LightingScene::Blackout,
        LightingScene::Spotlight,
        LightingScene::Fog,
    ];

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<LightingScene> {
        let lower = name.to_lowercase();
        LightingScene::ALL.into_iter().find(|s| s.value() == lower)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerPosition {
    pub player_num: i64,
    pub pan: i64,
    pub tilt: i64,
}

fn find_ftdi_port() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    for port in ports {
        if let SerialPortType::UsbPort(info) = &port.port_type {
            let manu = info.manufacturer.clone().unwrap_or_default().to_uppercase();
            let product = info.product.clone().unwrap_or_default().to_uppercase();
            let is_ftdi = info.vid == FTDI_VID
                || manu.contains("FTDI")
                || product.contains("FTDI")
                || product.contains("FT232");
            if is_ftdi {
                tracing::info!("Found FTDI device: {} ({product})", port.port_name);
                return Some(port.port_name);
            }
        }
    }
    tracing::warn!("No FTDI device found");
    None
}

fn open_port() -> Result<(Box<dyn SerialPort>, String), String> {
    let port_name = find_ftdi_port().ok_or("No FTDI/OpenDMX device found")?;
    let port = serialport::new(&port_name, DMX_BAUD)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::Two)
        .timeout(Duration::from_secs(1))
        .open()
        .map_err(|e| format!("Failed to open serial port {port_name}: {e}"))?;
    Ok((port, port_name))
}

/// Send one DMX frame: BREAK, mark-after-break, then the start code and all
/// 512 channel values. `frame` must be the 513-byte wire frame (frame[0] is
/// the 0x00 start code).
fn send_frame(port: &mut dyn SerialPort, frame: &[u8]) -> Result<(), String> {
    port.set_break().map_err(|e| format!("set break: {e}"))?;
    std::thread::sleep(BREAK_DURATION);
    port.clear_break()
        .map_err(|e| format!("clear break: {e}"))?;
    std::thread::sleep(MAB_DURATION);
    port.write_all(frame).map_err(|e| format!("write: {e}"))?;
    // Drain the OS transmit buffer before returning, so the next frame's
    // BREAK can't cut this frame short.
    port.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

/// Transmitter loop: stream the shadow universe to the port at a fixed rate,
/// forever. Errors are logged once per outage rather than 40x/second.
fn run_transmitter(mut port: Box<dyn SerialPort>, universe: Arc<Mutex<[u8; UNIVERSE_SIZE]>>) {
    let mut frame = [0u8; UNIVERSE_SIZE + 1]; // frame[0] stays 0x00 (start code)
    let mut failing = false;
    loop {
        let started = Instant::now();
        frame[1..].copy_from_slice(&universe.lock_recover()[..]);
        match send_frame(port.as_mut(), &frame) {
            Ok(()) => {
                if failing {
                    tracing::info!("DMX transmission recovered");
                    failing = false;
                }
            }
            Err(e) => {
                if !failing {
                    tracing::warn!("DMX transmission failing (will keep retrying): {e}");
                    failing = true;
                }
            }
        }
        if let Some(remaining) = FRAME_INTERVAL.checked_sub(started.elapsed()) {
            std::thread::sleep(remaining);
        }
    }
}

struct PositionsInner {
    positions: HashMap<i64, PlayerPosition>,
    file: PathBuf,
}

/// Status snapshot for the `/lights/status` endpoint.
pub struct LightingStatus {
    pub connected: bool,
    pub serial_port: Option<String>,
    pub has_light1: bool,
    pub has_light2: bool,
    pub has_fog: bool,
    pub calibrated_positions: usize,
}

pub struct LightingManager {
    /// Shadow copy of the DMX universe; the transmitter thread streams it.
    universe: Arc<Mutex<[u8; UNIVERSE_SIZE]>>,
    /// Name of the serial port a transmitter thread is running on, if any.
    port_name: Option<String>,
    inner: Mutex<PositionsInner>,
}

impl LightingManager {
    pub fn new() -> Self {
        // DMX is write-only, so calibration lives in a JSON file next to the
        // server's other data (override with LIGHTING_POSITIONS_PATH).
        let positions_file = std::env::var("LIGHTING_POSITIONS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("assets/lighting_positions.json"));

        let universe = Arc::new(Mutex::new([0u8; UNIVERSE_SIZE]));
        let port_name = match open_port() {
            Ok((port, name)) => {
                let shadow = Arc::clone(&universe);
                let spawned = std::thread::Builder::new()
                    .name("dmx-transmitter".to_string())
                    .spawn(move || run_transmitter(port, shadow));
                match spawned {
                    Ok(_) => {
                        tracing::info!("Streaming DMX on {name}");
                        Some(name)
                    }
                    Err(e) => {
                        tracing::error!("Failed to spawn DMX transmitter thread: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize DMX controller: {e}. Continuing without lighting support."
                );
                None
            }
        };

        let positions = load_positions(&positions_file);

        LightingManager {
            universe,
            port_name,
            inner: Mutex::new(PositionsInner {
                positions,
                file: positions_file,
            }),
        }
    }

    pub fn status(&self) -> LightingStatus {
        let connected = self.port_name.is_some();
        LightingStatus {
            connected,
            serial_port: self.port_name.clone(),
            // DMX is one-way; per-fixture presence can't actually be probed.
            has_light1: connected,
            has_light2: connected,
            has_fog: connected,
            calibrated_positions: self.inner.lock_recover().positions.len(),
        }
    }

    pub fn list_scenes(&self) -> Vec<String> {
        LightingScene::ALL
            .iter()
            .map(|s| s.value().to_string())
            .collect()
    }

    /// A copy of the current shadow universe (`[0]` is DMX channel 1).
    /// Used by tests and debugging; the transmitter reads the live buffer.
    pub fn universe_snapshot(&self) -> [u8; UNIVERSE_SIZE] {
        *self.universe.lock_recover()
    }

    /// Map a fixture id to its DMX start channel and whether it's a moving head.
    fn fixture_start(fixture_id: i64) -> Option<(usize, bool)> {
        match fixture_id {
            1 => Some((LIGHT1_START, true)),
            2 => Some((LIGHT2_START, true)),
            3 => Some((FOG_START, false)),
            _ => None,
        }
    }

    /// Write one channel of the shadow universe (1-512), clamping to 0-255.
    fn write_channel(universe: &mut [u8; UNIVERSE_SIZE], channel: usize, value: i64) {
        if (1..=UNIVERSE_SIZE).contains(&channel) {
            universe[channel - 1] = value.clamp(0, 255) as u8;
        } else {
            tracing::warn!("Invalid DMX channel: {channel}");
        }
    }

    pub fn set_channel(&self, fixture_id: i64, channel: i64, value: i64) {
        let mut universe = self.universe.lock_recover();
        match Self::fixture_start(fixture_id) {
            // Moving heads span channels 1-11; the fog machine has a single
            // channel. Guard the ranges so `channel - 1` can't underflow on a
            // bad path param.
            Some((start, true)) if (1..=11).contains(&channel) => {
                Self::write_channel(&mut universe, start + (channel as usize - 1), value);
            }
            Some((start, false)) if channel == 1 => {
                Self::write_channel(&mut universe, start, value);
            }
            Some(_) => {
                tracing::warn!("DMX channel out of range for fixture {fixture_id}: {channel}")
            }
            None => tracing::warn!("Invalid fixture for channel control: {fixture_id}"),
        }
    }

    pub fn set_position(&self, fixture_id: i64, pan: i64, tilt: i64, fine: bool) {
        let mut universe = self.universe.lock_recover();
        if let Some((start, true)) = Self::fixture_start(fixture_id) {
            set_position(&mut universe, start, pan, tilt, fine);
        } else {
            tracing::warn!("Invalid fixture id for position: {fixture_id}");
        }
    }

    /// Kill all light and fog output (colors/positions are left as-is so a
    /// scene can fade back in from where it was).
    pub fn blackout(&self) {
        let mut universe = self.universe.lock_recover();
        for start in [LIGHT1_START, LIGHT2_START] {
            Self::write_channel(&mut universe, start + OFF_DIMMER, 0);
        }
        Self::write_channel(&mut universe, FOG_START, 0);
    }

    pub fn has_position(&self, player_num: i64) -> bool {
        self.inner
            .lock_recover()
            .positions
            .contains_key(&player_num)
    }

    pub fn save_player_position(&self, player_num: i64, pan: i64, tilt: i64) {
        let mut inner = self.inner.lock_recover();
        inner.positions.insert(
            player_num,
            PlayerPosition {
                player_num,
                pan,
                tilt,
            },
        );
        save_positions(&inner.file, &inner.positions);
    }

    pub fn get_all_positions(&self) -> HashMap<i64, PlayerPosition> {
        self.inner.lock_recover().positions.clone()
    }

    pub fn spotlight_player(&self, player_num: i64, brightness: i64, fixture_id: i64) {
        let position = match self
            .inner
            .lock_recover()
            .positions
            .get(&player_num)
            .copied()
        {
            Some(p) => p,
            None => {
                tracing::warn!("No saved position for player {player_num}");
                return;
            }
        };
        let mut universe = self.universe.lock_recover();
        if let Some((start, true)) = Self::fixture_start(fixture_id) {
            set_position(&mut universe, start, position.pan, position.tilt, true);
            // Spotlight look: white, requested brightness, no strobe.
            Self::write_channel(&mut universe, start + OFF_DIMMER, brightness);
            Self::write_channel(&mut universe, start + OFF_COLOR, colors::WHITE);
            Self::write_channel(&mut universe, start + OFF_STROBE, 0);
        } else {
            tracing::warn!("Invalid fixture id: {fixture_id}");
        }
    }

    /// Set color/dimmer/strobe on both moving heads at once. Heads can differ
    /// in color for asymmetric looks (e.g. the drama scene).
    pub fn set_heads(&self, colors: (i64, i64), dimmer: i64, strobe: i64) {
        let mut universe = self.universe.lock_recover();
        apply_light(&mut universe, LIGHT1_START, (colors.0, dimmer, strobe));
        apply_light(&mut universe, LIGHT2_START, (colors.1, dimmer, strobe));
    }

    /// Set the fog machine output (0 = off, 255 = full).
    pub fn set_fog(&self, intensity: i64) {
        let mut universe = self.universe.lock_recover();
        Self::write_channel(&mut universe, FOG_START, intensity);
    }
}

impl Default for LightingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Point a moving head. Positions are coarse-only (0-255); when `fine` is set,
/// the fine channels are zeroed so leftovers from manual channel fiddling
/// can't offset a recalled calibration.
fn set_position(universe: &mut [u8; UNIVERSE_SIZE], start: usize, pan: i64, tilt: i64, fine: bool) {
    LightingManager::write_channel(universe, start + OFF_PAN, pan);
    LightingManager::write_channel(universe, start + OFF_TILT, tilt);
    if fine {
        LightingManager::write_channel(universe, start + OFF_PAN_FINE, 0);
        LightingManager::write_channel(universe, start + OFF_TILT_FINE, 0);
    }
}

fn apply_light(
    universe: &mut [u8; UNIVERSE_SIZE],
    start: usize,
    (color, dimmer, strobe): (i64, i64, i64),
) {
    LightingManager::write_channel(universe, start + OFF_COLOR, color);
    LightingManager::write_channel(universe, start + OFF_DIMMER, dimmer);
    LightingManager::write_channel(universe, start + OFF_STROBE, strobe);
}

fn load_positions(file: &Path) -> HashMap<i64, PlayerPosition> {
    if !file.exists() {
        return HashMap::new();
    }
    match std::fs::read_to_string(file) {
        Ok(contents) => match serde_json::from_str::<HashMap<String, PlayerPosition>>(&contents) {
            Ok(raw) => raw
                .into_iter()
                .filter_map(|(k, v)| k.parse::<i64>().ok().map(|n| (n, v)))
                .collect(),
            Err(e) => {
                tracing::warn!("Failed to parse player positions: {e}");
                HashMap::new()
            }
        },
        Err(e) => {
            tracing::warn!("Failed to load player positions: {e}");
            HashMap::new()
        }
    }
}

fn save_positions(file: &Path, positions: &HashMap<i64, PlayerPosition>) {
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let raw: HashMap<String, PlayerPosition> =
        positions.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    match serde_json::to_string_pretty(&raw) {
        Ok(json) => {
            if let Err(e) = std::fs::write(file, json) {
                tracing::error!("Failed to save player positions: {e}");
            }
        }
        Err(e) => tracing::error!("Failed to serialize player positions: {e}"),
    }
}
