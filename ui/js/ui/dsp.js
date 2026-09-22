import { state, esc, activePresetName } from '../state.js';

export function dspView() {
  const d = state.dsp ?? {};
  const row = (id, label, min, max, step, value, unit, color = 'text-accent') => `
    <div class="flex items-center gap-3">
      <span class="text-[11px] text-mut w-[150px]">${label}</span>
      <input data-dsp="${id}" type="range" min="${min}" max="${max}" step="${step}" value="${value}" class="${color} flex-1" />
      <span class="font-mono text-[10px] text-dim w-[76px] text-right" id="${id}Val">${(+value).toFixed(2)}${unit}</span>
    </div>`;
  return `
  <div class="flex-1 min-h-0 overflow-auto p-4 space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-hi text-[13px] font-bold">DSP ГОЛОС & ТОКЕНЫ</h2>
      <div class="flex items-center gap-2">
        <span class="font-mono text-[10px] text-dim">текущий:</span>
        <span class="font-mono text-[10px] text-accent">${esc(activePresetName())}</span>
        <button data-action="save-current-preset" class="px-2.5 py-1 rounded border border-line hover:border-hover text-[10px] text-mut">Сохранить как пресет</button>
      </div>
    </div>
    <div class="bg-surf border border-line rounded p-4 space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-hi text-[12px] font-semibold">PITCH & FORMANT</h3>
        <label class="flex items-center gap-2 text-[11px] text-mut">
          <input data-dsp-bool="pitch_enabled" type="checkbox" class="accent-accent" ${d.pitch_enabled ? 'checked' : ''} /> enabled
        </label>
      </div>
      ${row('pitch_semitones', 'Pitch (полутоны)', -12, 12, 0.1, d.pitch_semitones ?? 0, ' st')}
      ${row('formant_shift', 'Formant shift', 0.5, 2, 0.01, d.formant_shift ?? 1, ' ×')}
    </div>
    <div class="bg-surf border border-line rounded p-4 space-y-3">
      <h3 class="text-hi text-[12px] font-semibold">4-ПОЛОСНЫЙ ЭКВАЛАЙЗЕР</h3>
      ${row('eq_low_db', 'Low shelf', -24, 24, 0.5, d.eq_low_db ?? 0, ' dB', 'text-info')}
      ${row('eq_mid_gain_db', 'Peaking gain', -24, 24, 0.5, d.eq_mid_gain_db ?? 0, ' dB', 'text-info')}
      ${row('eq_mid_freq', 'Peaking freq', 60, 12000, 10, d.eq_mid_freq ?? 1200, ' Hz', 'text-info')}
      ${row('eq_mid_q', 'Peaking Q', 0.2, 8, 0.1, d.eq_mid_q ?? 1.2, '', 'text-info')}
      ${row('eq_high_db', 'High shelf', -24, 24, 0.5, d.eq_high_db ?? 0, ' dB', 'text-info')}
    </div>
    <div class="bg-surf border border-line rounded p-4 space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-hi text-[12px] font-semibold">RADIO FX</h3>
        <label class="flex items-center gap-2 text-[11px] text-mut">
          <input data-dsp-bool="radio_enabled" type="checkbox" class="accent-warn" ${d.radio_enabled ? 'checked' : ''} /> enabled
        </label>
      </div>
      ${row('radio_bp_low_hz', 'Band-pass low', 80, 2000, 5, d.radio_bp_low_hz ?? 380, ' Hz', 'text-warn')}
      ${row('radio_bp_high_hz', 'Band-pass high', 1500, 12000, 50, d.radio_bp_high_hz ?? 3600, ' Hz', 'text-warn')}
      ${row('radio_drive', 'Drive saturation', 0, 1, 0.01, d.radio_drive ?? 0, '', 'text-warn')}
      ${row('radio_noise_db', 'Noise floor', -96, -20, 1, d.radio_noise_db ?? -96, ' dB', 'text-warn')}
    </div>
    <div class="bg-surf border border-line rounded p-4 space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-hi text-[12px] font-semibold">ШУМОПОДАВЛЕНИЕ</h3>
        <label class="flex items-center gap-2 text-[11px] text-mut"><input data-dsp-bool="noise_gate_enabled" type="checkbox" class="accent-ok" ${d.noise_gate_enabled === false ? '' : 'checked'} /> включено</label>
      </div>
      ${row('noise_gate_db', 'Порог шума', -90, -20, 1, d.noise_gate_db ?? -52, ' dB', 'text-ok')}
      <p class="text-[10px] text-ghost">Тише порога — не передаётся. Начните с −52 dB.</p>
    </div>
    <div class="bg-surf border border-line rounded p-4 space-y-3">
      <h3 class="text-hi text-[12px] font-semibold">ROUTING & DUCKING</h3>
      ${row('duck_atten_db', 'Ducking attenuation', -60, 0, 1, d.duck_atten_db ?? -12, ' dB', 'text-ok')}
      ${row('duck_release_ms', 'Sidechain release', 20, 2000, 10, d.duck_release_ms ?? 120, ' ms', 'text-ok')}
      ${row('monitor_volume', 'В наушники (себе)', 0, 1, 0.01, d.monitor_volume ?? 1, ' ×', 'text-accent')}
      ${row('stream_volume', 'В эфир (стрим)', 0, 2, 0.01, d.stream_volume ?? 1, ' ×', 'text-ok')}
    </div>
  </div>`;
}
