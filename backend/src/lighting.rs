//! DMX lighting control for moving head lights and fog machine.
//!
//! Talks to an FTDI-based OpenDMX interface over serial (via `serialport`),
//! mirroring the Python implementation. Degrades gracefully to a no-op when no
//! interface is connected. The Python `LightingSequence` timeline system is not
//! ported because no HTTP route reaches it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serialport::{DataBits, Parity, SerialPort, SerialPortType, StopBits};

use crate::lock::LockExt;

const DMX_BAUD: u32 = 250_000;
const FTDI_VID: u16 = 0x0403;

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

/// Direct OpenDMX/FTDI serial controller holding a 512-channel DMX universe.
struct OpenDMXController {
    dmx_data: [u8; 512],
    port: Box<dyn SerialPort>,
    port_name: String,
}

impl OpenDMXController {
    fn open() -> Result<Self, String> {
        let port_name = find_ftdi_port().ok_or("No FTDI/OpenDMX device found")?;
        let port = serialport::new(&port_name, DMX_BAUD)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::Two)
            .timeout(Duration::from_secs(1))
            .open()
            .map_err(|e| format!("Failed to open serial port {port_name}: {e}"))?;
        tracing::info!("Successfully opened OpenDMX on port {port_name}");
        Ok(OpenDMXController {
            dmx_data: [0u8; 512],
            port,
            port_name,
        })
    }

    /// Set a DMX channel value (1-512), clamped to 0-255.
    fn set_channel(&mut self, channel: usize, value: i64) {
        if (1..=512).contains(&channel) {
            self.dmx_data[channel - 1] = value.clamp(0, 255) as u8;
        } else {
            tracing::warn!("Invalid DMX channel: {channel}");
        }
    }

    /// Send the current DMX universe to the device.
    fn render(&mut self) {
        if let Err(e) = self.port.write_all(&self.dmx_data) {
            tracing::error!("Failed to send DMX data: {e}");
        }
    }
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

struct LightingInner {
    controller: Option<OpenDMXController>,
    positions: HashMap<i64, PlayerPosition>,
    positions_file: PathBuf,
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
    inner: Mutex<LightingInner>,
}

impl LightingManager {
    pub fn new() -> Self {
        let positions_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("lighting_positions.json");

        let controller = match OpenDMXController::open() {
            Ok(c) => {
                tracing::info!("Successfully initialized DMX controller and fixtures");
                Some(c)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize DMX controller: {e}. Continuing without lighting support.");
                None
            }
        };

        let positions = load_positions(&positions_file);

        LightingManager {
            inner: Mutex::new(LightingInner {
                controller,
                positions,
                positions_file,
            }),
        }
    }

    pub fn connected(&self) -> bool {
        self.inner.lock_recover().controller.is_some()
    }

    pub fn status(&self) -> LightingStatus {
        let inner = self.inner.lock_recover();
        let connected = inner.controller.is_some();
        LightingStatus {
            connected,
            serial_port: inner.controller.as_ref().map(|c| c.port_name.clone()),
            has_light1: connected,
            has_light2: connected,
            has_fog: connected,
            calibrated_positions: inner.positions.len(),
        }
    }

    pub fn list_scenes(&self) -> Vec<String> {
        LightingScene::ALL
            .iter()
            .map(|s| s.value().to_string())
            .collect()
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

    pub fn set_channel(&self, fixture_id: i64, channel: i64, value: i64) {
        let mut inner = self.inner.lock_recover();
        let Some(controller) = inner.controller.as_mut() else {
            return;
        };
        match Self::fixture_start(fixture_id) {
            // Channels 1-11 map to offsets 0-10; guard the range so the
            // `channel - 1` offset can't underflow on a bad path param.
            Some((start, true)) if (1..=11).contains(&channel) => {
                controller.set_channel(start + (channel as usize - 1), value);
                controller.render();
            }
            Some((_, true)) => {
                tracing::warn!("DMX channel out of range (1-11): {channel}")
            }
            _ => tracing::warn!("Invalid fixture for channel control: {fixture_id}"),
        }
    }

    pub fn set_position(&self, fixture_id: i64, pan: i64, tilt: i64, fine: bool) {
        let mut inner = self.inner.lock_recover();
        let Some(controller) = inner.controller.as_mut() else {
            return;
        };
        if let Some((start, true)) = Self::fixture_start(fixture_id) {
            set_position(controller, start, pan, tilt, fine);
        } else {
            tracing::warn!("Invalid fixture id for position: {fixture_id}");
        }
    }

    pub fn blackout(&self) {
        let mut inner = self.inner.lock_recover();
        let Some(controller) = inner.controller.as_mut() else {
            return;
        };
        for start in [LIGHT1_START, LIGHT2_START] {
            controller.set_channel(start + OFF_DIMMER, 0);
        }
        controller.render();
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
        let file = inner.positions_file.clone();
        let positions = inner.positions.clone();
        save_positions(&file, &positions);
    }

    pub fn get_all_positions(&self) -> HashMap<i64, PlayerPosition> {
        self.inner.lock_recover().positions.clone()
    }

    pub fn spotlight_player(&self, player_num: i64, brightness: i64, fixture_id: i64) {
        let mut inner = self.inner.lock_recover();
        let position = match inner.positions.get(&player_num).copied() {
            Some(p) => p,
            None => {
                tracing::warn!("No saved position for player {player_num}");
                return;
            }
        };
        let Some(controller) = inner.controller.as_mut() else {
            return;
        };
        if let Some((start, true)) = Self::fixture_start(fixture_id) {
            set_position(controller, start, position.pan, position.tilt, true);
            // spotlight: white color, full dimmer, no strobe.
            controller.set_channel(start + OFF_DIMMER, brightness);
            controller.set_channel(start + OFF_COLOR, 10);
            controller.set_channel(start + OFF_STROBE, 0);
            controller.render();
        } else {
            tracing::warn!("Invalid fixture id: {fixture_id}");
        }
    }

    pub fn trigger_scene(&self, scene_name: &str) {
        let Some(scene) = LightingScene::from_str(scene_name) else {
            tracing::warn!("Unknown scene: {scene_name}");
            return;
        };
        if scene == LightingScene::Blackout {
            self.blackout();
            return;
        }

        let mut inner = self.inner.lock_recover();
        let Some(controller) = inner.controller.as_mut() else {
            return;
        };
        tracing::info!("Triggering scene: {scene_name}");

        // (color, dimmer, strobe) applied to both moving heads.
        let settings: Option<[(i64, i64, i64); 2]> = match scene {
            LightingScene::Death => Some([(20, 255, 0), (20, 255, 0)]),
            LightingScene::Drama => Some([(30, 200, 50), (40, 200, 50)]),
            LightingScene::Goodnight => Some([(60, 100, 0), (60, 100, 0)]),
            LightingScene::Morning => Some([(10, 255, 0), (10, 255, 0)]),
            LightingScene::Reveal => Some([(140, 255, 0), (140, 255, 0)]),
            _ => None,
        };

        if let Some([s1, s2]) = settings {
            apply_light(controller, LIGHT1_START, s1);
            apply_light(controller, LIGHT2_START, s2);
            controller.render();
        }
    }
}

impl Default for LightingManager {
    fn default() -> Self {
        Self::new()
    }
}

fn set_position(controller: &mut OpenDMXController, start: usize, pan: i64, tilt: i64, fine: bool) {
    controller.set_channel(start + OFF_PAN, pan);
    controller.set_channel(start + OFF_TILT, tilt);
    if fine {
        controller.set_channel(start + OFF_PAN_FINE, pan);
        controller.set_channel(start + OFF_TILT_FINE, tilt);
    }
    controller.render();
}

fn apply_light(
    controller: &mut OpenDMXController,
    start: usize,
    (color, dimmer, strobe): (i64, i64, i64),
) {
    controller.set_channel(start + OFF_COLOR, color);
    controller.set_channel(start + OFF_DIMMER, dimmer);
    controller.set_channel(start + OFF_STROBE, strobe);
}

fn load_positions(file: &PathBuf) -> HashMap<i64, PlayerPosition> {
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

fn save_positions(file: &PathBuf, positions: &HashMap<i64, PlayerPosition>) {
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
