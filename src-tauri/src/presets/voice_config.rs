use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::audio::dsp::{DspSharedParams, DspSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pitch {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub semitones: f32,
    #[serde(default = "one")]
    pub formant_shift: f32,
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Equalizer {
    #[serde(default)]
    pub low_shelf_gain_db: f32,
    #[serde(default = "default_mid_freq")]
    pub mid_freq_hz: f32,
    #[serde(default)]
    pub mid_gain_db: f32,
    #[serde(default = "default_mid_q")]
    pub mid_q: f32,
    #[serde(default)]
    pub high_shelf_gain_db: f32,
}

fn default_mid_freq() -> f32 {
    1200.0
}
fn default_mid_q() -> f32 {
    1.2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioFxSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bp_low")]
    pub bandpass_low_hz: f32,
    #[serde(default = "default_bp_high")]
    pub bandpass_high_hz: f32,
    #[serde(default)]
    pub drive_saturation: f32,
    #[serde(default = "default_noise")]
    pub noise_floor_db: f32,
}

fn default_bp_low() -> f32 {
    380.0
}
fn default_bp_high() -> f32 {
    3600.0
}
fn default_noise() -> f32 {
    -96.0
}

impl Default for RadioFxSection {
    fn default() -> Self {
        Self {
            enabled: false,
            bandpass_low_hz: default_bp_low(),
            bandpass_high_hz: default_bp_high(),
            drive_saturation: 0.0,
            noise_floor_db: default_noise(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DspSection {
    #[serde(default)]
    pub pitch: Option<Pitch>,
    #[serde(default)]
    pub equalizer: Option<Equalizer>,
    #[serde(default)]
    pub radio_fx: Option<RadioFxSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routing {
    #[serde(default = "default_duck")]
    pub ducking_attenuation_db: f32,
    #[serde(default = "default_release")]
    pub sidechain_release_ms: f32,
}

fn default_duck() -> f32 {
    -12.0
}
fn default_release() -> f32 {
    120.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoicePreset {
    pub meta: Meta,
    #[serde(default)]
    pub dsp: DspSection,
    #[serde(default)]
    pub routing: Option<Routing>,
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            name: "Untitled Voice".into(),
            description: String::new(),
            author: "OpenWire".into(),
            version: 1,
        }
    }
}

impl VoicePreset {
    pub fn apply(&self, params: &DspSharedParams) {
        use std::sync::atomic::Ordering;
        let put = |slot: &std::sync::atomic::AtomicU32, v: f32| {
            slot.store(v.to_bits(), Ordering::Relaxed);
        };

        if let Some(p) = &self.dsp.pitch {
            params.pitch_enabled.store(p.enabled, Ordering::Relaxed);
            put(&params.pitch_semitones, p.semitones);
            put(&params.formant_shift, p.formant_shift.max(0.01));
        }
        if let Some(eq) = &self.dsp.equalizer {
            put(&params.eq_low_db, eq.low_shelf_gain_db);
            put(&params.eq_mid_freq, eq.mid_freq_hz);
            put(&params.eq_mid_gain_db, eq.mid_gain_db);
            put(&params.eq_mid_q, eq.mid_q);
            put(&params.eq_high_db, eq.high_shelf_gain_db);
        }
        if let Some(r) = &self.dsp.radio_fx {
            params.radio_enabled.store(r.enabled, Ordering::Relaxed);
            put(&params.radio_bp_low_hz, r.bandpass_low_hz);
            put(&params.radio_bp_high_hz, r.bandpass_high_hz);
            put(&params.radio_drive, r.drive_saturation);
            put(&params.radio_noise_db, r.noise_floor_db);
        }
        if let Some(rt) = &self.routing {
            put(&params.duck_atten_db, rt.ducking_attenuation_db);
            put(&params.duck_release_ms, rt.sidechain_release_ms);
        }
    }

    #[allow(dead_code)]
    pub fn from_dto(meta: Meta, snap: &DspSnapshot, name: &str) -> Self {
        Self {
            meta: Meta {
                name: name.to_string(),
                ..meta
            },
            dsp: DspSection {
                pitch: Some(Pitch {
                    enabled: snap.pitch_enabled,
                    semitones: snap.pitch_semitones,
                    formant_shift: snap.formant_shift,
                }),
                equalizer: Some(Equalizer {
                    low_shelf_gain_db: snap.eq_low_db,
                    mid_freq_hz: snap.eq_mid_freq,
                    mid_gain_db: snap.eq_mid_gain_db,
                    mid_q: snap.eq_mid_q,
                    high_shelf_gain_db: snap.eq_high_db,
                }),
                radio_fx: Some(RadioFxSection {
                    enabled: snap.radio_enabled,
                    bandpass_low_hz: snap.radio_bp_low_hz,
                    bandpass_high_hz: snap.radio_bp_high_hz,
                    drive_saturation: snap.radio_drive,
                    noise_floor_db: snap.radio_noise_db,
                }),
            },
            routing: Some(Routing {
                ducking_attenuation_db: snap.duck_atten_db,
                sidechain_release_ms: snap.duck_release_ms,
            }),
        }
    }
}

pub struct PresetStore {
    dir: PathBuf,
}

impl Default for PresetStore {
    fn default() -> Self {
        Self::new().expect("failed to resolve OpenWire documents directory")
    }
}

impl PresetStore {
    pub fn new() -> Result<Self> {
        let dir = resolve_openwire_root()?.join("Voices");
        fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create preset directory {}", dir.display()))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list_presets(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir)
            .with_context(|| format!("cannot read {}", self.dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(meta) = fs::read_to_string(&path) {
                    let short = toml::from_str::<PresetHead>(&meta).ok();
                    names.push(short.map(|h| h.meta.name).unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    }));
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn load_preset(&self, name: &str) -> Result<VoicePreset> {
        let path = self.resolve_name(name)?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("cannot read preset {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("invalid preset format in {}", path.display()))
    }

    pub fn save_preset(&self, name: &str, toml_str: &str) -> Result<PathBuf> {
        let parsed: VoicePreset =
            toml::from_str(toml_str).map_err(|e| anyhow!("preset TOML is invalid: {e}"))?;
        let pretty = toml::to_string_pretty(&parsed)?;
        let path = self.resolve_name(name)?;
        fs::write(&path, pretty)
            .with_context(|| format!("cannot write preset {}", path.display()))?;
        Ok(path)
    }

    pub fn delete_preset(&self, name: &str) -> Result<()> {
        let path = self.resolve_name(name)?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("cannot delete preset {}", path.display()))?;
        }
        Ok(())
    }

    fn resolve_name(&self, name: &str) -> Result<PathBuf> {
        let clean: String = name
            .chars()
            .map(|c| match c {
                '/' | '\\' | '\0' => '_',
                _ => c,
            })
            .collect();
        if clean.trim().is_empty() || clean == "." || clean == ".." {
            return Err(anyhow!("preset name must not be empty or a traversal"));
        }
        let clean = clean.strip_suffix(".toml").unwrap_or(&clean).to_string();
        Ok(self.dir.join(format!("{clean}.toml")))
    }
}

#[derive(Deserialize)]
struct PresetHead {
    meta: MetaHead,
}

#[derive(Deserialize)]
struct MetaHead {
    name: String,
}

pub fn resolve_openwire_root() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home directory"))?;

    let candidates = [
        dirs::document_dir(),
        Some(home.join("Документы")),
        Some(home.join("Documents")),
        Some(home.clone()),
    ];
    for cand in candidates.into_iter().flatten() {
        if cand.is_dir() {
            return Ok(cand.join("OpenWire"));
        }
    }
    Ok(home.join("OpenWire"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[meta]
name = "Cyberpunk Radio"
description = "Голос диспетчера с перегрузом и полосовым срезом"
author = "OpenWire"
version = 1

[dsp.pitch]
enabled = true
semitones = -3.5
formant_shift = 0.85

[dsp.equalizer]
low_shelf_gain_db = -6.0
mid_freq_hz = 1200.0
mid_gain_db = 4.5
mid_q = 1.8
high_shelf_gain_db = -8.0

[dsp.radio_fx]
enabled = true
bandpass_low_hz = 380.0
bandpass_high_hz = 3600.0
drive_saturation = 0.35
noise_floor_db = -48.0

[routing]
ducking_attenuation_db = -14.0
sidechain_release_ms = 120.0
"#;

    #[test]
    fn sample_preset_roundtrips() {
        let preset: VoicePreset = toml::from_str(SAMPLE).expect("spec preset must parse");
        assert_eq!(preset.meta.name, "Cyberpunk Radio");
        let pitch = preset.dsp.pitch.as_ref().expect("pitch section");
        assert!(pitch.enabled);
        assert!((pitch.semitones + 3.5).abs() < f32::EPSILON);
        let radio = preset.dsp.radio_fx.as_ref().expect("radio section");
        assert!((radio.bandpass_high_hz - 3600.0).abs() < f32::EPSILON);
        let routing = preset.routing.as_ref().expect("routing section");
        assert!((routing.ducking_attenuation_db + 14.0).abs() < f32::EPSILON);

        let back = toml::to_string_pretty(&preset).expect("serialize");
        toml::from_str::<VoicePreset>(&back).expect("roundtrip");
    }

    #[test]
    fn name_resolution_is_sanitised() {
        let store = PresetStore {
            dir: std::env::temp_dir().join("openwire_test_presets"),
        };
        let path = store.resolve_name("../evil/path").expect("sanitised");
        assert_eq!(path.file_name().unwrap(), ".._evil_path.toml");
        assert!(path.starts_with(&store.dir));
        assert!(store.resolve_name("").is_err());
        assert!(store.resolve_name("..").is_err());
    }
}
