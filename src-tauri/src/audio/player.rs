use super::dsp::SAMPLE_RATE;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer as SymSampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Clone)]
pub struct SampleData {
    pub samples: Arc<Vec<f32>>,
    pub duration_ms: u32,
}

pub fn decode_file(path: &Path) -> Result<SampleData> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("cannot open sound file {}", path.display()))?;
    let ext = path.extension().and_then(|e| e.to_str());
    decode_source(Box::new(file), ext, &path.display().to_string())
}

fn decode_source(
    source: Box<dyn symphonia::core::io::MediaSource>,
    extension: Option<&str>,
    label: &str,
) -> Result<SampleData> {
    let mss = MediaSourceStream::new(source, Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = extension {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("unsupported audio container: {label}"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("no default audio track in {label}"))?;
    let track_id = track.id;
    let src_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("no decoder available for track")?;
    let mut mono = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => append_mono(&buf, &mut mono),
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }
    if mono.is_empty() {
        return Err(anyhow!("no decodable samples in {label}"));
    }
    let samples = if src_rate != SAMPLE_RATE {
        resample_block(&mono, src_rate, SAMPLE_RATE)
    } else {
        mono
    };
    let duration_ms = (samples.len() as u64 * 1000 / SAMPLE_RATE as u64) as u32;
    Ok(SampleData {
        samples: Arc::new(samples),
        duration_ms,
    })
}
fn append_mono(buf: &AudioBufferRef, out: &mut Vec<f32>) {
    let spec = *buf.spec();
    let channels = spec.channels.count().max(1);
    let mut samples = SymSampleBuffer::<f32>::new(buf.capacity() as u64, spec);
    samples.copy_interleaved_ref(buf.clone());
    for frame in samples.samples().chunks(channels) {
        out.push(frame.iter().sum::<f32>() / channels as f32);
    }
}
pub fn resample_block(src: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || src.is_empty() {
        return src.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let mut out = Vec::with_capacity((src.len() as f64 / ratio) as usize);
    for i in 0..out.capacity() {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = src[idx.min(src.len() - 1)];
        let b = src[(idx + 1).min(src.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}
