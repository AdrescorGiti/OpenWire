use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use crate::audio::dsp::SAMPLE_RATE;
use crate::audio::mixer::{MixerControl, PlayRequest};
use crate::audio::player::{self, SampleData};
use crate::presets::voice_config::resolve_openwire_root;

use super::bank::{Pad, Soundboard};

pub fn pad_id(bank: usize, slot: usize) -> usize {
    bank.saturating_mul(1_000_000).saturating_add(slot)
}

fn bass_boost(samples: &[f32]) -> Vec<f32> {
    let mut shelf = crate::audio::dsp::Biquad::low_shelf(SAMPLE_RATE as f32, 200.0, 6.0);
    samples.iter().map(|&s| shelf.process(s)).collect()
}

pub struct SoundManager {
    board: RwLock<Soundboard>,
    cache: Mutex<HashMap<PathBuf, SampleData>>,
    control: MixerControl,
    board_path: PathBuf,
}

impl SoundManager {
    pub fn new(control: MixerControl) -> Result<Self> {
        let root = resolve_openwire_root()?;
        let board_path = root.join("soundboard.toml");
        let board = Soundboard::load_or_default(&board_path);
        Ok(Self {
            board: RwLock::new(board),
            cache: Mutex::new(HashMap::new()),
            control,
            board_path,
        })
    }

    pub fn snapshot(&self) -> Soundboard {
        self.board.read().expect("soundboard lock").clone()
    }

    pub fn update_pad(&self, bank: usize, pad: Pad) -> Result<()> {
        {
            let mut board = self.board.write().expect("soundboard lock");
            let b = board
                .banks
                .get_mut(bank)
                .ok_or_else(|| anyhow!("bank {bank}"))?;
            let slot = b
                .pads
                .get_mut(pad.id)
                .ok_or_else(|| anyhow!("pad {}", pad.id))?;
            *slot = pad;
        }
        self.persist()
    }

    pub fn add_pad(&self, bank: usize) -> Result<Pad> {
        let pad = {
            let mut board = self.board.write().expect("soundboard lock");
            let b = board
                .banks
                .get_mut(bank)
                .ok_or_else(|| anyhow!("bank {bank}"))?;
            let pad = Pad {
                id: b.pads.len(),
                bank,
                ..Pad::default()
            };
            b.pads.push(pad.clone());
            pad
        };
        self.persist()?;
        Ok(pad)
    }

    pub fn clear_pad(&self, bank: usize, slot: usize) -> Result<()> {
        {
            let mut board = self.board.write().expect("soundboard lock");
            let b = board
                .banks
                .get_mut(bank)
                .ok_or_else(|| anyhow!("bank {bank}"))?;
            let pad = b.pads.get_mut(slot).ok_or_else(|| anyhow!("pad {slot}"))?;
            pad.path = None;
            pad.name.clear();
            pad.duration_ms = 0;
            pad.trim_start_ms = 0;
            pad.trim_end_ms = 0;
        }
        self.persist()
    }

    pub fn waveform(&self, bank: usize, slot: usize, points: usize) -> Result<Vec<f32>> {
        let pad = {
            let board = self.board.read().expect("soundboard lock");
            board
                .banks
                .get(bank)
                .and_then(|b| b.pads.get(slot))
                .cloned()
                .ok_or_else(|| anyhow!("pad {bank}/{slot} does not exist"))?
        };
        let data = self.pad_sample(&pad)?;
        let points = points.max(1);
        if data.samples.is_empty() {
            return Ok(vec![0.0; points]);
        }
        let bucket = (data.samples.len() as f64 / points as f64).max(1.0);
        let mut out = Vec::with_capacity(points);
        for i in 0..points {
            let start = (i as f64 * bucket) as usize;
            let end = (((i + 1) as f64 * bucket) as usize).min(data.samples.len());
            let peak = data.samples[start..end.max(start + 1).min(data.samples.len())]
                .iter()
                .fold(0.0f32, |acc, s| acc.max(s.abs()));
            out.push(peak.min(1.0));
        }
        Ok(out)
    }

    pub fn import(&self, bank: usize, slot: usize, path: PathBuf) -> Result<Pad> {
        let data = self.decode(&path)?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sound".into());
        let mut board = self.board.write().expect("soundboard lock");
        let b = board
            .banks
            .get_mut(bank)
            .ok_or_else(|| anyhow!("bank {bank}"))?;
        let pad = b.pads.get_mut(slot).ok_or_else(|| anyhow!("pad {slot}"))?;
        pad.name = name;
        pad.path = Some(path);
        pad.duration_ms = data.duration_ms;
        let result = pad.clone();
        drop(board);
        self.persist()?;
        Ok(result)
    }

    pub fn play(&self, bank: usize, slot: usize) -> Result<()> {
        let pad = {
            let board = self.board.read().expect("soundboard lock");
            board
                .banks
                .get(bank)
                .and_then(|b| b.pads.get(slot))
                .cloned()
                .ok_or_else(|| anyhow!("pad {bank}/{slot} does not exist"))?
        };

        if pad.path.is_none() && pad.duration_ms == 0 {
            return Ok(());
        }
        self.play_pad(&pad)
    }

    pub fn play_pad(&self, pad: &Pad) -> Result<()> {
        let data = self.pad_sample(pad)?;

        let start =
            (pad.trim_start_ms as usize * SAMPLE_RATE as usize / 1000).min(data.samples.len());
        let end = if pad.trim_end_ms > 0 {
            let e = pad.trim_end_ms as usize * SAMPLE_RATE as usize / 1000;
            e.clamp(start, data.samples.len())
        } else {
            data.samples.len()
        };
        let targeted = Arc::new(data.samples[start..end].to_vec());

        let slice: Arc<Vec<f32>> = if pad.bass_boost {
            Arc::new(bass_boost(&targeted))
        } else if start == 0 && end == data.samples.len() {
            data.samples.clone()
        } else {
            targeted
        };

        self.control.play(PlayRequest {
            pad_id: pad_id(pad.bank, pad.id),
            samples: slice,
            monitor_gain: pad.monitor_volume.clamp(0.0, 1.0),
            stream_gain: pad.stream_volume.clamp(0.0, 2.0),
            looping: matches!(pad.play_mode.as_str(), "loop" | "toggle"),
        });
        Ok(())
    }

    pub fn stop_pad(&self, bank: usize, slot: usize) {
        self.control.stop_pad(pad_id(bank, slot));
    }

    pub fn stop_all(&self) {
        self.control.stop_all();
    }

    fn pad_sample(&self, pad: &Pad) -> Result<SampleData> {
        let path = pad
            .path
            .clone()
            .ok_or_else(|| anyhow!("pad '{}' has no audio attached", pad.name))?;
        self.decode(&path)
    }

    fn decode(&self, path: &PathBuf) -> Result<SampleData> {
        if let Some(hit) = self.cache.lock().expect("cache lock").get(path) {
            return Ok(hit.clone());
        }
        let data = player::decode_file(path)?;
        self.cache
            .lock()
            .expect("cache lock")
            .insert(path.clone(), data.clone());
        Ok(data)
    }

    fn persist(&self) -> Result<()> {
        self.board
            .read()
            .expect("soundboard lock")
            .save(&self.board_path)
    }
}
