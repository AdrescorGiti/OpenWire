use anyhow::{Context, Result};
use libspa as spa;
use pipewire as pw;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use super::dsp::{DspChain, DspSharedParams, DuckEnvelope, SAMPLE_RATE};
use super::mixer::MixerBus;

const TARGET_QUANTUM: usize = 64;
const RING_CAPACITY: usize = TARGET_QUANTUM * (SAMPLE_RATE as usize / TARGET_QUANTUM / 4);
const MAX_BLOCK: usize = 4096;
const NODE_LATENCY: &str = "64/48000";

#[derive(Default)]
pub struct Meters {
    pub mic_peak: AtomicU32,

    pub out_peak: AtomicU32,

    pub quantum: AtomicU32,

    pub latency_ms: AtomicU32,
    pub version: Mutex<String>,
}

const PEAK_DECAY: f32 = 0.82;

impl Meters {
    pub fn mic(&self) -> f32 {
        f32::from_bits(self.mic_peak.load(Ordering::Relaxed))
    }
    pub fn out(&self) -> f32 {
        f32::from_bits(self.out_peak.load(Ordering::Relaxed))
    }
    pub fn quantum(&self) -> u32 {
        self.quantum.load(Ordering::Relaxed)
    }
    pub fn latency_ms(&self) -> f32 {
        f32::from_bits(self.latency_ms.load(Ordering::Relaxed))
    }
    pub fn version(&self) -> String {
        self.version
            .lock()
            .map(|v| v.clone())
            .unwrap_or_else(|_| "unknown".into())
    }

    fn publish_peak(slot: &AtomicU32, peak: f32) {
        let prev = f32::from_bits(slot.load(Ordering::Relaxed));
        let next = peak.max(prev * PEAK_DECAY);
        slot.store(next.to_bits(), Ordering::Relaxed);
    }
}

struct RenderCore {
    dsp_rx: ringbuf::HeapCons<f32>,
    mixer: MixerBus,
    monitor_tx: ringbuf::HeapProd<f32>,
    duck: DuckEnvelope,
    params: Arc<DspSharedParams>,
    meters: Arc<Meters>,
    monitor_scratch: Vec<f32>,
    stream_scratch: Vec<f32>,
    out_scratch: Vec<f32>,
}

impl RenderCore {
    fn render_block(&mut self, frames: usize) -> &[f32] {
        let frames = frames.min(MAX_BLOCK);
        let monitor = &mut self.monitor_scratch[..frames];
        let stream = &mut self.stream_scratch[..frames];

        let pad_active = self.mixer.render(frames, monitor, stream);

        let snap = self.params.snapshot();
        let duck = self.duck.next_gain(pad_active, &snap, frames);

        let mut mic_peak = 0.0f32;
        let mut out_peak = 0.0f32;
        for i in 0..frames {
            let mic = self.dsp_rx.try_pop().unwrap_or(0.0);
            mic_peak = mic_peak.max(mic.abs());

            let monitor_mic = if snap.monitor_enabled {
                mic * snap.monitor_volume
            } else {
                0.0
            };
            let vmic = mic * duck * snap.stream_volume + stream[i];
            let monitor = monitor_mic + monitor[i];

            out_peak = out_peak.max(vmic.abs());
            self.out_scratch[i] = vmic.clamp(-1.0, 1.0);

            let _ = self.monitor_tx.try_push(monitor.clamp(-1.0, 1.0));
        }
        Meters::publish_peak(&self.meters.mic_peak, mic_peak);
        Meters::publish_peak(&self.meters.out_peak, out_peak);
        &self.out_scratch[..frames]
    }
}

pub struct AudioEngine {
    pub meters: Arc<Meters>,
    _thread: std::thread::JoinHandle<()>,
}

pub fn start(dsp_params: Arc<DspSharedParams>, mixer: MixerBus) -> Result<AudioEngine> {
    let meters = Arc::new(Meters::default());
    let meters_thread = meters.clone();

    let handle = std::thread::Builder::new()
        .name("openwire-pipewire".into())
        .spawn(move || {
            if let Err(err) = run_graph(dsp_params, mixer, meters_thread) {
                log_err(&format!("PipeWire graph terminated: {err:#}"));
            }
        })
        .context("failed to spawn PipeWire thread")?;

    Ok(AudioEngine {
        meters,
        _thread: handle,
    })
}

fn log_err(msg: &str) {
    eprintln!("[openwire][engine] {msg}");
}

fn format_params() -> Result<Vec<u8>> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(SAMPLE_RATE);
    info.set_channels(1);
    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    let bytes = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .map_err(|e| anyhow::anyhow!("spa pod serialization failed: {e:?}"))?
    .0
    .into_inner();
    Ok(bytes)
}

fn connect_format(stream: &pw::stream::Stream, direction: spa::utils::Direction) -> Result<()> {
    let values = format_params()?;
    let pod = spa::pod::Pod::from_bytes(&values).context("invalid format pod")?;
    let mut params = [pod];
    stream
        .connect(
            direction,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .context("stream.connect failed")?;
    Ok(())
}

fn run_graph(dsp_params: Arc<DspSharedParams>, mixer: MixerBus, meters: Arc<Meters>) -> Result<()> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None).context("MainLoop")?;
    let context = pw::context::ContextRc::new(&mainloop, None).context("Context")?;
    let core = context.connect_rc(None).context("connect to PipeWire")?;

    {
        let meters = meters.clone();
        let _core_listener = core
            .add_listener_local()
            .info(move |info| {
                if let Ok(mut slot) = meters.version.lock() {
                    *slot = info.version().to_string();
                }
            })
            .register();
    }

    let (mut dsp_tx, dsp_rx) = HeapRb::<f32>::new(RING_CAPACITY).split();
    let (monitor_tx, mut monitor_rx) = HeapRb::<f32>::new(RING_CAPACITY).split();

    let capture_props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "DSP",
        *pw::keys::APP_NAME => "OpenWire",
        *pw::keys::NODE_LATENCY => NODE_LATENCY,
    };
    let capture = pw::stream::StreamBox::new(&core, "openwire-capture", capture_props)
        .context("capture stream")?;

    let mut dsp_chain = DspChain::new(dsp_params.clone());
    let mut capture_buf: Vec<f32> = Vec::with_capacity(MAX_BLOCK);

    let _capture_listener = capture
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else {
                return;
            };
            let (offset, size) = (d.chunk().offset() as usize, d.chunk().size() as usize);
            let Some(data) = d.data() else {
                return;
            };
            let end = (offset + size).min(data.len());
            if end <= offset {
                return;
            }

            capture_buf.clear();
            for bytes in data[offset..end].chunks_exact(4) {
                capture_buf.push(f32::from_le_bytes(bytes.try_into().unwrap()));
            }
            if capture_buf.is_empty() {
                return;
            }
            dsp_chain.process_block(&mut capture_buf);
            for &s in &capture_buf {
                let _ = dsp_tx.try_push(s);
            }
        })
        .register()
        .context("capture listener")?;

    let mut render_core = RenderCore {
        dsp_rx,
        mixer,
        monitor_tx,
        duck: DuckEnvelope::new(),
        params: dsp_params.clone(),
        meters: meters.clone(),
        monitor_scratch: vec![0.0; MAX_BLOCK],
        stream_scratch: vec![0.0; MAX_BLOCK],
        out_scratch: vec![0.0; MAX_BLOCK],
    };

    let vmic_props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CLASS => "Audio/Source",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::NODE_NAME => "openwire.virtual-mic",
        *pw::keys::NODE_DESCRIPTION => "OpenWire Virtual Mic",
        *pw::keys::APP_NAME => "OpenWire",
        *pw::keys::NODE_LATENCY => NODE_LATENCY,
        *pw::keys::NODE_ALWAYS_PROCESS => "true",
    };
    let vmic = pw::stream::StreamBox::new(&core, "openwire-virtual-mic", vmic_props)
        .context("virtual mic stream")?;

    let meters_vmic = meters.clone();
    let _vmic_listener = vmic
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else {
                return;
            };
            let Some(data) = d.data() else {
                return;
            };
            let frames = (data.len() / 4).min(MAX_BLOCK);

            meters_vmic.quantum.store(frames as u32, Ordering::Relaxed);
            meters_vmic.latency_ms.store(
                (frames as f32 * 1000.0 / SAMPLE_RATE as f32).to_bits(),
                Ordering::Relaxed,
            );

            let samples = render_core.render_block(frames);

            for (i, sample) in samples.iter().enumerate() {
                data[i * 4..i * 4 + 4].copy_from_slice(&sample.to_le_bytes());
            }
            let chunk = d.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = 4;
            *chunk.size_mut() = (frames * 4) as u32;
        })
        .register()
        .context("vmic listener")?;

    let monitor_props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "DSP",
        *pw::keys::APP_NAME => "OpenWire",
        *pw::keys::NODE_NAME => "openwire.monitor",
        *pw::keys::NODE_DESCRIPTION => "OpenWire Monitor",
        *pw::keys::NODE_LATENCY => NODE_LATENCY,
    };
    let monitor = pw::stream::StreamBox::new(&core, "openwire-monitor", monitor_props)
        .context("monitor stream")?;

    let _monitor_listener = monitor
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else {
                return;
            };
            let Some(data) = d.data() else {
                return;
            };
            let frames = (data.len() / 4).min(MAX_BLOCK);
            for i in 0..frames {
                let s = monitor_rx.try_pop().unwrap_or(0.0);
                data[i * 4..i * 4 + 4].copy_from_slice(&s.to_le_bytes());
            }
            let chunk = d.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = 4;
            *chunk.size_mut() = (frames * 4) as u32;
        })
        .register()
        .context("monitor listener")?;

    connect_format(&capture, spa::utils::Direction::Input)?;
    connect_format(&vmic, spa::utils::Direction::Output)?;
    connect_format(&monitor, spa::utils::Direction::Output)?;

    mainloop.run();
    Ok(())
}
