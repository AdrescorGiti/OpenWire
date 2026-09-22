import { state, esc, padKey, formatMs } from '../state.js';

export function boardView() {
  return `<div class="flex-1 min-h-0 overflow-auto p-4 space-y-3">
    <div id="padGrid" class="grid grid-cols-5 gap-3"></div>
    <button data-action="add-pad" class="w-full h-[44px] rounded border border-dashed border-line hover:border-accent hover:bg-surf text-[11px] text-mut hover:text-accent">＋ Добавить пад</button>
  </div>`;
}

export function updatePads() {
  const grid = document.getElementById('padGrid');
  if (!grid || !state.board) return;
  const pads = state.board.banks[state.bank]?.pads ?? [];
  grid.innerHTML = pads.map((p) => {
    const key = padKey(state.bank, p.id);
    const looping = state.latched.has(key);
    const playing = state.playing.has(key) || looping;
    const selected = state.selected.bank === state.bank && state.selected.slot === p.id;
    const filtered = state.search && !p.name.toLowerCase().includes(state.search.toLowerCase());
    const empty = !p.path && !p.duration_ms;
    let frame = 'border-strong hover:border-hover';
    if (empty) frame = 'border-line border-dashed hover:border-hover';
    if (selected) frame = 'border-accent';
    if (playing) frame = 'border-ok';
    return `
    <button data-pad="${p.id}" class="pad ${playing ? 'pad-playing' : ''} ${looping ? 'pad-looping' : ''} text-left h-[104px] bg-surf border ${frame} rounded p-3 flex flex-col relative overflow-hidden ${filtered ? 'opacity-25' : 'hover:bg-surf2'}">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[10px] text-ghost">${String(p.id + 1).padStart(2, '0')}</span>
        <span class="font-mono text-[10px] ${playing ? 'text-ok' : 'text-dim'}">[${esc(p.hotkey || '·')}]</span>
      </div>
      <div class="text-[18px] mt-1">${empty ? '<span class="text-ghost">＋</span>' : (playing ? '<span class="text-ok">♫</span>' : '<span class="text-mut">▷</span>')}</div>
      <div class="mt-auto truncate text-[11px] ${empty ? 'text-ghost' : 'text-body'}">${esc(p.name || 'пустой пад')}</div>
      ${empty ? '' : `<div class="font-mono text-[8px] text-ghost truncate">${esc(formatMs(p.duration_ms))}${p.play_mode && p.play_mode !== 'oneshot' ? ' · ' + esc(p.play_mode) : ''}${p.bass_boost ? ' · BASS' : ''}</div>`}
      ${playing ? `<div class="absolute bottom-0 left-0 h-[3px] bg-ok pad-progress" style="animation-duration:${(p.duration_ms || 1500)}ms"></div>` : ''}
    </button>`;
  }).join('');
}
