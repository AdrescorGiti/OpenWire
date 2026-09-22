import { state } from './state.js';
import { metersPoll } from './ipc.js';

const clipHold = { mic: 0, out: 0 };

function paintMeter(barId, clipId, value, key) {
  const bar = document.getElementById(barId);
  const clip = document.getElementById(clipId);
  if (!bar || !clip) return;
  const pct = Math.min(100, Math.max(0, value * 100));
  bar.style.width = `${pct}%`;
  bar.style.background = value > 0.98 ? '#E5534B' : (key === 'mic' ? '#F4772E' : '#3FD46C');
  if (value > 0.98) clipHold[key] = Date.now() + 900;
  clip.classList.toggle('lit', Date.now() < clipHold[key]);
}

export async function pollMeters() {
  try {
    const m = await metersPoll();
    state.telemetry = m;
    paintMeter('micVU', 'micVUClip', m.mic, 'mic');
    paintMeter('outVU', 'outVUClip', m.out, 'out');
    const set = (id, v) => {
      const el = document.getElementById(id);
      if (el) el.textContent = v;
    };
    set('sbPw', `PipeWire ${m.pipewire_version || '…'}`);
    set('sbQuantum', `Quantum: ${m.quantum || '—'}`);
    set('sbLatency', `Latency: ${m.quantum ? m.latency_ms.toFixed(1) + 'ms' : '—'}`);
    set('sbVoices', String(m.active_voices ?? 0));
    set('sbPreset', m.active_preset || '—');
    set('sbEngine', m.engine_running ? 'Audio: Ready' : 'PipeWire offline');
    const chip = document.getElementById('presetName');
    if (chip) chip.textContent = m.active_preset || '—';
    const led = document.getElementById('micLed');
    if (led) {
      led.className = `w-1.5 h-1.5 rounded-full ${
        m.engine_running ? (m.mic > 0.02 ? 'bg-ok' : 'bg-warn') : 'bg-err'
      }`;
    }
  } catch (_) {
  }
}

export function startMeterLoop(intervalMs = 120) {
  setInterval(pollMeters, intervalMs);
}
