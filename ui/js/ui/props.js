import { state, esc, selectedPad } from '../state.js';
import { padWaveform } from '../ipc.js';

export function propsPanelHtml() {
  const p = selectedPad() ?? {};
  const hasAudio = !!(p.path || p.duration_ms);
  const duration = p.duration_ms || 0;
  const startX = duration ? 14 + ((p.trim_start_ms || 0) / duration) * 232 : 14;
  const endX = duration && p.trim_end_ms ? 14 + (p.trim_end_ms / duration) * 232 : 246;
  return `
  <div class="w-[288px] bg-bar border-l border-line p-3 space-y-3 overflow-auto">
    <div class="bg-surf border border-line rounded overflow-hidden">
      <div class="px-2 py-1.5 border-b border-line flex items-center justify-between">
        <span class="font-mono text-[9px] text-dim">TRIM / WAVEFORM</span>
        <span class="font-mono text-[9px] text-ghost" id="trimLabel">${hasAudio ? `${p.trim_start_ms | 0}ms → ${(p.trim_end_ms || duration) | 0}ms` : '—'}</span>
      </div>
      <svg id="waveSvg" viewBox="0 0 260 56" class="w-full h-[56px] bg-inset" style="touch-action:none"
           data-start="${startX}" data-end="${endX}" data-duration="${duration}">
        <polygon id="waveFill" points="" fill="rgba(63,191,174,0.35)"/>
        <rect id="trimShadeA" x="0" y="0" width="${startX}" height="56" fill="rgba(0,0,0,0.45)"/>
        <rect id="trimShadeB" x="${endX}" y="0" width="${260 - endX}" height="56" fill="rgba(0,0,0,0.45)"/>
        <g id="trimStart" class="trim-handle"><rect x="${startX - 3}" y="0" width="7" height="56" fill="transparent"/><line x1="${startX}" y1="0" x2="${startX}" y2="56" stroke="#F4772E" stroke-width="2"/></g>
        <g id="trimEnd" class="trim-handle"><rect x="${endX - 3}" y="0" width="7" height="56" fill="transparent"/><line x1="${endX}" y1="0" x2="${endX}" y2="56" stroke="#F4772E" stroke-width="2"/></g>
        <text x="6" y="12" fill="#8A959E" font-size="7" font-family="monospace">TRIM</text>
      </svg>
    </div>

    <div class="bg-surf border border-line rounded p-3 space-y-3">
      <div class="flex items-center justify-between gap-2">
        <span class="font-mono text-[10px] text-dim truncate">PAD · ${esc(p.name || 'пусто')}</span>
        ${hasAudio ? `<button data-action="clear-pad" class="font-mono text-[9px] text-err hover:underline shrink-0">очистить</button>` : ''}
      </div>
      <div class="space-y-1">
        <div class="flex justify-between"><span class="text-[11px] text-mut">В наушники (Себе)</span>
          <span class="font-mono text-[10px] text-accent" id="padMonitorVal">${Math.round((p.monitor_volume ?? 1) * 100)}%</span></div>
        <input data-pad-slider="monitor_volume" type="range" min="0" max="100" step="1" value="${Math.round((p.monitor_volume ?? 1) * 100)}" class="w-full text-accent" />
      </div>
      <div class="space-y-1">
        <div class="flex justify-between"><span class="text-[11px] text-mut">В эфир (Стрим)</span>
          <span class="font-mono text-[10px] text-ok" id="padStreamVal">${Math.round((p.stream_volume ?? 1) * 100)}%</span></div>
        <input data-pad-slider="stream_volume" type="range" min="0" max="200" step="1" value="${Math.round((p.stream_volume ?? 1) * 100)}" class="w-full text-ok" />
      </div>
      <label class="flex items-center justify-between text-[11px] text-mut">
        BassBoost <input data-pad-flag="bass_boost" type="checkbox" class="accent-accent" ${p.bass_boost ? 'checked' : ''} />
      </label>
    </div>

    <div class="bg-surf border border-line rounded p-3 space-y-2">
      <div class="font-mono text-[9px] text-dim">HOTKEY</div>
      <button data-action="capture-hotkey" id="hotkeyCapture" class="w-full bg-inset border border-line hover:border-accent rounded px-2 py-1.5 font-mono text-[11px] text-accent">
        ${esc(p.hotkey || 'Нажмите клавишу…')}
      </button>
      <div class="font-mono text-[9px] text-dim pt-1">PLAY MODE</div>
      <select data-pad-mode id="playMode" class="w-full bg-inset border border-line rounded px-2 py-1.5 text-[11px] text-body font-mono">
        <option value="oneshot" ${(p.play_mode ?? 'oneshot') === 'oneshot' ? 'selected' : ''}>one-shot</option>
        <option value="loop" ${p.play_mode === 'loop' ? 'selected' : ''}>loop</option>
        <option value="toggle" ${p.play_mode === 'toggle' ? 'selected' : ''}>toggle</option>
      </select>
      <button data-action="test-pad" class="w-full mt-1 px-2 py-1.5 rounded bg-surf2 hover:bg-hover text-[11px] text-body">▶ Тест пада</button>
    </div>
  </div>`;
}

export async function refreshWaveform() {
  const svg = document.getElementById('waveSvg');
  if (!svg) return;
  const p = selectedPad();
  const key = p ? `${state.selected.bank}:${p.id}:${p.duration_ms}:${p.path ?? 'memory'}` : null;
  if (state.waveformKey === key) { drawWaveform(svg); return; }
  state.waveformKey = key;
  state.waveform = null;
  if (p && (p.path || p.duration_ms)) {
    try {
      state.waveform = await padWaveform(state.selected.bank, p.id, 160);
    } catch (_) {
      state.waveform = null;
    }
  }
  if (state.waveformKey === key) drawWaveform(document.getElementById('waveSvg'));
}

function drawWaveform(svg) {
  if (!svg) return;
  const fill = svg.querySelector('#waveFill');
  if (!fill) return;
  const pts = state.waveform?.length ? state.waveform : dummyWave();
  const n = pts.length;
  const xAt = (i) => (14 + (i / (n - 1)) * 232).toFixed(1);
  const top = pts.map((amp, i) => `${xAt(i)},${(28 - amp * 24).toFixed(1)}`);
  const bottom = pts.map((amp, i) => `${xAt(i)},${(28 + amp * 24).toFixed(1)}`).reverse();
  fill.setAttribute('points', top.concat(bottom).join(' '));
}

function dummyWave() {
  const pts = [];
  for (let i = 0; i < 120; i++) {
    pts.push(Math.exp(-Math.pow((i - 60) / 42, 2)) * 0.6);
  }
  return pts;
}
