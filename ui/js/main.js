import { state, esc } from './state.js';
import { refreshBoard, refreshDsp, refreshPresets, refreshSettings } from './ipc.js';
import { titlebar, sidebar, toolbar, statusbar, settingsModal } from './ui/shell.js';
import { boardView, updatePads } from './ui/board.js';
import { dspView } from './ui/dsp.js';
import { libraryView } from './ui/library.js';
import { propsPanelHtml, refreshWaveform } from './ui/props.js';
import { registerEvents } from './events.js';
import { startMeterLoop } from './meters.js';
export function renderShell() { document.getElementById('app').innerHTML = `${titlebar()}<div class="flex flex-1 min-h-0">${sidebar()}<main class="flex-1 min-w-0 flex flex-col bg-deep">${toolbar()}<div class="flex-1 min-h-0 flex"><section id="viewHost" class="flex-1 min-w-0 flex flex-col"></section><aside id="propsHost" class="shrink-0"></aside></div>${statusbar()}</main></div>${settingsModal()}`; updateView(); updateRightPanels(); }
export function updateView() { const host = document.getElementById('viewHost'); if (!host) return; host.innerHTML = { dsp: dspView, library: libraryView }[state.view]?.() ?? boardView(); if (state.view === 'board') updatePads(); }
export function updateRightPanels() { const host = document.getElementById('propsHost'); if (!host) return; host.innerHTML = propsPanelHtml(); refreshWaveform(); }
async function loadAll() { await Promise.all([refreshBoard(), refreshDsp(), refreshPresets(), refreshSettings()]); renderShell(); }
async function boot() { registerEvents(); startMeterLoop(120); try { await loadAll(); } catch (err) { document.getElementById('app').innerHTML = `<div class="p-8 font-mono text-[12px] text-err">OpenWire bootstrap failed: ${esc(err)}</div>`; } }
boot();
