use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::audio::dsp::{DspSharedParams, DspSnapshot};
use crate::audio::engine::{self, AudioEngine, Meters};
use crate::audio::mixer::{MixerBus, MixerControl};
use crate::input::HotkeyService;
use crate::presets::voice_config::{resolve_openwire_root, PresetStore};
use crate::soundboard::SoundManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "yes")]
    pub ducking_enabled: bool,
    #[serde(default = "yes")]
    pub monitor_enabled: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ducking_enabled: true,
            monitor_enabled: true,
        }
    }
}

impl Settings {
    fn path() -> Result<std::path::PathBuf> {
        Ok(resolve_openwire_root()?.join("settings.toml"))
    }

    pub fn load() -> Self {
        let Ok(path) = Self::path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| toml::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self).context("settings serialization")?;
        std::fs::write(&path, raw).with_context(|| format!("cannot write {}", path.display()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DspDto {
    pub pitch_enabled: bool,
    pub pitch_semitones: f32,
    pub formant_shift: f32,
    pub eq_low_db: f32,
    pub eq_mid_freq: f32,
    pub eq_mid_gain_db: f32,
    pub eq_mid_q: f32,
    pub eq_high_db: f32,
    pub radio_enabled: bool,
    pub radio_bp_low_hz: f32,
    pub radio_bp_high_hz: f32,
    pub radio_drive: f32,
    pub radio_noise_db: f32,
    pub ducking_enabled: bool,
    pub duck_atten_db: f32,
    pub duck_release_ms: f32,
    pub monitor_volume: f32,
    pub stream_volume: f32,
    pub noise_gate_enabled: bool,
    pub noise_gate_db: f32,
}

impl From<DspSnapshot> for DspDto {
    fn from(s: DspSnapshot) -> Self {
        Self {
            pitch_enabled: s.pitch_enabled,
            pitch_semitones: s.pitch_semitones,
            formant_shift: s.formant_shift,
            eq_low_db: s.eq_low_db,
            eq_mid_freq: s.eq_mid_freq,
            eq_mid_gain_db: s.eq_mid_gain_db,
            eq_mid_q: s.eq_mid_q,
            eq_high_db: s.eq_high_db,
            radio_enabled: s.radio_enabled,
            radio_bp_low_hz: s.radio_bp_low_hz,
            radio_bp_high_hz: s.radio_bp_high_hz,
            radio_drive: s.radio_drive,
            radio_noise_db: s.radio_noise_db,
            ducking_enabled: s.ducking_enabled,
            duck_atten_db: s.duck_atten_db,
            duck_release_ms: s.duck_release_ms,
            monitor_volume: s.monitor_volume,
            stream_volume: s.stream_volume,
            noise_gate_enabled: s.noise_gate_enabled,
            noise_gate_db: s.noise_gate_db,
        }
    }
}

impl DspDto {
    pub fn apply(&self, p: &DspSharedParams) {
        let put = |slot: &AtomicU32, v: f32| slot.store(v.to_bits(), Ordering::Relaxed);
        p.pitch_enabled.store(self.pitch_enabled, Ordering::Relaxed);
        put(&p.pitch_semitones, self.pitch_semitones);
        put(&p.formant_shift, self.formant_shift);
        put(&p.eq_low_db, self.eq_low_db);
        put(&p.eq_mid_freq, self.eq_mid_freq);
        put(&p.eq_mid_gain_db, self.eq_mid_gain_db);
        put(&p.eq_mid_q, self.eq_mid_q);
        put(&p.eq_high_db, self.eq_high_db);
        p.radio_enabled.store(self.radio_enabled, Ordering::Relaxed);
        put(&p.radio_bp_low_hz, self.radio_bp_low_hz);
        put(&p.radio_bp_high_hz, self.radio_bp_high_hz);
        put(&p.radio_drive, self.radio_drive);
        put(&p.radio_noise_db, self.radio_noise_db);
        p.ducking_enabled
            .store(self.ducking_enabled, Ordering::Relaxed);
        put(&p.duck_atten_db, self.duck_atten_db);
        put(&p.duck_release_ms, self.duck_release_ms);
        put(&p.monitor_volume, self.monitor_volume);
        put(&p.stream_volume, self.stream_volume);
        p.noise_gate_enabled
            .store(self.noise_gate_enabled, Ordering::Relaxed);
        put(&p.noise_gate_db, self.noise_gate_db);
    }
}

pub struct AppState {
    pub dsp: Arc<DspSharedParams>,
    pub mixer: MixerControl,
    pub meters: Arc<Meters>,
    pub sound: Arc<SoundManager>,
    pub presets: PresetStore,
    #[allow(dead_code)]
    pub hotkeys: HotkeyService,
    pub settings: RwLock<Settings>,

    pub active_preset: RwLock<Option<String>>,
    pub engine: Mutex<Option<AudioEngine>>,
}

impl AppState {
    pub fn bootstrap() -> Result<Self> {
        let dsp = DspSharedParams::new_defaults();
        let (mixer_control, mixer_bus) = MixerBus::split();

        let sound = Arc::new(SoundManager::new(mixer_control.clone())?);
        let presets = PresetStore::new()?;
        let settings = Settings::load();
        dsp.monitor_enabled
            .store(settings.monitor_enabled, Ordering::Relaxed);

        let engine = match engine::start(dsp.clone(), mixer_bus) {
            Ok(e) => Some(e),
            Err(err) => {
                eprintln!("[openwire] PipeWire engine failed to start: {err:#}");
                None
            }
        };
        let meters = engine
            .as_ref()
            .map(|e| e.meters.clone())
            .unwrap_or_default();

        let hotkeys = HotkeyService::new();
        hotkeys.sync_from_board(&sound.snapshot());
        hotkeys.spawn_listener(sound.clone(), mixer_control.clone());

        Ok(Self {
            dsp,
            mixer: mixer_control,
            meters,
            sound,
            presets,
            hotkeys,
            settings: RwLock::new(settings),
            active_preset: RwLock::new(None),
            engine: Mutex::new(engine),
        })
    }

    pub fn dsp_dto(&self) -> DspDto {
        self.dsp.snapshot().into()
    }
}
