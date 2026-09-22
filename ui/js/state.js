export const state = {
  view: 'board', bank: 0, board: null, dsp: null, presets: [], presetsDir: '',
  selected: { bank: 0, slot: 0 }, playing: new Map(), latched: new Set(),
  settings: null, search: '', telemetry: {}, waveform: null, waveformKey: null,
};
export const $ = (sel, root = document) => root.querySelector(sel);
export const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];
export const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;' }[c]));
export const padKey = (b, s) => `${b}:${s}`;
export const debounce = (fn, ms) => { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; };
export const pad = (b, s) => state.board?.banks?.[b]?.pads?.[s] ?? null;
export const selectedPad = () => pad(state.selected.bank, state.selected.slot);
export const activePresetName = () => state.telemetry.active_preset || '—';
export function formatMs(ms) { if (!ms) return ''; const s = ms / 1000; return s >= 60 ? `${Math.floor(s/60)}:${String(Math.round(s)%60).padStart(2,'0')}` : `${s.toFixed(1)}s`; }
export function toast(msg, isErr = false) { const el = document.createElement('div'); el.className = `fixed left-1/2 -translate-x-1/2 bottom-[72px] z-[60] px-3 py-1.5 rounded border text-[11px] font-mono ${isErr ? 'bg-err/10 border-err/50 text-err' : 'bg-surf2 border-ok/40 text-ok'}`; el.textContent = msg; document.body.appendChild(el); setTimeout(() => el.remove(), 3200); }
