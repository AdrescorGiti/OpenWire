import { state, esc } from '../state.js';

export function libraryView() {
  return `
  <div class="flex-1 min-h-0 overflow-auto p-4 space-y-3">
    <div class="flex items-center justify-between gap-3">
      <h2 class="text-hi text-[13px] font-bold whitespace-nowrap overflow-hidden text-ellipsis">БИБЛИОТЕКА КОНФИГОВ <span class="font-mono text-dim text-[10px]">${esc(state.presetsDir || '~/Документы/OpenWire/Voices')}</span></h2>
      <button data-action="refresh-presets" class="px-2.5 py-1 rounded border border-line hover:border-hover text-[11px] text-mut whitespace-nowrap">Обновить</button>
    </div>
    ${state.presets.length ? state.presets.map((p) => `
      <div class="flex items-center gap-3 bg-surf border border-line rounded px-3 py-2">
        <span class="text-accent">≋</span>
        <span class="font-mono text-[11px] text-hi flex-1 truncate">${esc(p)}.toml</span>
        ${p === state.telemetry.active_preset ? '<span class="font-mono text-[9px] text-ok border border-ok/40 rounded px-1.5 py-[1px] shrink-0">ACTIVE</span>' : ''}
        <button data-lib-load="${esc(p)}" class="px-2 py-1 rounded bg-surf2 hover:bg-hover text-[10px] text-body shrink-0">Загрузить</button>
        <button data-lib-del="${esc(p)}" class="px-2 py-1 rounded bg-surf2 hover:bg-err/20 text-[10px] text-err shrink-0">Удалить</button>
      </div>`).join('') : `
      <div class="bg-surf border border-line border-dashed rounded p-6 text-center space-y-2">
        <div class="text-dim font-mono text-[11px]">Конфигов не найдено.</div>
        <div class="text-ghost text-[11px]">Сохраните пресет из панели DSP — он появится в этой папке.</div>
      </div>`}
  </div>`;
}
