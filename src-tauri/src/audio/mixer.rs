use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const COMMAND_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct PlayRequest {
    pub pad_id: usize,
    pub samples: Arc<Vec<f32>>,
    pub monitor_gain: f32,
    pub stream_gain: f32,

    pub looping: bool,
}

pub enum MixerCommand {
    Play(PlayRequest),
    StopPad(usize),
    StopAll,
}

struct ActiveVoice {
    pad_id: usize,
    samples: Arc<Vec<f32>>,
    pos: usize,
    monitor_gain: f32,
    stream_gain: f32,
    looping: bool,
}

#[derive(Clone)]
pub struct MixerControl {
    tx: Arc<Mutex<HeapProd<MixerCommand>>>,
    active: Arc<AtomicUsize>,
}

impl MixerControl {
    pub fn play(&self, req: PlayRequest) {
        if let Ok(mut tx) = self.tx.lock() {
            let _ = tx.try_push(MixerCommand::Play(req));
        }
    }

    pub fn stop_pad(&self, pad_id: usize) {
        if let Ok(mut tx) = self.tx.lock() {
            let _ = tx.try_push(MixerCommand::StopPad(pad_id));
        }
    }

    pub fn stop_all(&self) {
        if let Ok(mut tx) = self.tx.lock() {
            let _ = tx.try_push(MixerCommand::StopAll);
        }
    }

    pub fn active_voices(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }
}

pub struct MixerBus {
    rx: HeapCons<MixerCommand>,
    voices: Vec<ActiveVoice>,
    active_counter: Arc<AtomicUsize>,
}

impl MixerBus {
    pub fn split() -> (MixerControl, MixerBus) {
        let (cmd_tx, cmd_rx) = HeapRb::<MixerCommand>::new(COMMAND_CAPACITY).split();
        let active = Arc::new(AtomicUsize::new(0));
        let control = MixerControl {
            tx: Arc::new(Mutex::new(cmd_tx)),
            active: active.clone(),
        };
        let bus = MixerBus {
            rx: cmd_rx,
            voices: Vec::with_capacity(32),
            active_counter: active,
        };
        (control, bus)
    }

    fn poll_commands(&mut self) {
        while let Some(cmd) = self.rx.try_pop() {
            match cmd {
                MixerCommand::Play(req) => {
                    self.voices.retain(|v| v.pad_id != req.pad_id);
                    self.voices.push(ActiveVoice {
                        pad_id: req.pad_id,
                        samples: req.samples,
                        pos: 0,
                        monitor_gain: req.monitor_gain,
                        stream_gain: req.stream_gain,
                        looping: req.looping,
                    });
                }
                MixerCommand::StopPad(id) => self.voices.retain(|v| v.pad_id != id),
                MixerCommand::StopAll => self.voices.clear(),
            }
        }
        self.active_counter
            .store(self.voices.len(), Ordering::Relaxed);
    }

    pub fn render(&mut self, frames: usize, monitor: &mut [f32], stream: &mut [f32]) -> bool {
        self.poll_commands();
        monitor[..frames].fill(0.0);
        stream[..frames].fill(0.0);

        let mut audible = false;

        for voice in self.voices.iter_mut() {
            if voice.samples.is_empty() {
                continue;
            }
            let mut i = 0;
            while i < frames {
                let remaining = voice.samples.len() - voice.pos;
                let n = remaining.min(frames - i);
                if n == 0 {
                    if voice.looping {
                        voice.pos = 0;
                        continue;
                    }
                    break;
                }
                audible = true;
                for j in 0..n {
                    let s = voice.samples[voice.pos + j];
                    monitor[i + j] += s * voice.monitor_gain;
                    stream[i + j] += s * voice.stream_gain;
                }
                voice.pos += n;
                i += n;
            }
        }
        self.voices.retain(|v| v.looping || v.pos < v.samples.len());
        self.active_counter
            .store(self.voices.len(), Ordering::Relaxed);

        audible
    }
}
