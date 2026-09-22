import { state } from './state.js';
export const invoke = (cmd, args = {}) => window.__TAURI__.core.invoke(cmd, args);
export const tauriWin = () => window.__TAURI__.window.getCurrentWindow();
export async function refreshBoard() { state.board = await invoke('board_get'); }
export async function refreshDsp() { state.dsp = await invoke('dsp_get'); }
export async function refreshPresets() { state.presets = await invoke('presets_list'); state.presetsDir = await invoke('presets_dir'); }
export async function refreshSettings() { const view = await invoke('settings_get'); state.settings = view.settings; }
export const dspSet = (dto) => invoke('dsp_set', { dto });
export const presetLoad = (name) => invoke('preset_load', { name });
export const presetSave = (name, toml) => invoke('preset_save', { name, toml });
export const presetPreview = (toml) => invoke('preset_preview', { toml });
export const presetDelete = (name) => invoke('preset_delete', { name });
export const padPlay = (bank, slot) => invoke('pad_play', { bank, slot });
export const padStop = (bank, slot) => invoke('pad_stop', { bank, slot });
export const padImport = (bank, slot, path) => invoke('pad_import', { bank, slot, path });
export const padAdd = (bank) => invoke('pad_add', { bank });
export const padUpdate = (bank, pad) => invoke('pad_update', { bank, pad });
export const padClear = (bank, slot) => invoke('pad_clear', { bank, slot });
export const padWaveform = (bank, slot, points) => invoke('pad_waveform', { bank, slot, points });
export const padsStopAll = () => invoke('pads_stop_all');
export const settingsSet = (settings) => invoke('settings_set', { settings });
export const metersPoll = () => invoke('meters_poll');
