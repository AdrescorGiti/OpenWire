use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const PADS_PER_BANK: usize = 15;
pub const DEFAULT_BANKS: [&str; 3] = ["Мемы", "Катка", "Стрим"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pad {
    pub id: usize,

    #[serde(default)]
    pub bank: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub hotkey: Option<String>,

    #[serde(default = "full")]
    pub monitor_volume: f32,

    #[serde(default = "full")]
    pub stream_volume: f32,
    #[serde(default)]
    pub trim_start_ms: u32,
    #[serde(default)]
    pub trim_end_ms: u32,

    #[serde(default)]
    pub duration_ms: u32,

    #[serde(default)]
    pub bass_boost: bool,

    #[serde(default = "oneshot")]
    pub play_mode: String,
}

fn oneshot() -> String {
    "oneshot".into()
}

fn full() -> f32 {
    1.0
}

impl Default for Pad {
    fn default() -> Self {
        Self {
            id: 0,
            bank: 0,
            name: String::new(),
            path: None,
            hotkey: None,
            monitor_volume: 1.0,
            stream_volume: 1.0,
            trim_start_ms: 0,
            trim_end_ms: 0,
            duration_ms: 0,
            bass_boost: false,
            play_mode: "oneshot".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub name: String,
    pub pads: Vec<Pad>,
}

impl Bank {
    pub fn new(name: &str, bank: usize) -> Self {
        Self {
            name: name.to_string(),
            pads: (0..PADS_PER_BANK)
                .map(|id| Pad {
                    id,
                    bank,
                    hotkey: default_hotkey(id),
                    ..Pad::default()
                })
                .collect(),
        }
    }
}

fn default_hotkey(slot: usize) -> Option<String> {
    const KEYS: [&str; PADS_PER_BANK] = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "Q", "W", "E", "R", "T",
    ];
    KEYS.get(slot).map(|s| s.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soundboard {
    pub banks: Vec<Bank>,
}

impl Default for Soundboard {
    fn default() -> Self {
        Self {
            banks: DEFAULT_BANKS
                .iter()
                .enumerate()
                .map(|(i, n)| Bank::new(n, i))
                .collect(),
        }
    }
}

impl Soundboard {
    pub fn load_or_default(path: &PathBuf) -> Self {
        let board = match fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        board.with_fixed_indices()
    }

    fn with_fixed_indices(mut self) -> Self {
        for (bank_idx, bank) in self.banks.iter_mut().enumerate() {
            while bank.pads.len() < PADS_PER_BANK {
                let id = bank.pads.len();
                bank.pads.push(Pad {
                    id,
                    bank: bank_idx,
                    hotkey: default_hotkey(id),
                    ..Pad::default()
                });
            }
            for (slot, pad) in bank.pads.iter_mut().enumerate() {
                pad.bank = bank_idx;
                pad.id = slot;
            }
        }
        self
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self).context("soundboard serialization")?;
        fs::write(path, raw).with_context(|| format!("cannot write soundboard {}", path.display()))
    }
}
