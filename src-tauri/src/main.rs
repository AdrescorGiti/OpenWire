mod audio;
mod input;
mod presets;
mod soundboard;
mod state;

use crate::audio::dsp::DspSnapshot;
use crate::soundboard::bank::{Pad, Soundboard};
use crate::state::{AppState, DspDto, Settings};
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager, State, WindowEvent,
};

#[derive(Serialize)]
struct MeterFrame {
    mic: f32,
    out: f32,
    active_voices: usize,
    engine_running: bool,
    quantum: u32,
    latency_ms: f32,
    pipewire_version: String,
    active_preset: Option<String>,
}

#[tauri::command]
fn meters_poll(state: State<'_, AppState>) -> MeterFrame {
    MeterFrame {
        mic: state.meters.mic(),
        out: state.meters.out(),
        active_voices: state.mixer.active_voices(),
        engine_running: state.engine.lock().map(|e| e.is_some()).unwrap_or(false),
        quantum: state.meters.quantum(),
        latency_ms: state.meters.latency_ms(),
        pipewire_version: state.meters.version(),
        active_preset: state
            .active_preset
            .read()
            .expect("active preset lock")
            .clone(),
    }
}
#[tauri::command]
fn dsp_get(state: State<'_, AppState>) -> DspDto {
    state.dsp_dto()
}
#[tauri::command]
fn dsp_set(state: State<'_, AppState>, dto: DspDto) -> Result<(), String> {
    dto.apply(&state.dsp);
    Ok(())
}
#[tauri::command]
fn presets_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut names = state.presets.list_presets().map_err(|e| e.to_string())?;
    names.insert(0, "Обычный голос".into());
    Ok(names)
}
#[tauri::command]
fn presets_dir(state: State<'_, AppState>) -> String {
    state.presets.dir().display().to_string()
}
#[tauri::command]
fn preset_load(state: State<'_, AppState>, name: String) -> Result<DspDto, String> {
    if name == "Обычный голос" {
        let dto: DspDto = DspSnapshot::default().into();
        dto.apply(&state.dsp);
        set_active_preset(&state, "Обычный голос");
        return Ok(dto);
    }
    let preset = state
        .presets
        .load_preset(&name)
        .map_err(|e| e.to_string())?;
    preset.apply(&state.dsp);
    set_active_preset(&state, &preset.meta.name);
    Ok(state.dsp_dto())
}
#[tauri::command]
fn preset_save(state: State<'_, AppState>, name: String, toml: String) -> Result<String, String> {
    let path = state
        .presets
        .save_preset(&name, &toml)
        .map_err(|e| e.to_string())?;
    if let Ok(preset) = state.presets.load_preset(&name) {
        preset.apply(&state.dsp);
        set_active_preset(&state, &preset.meta.name);
    }
    Ok(path.display().to_string())
}
#[tauri::command]
fn preset_preview(state: State<'_, AppState>, toml: String) -> Result<DspDto, String> {
    let preset: crate::presets::voice_config::VoicePreset =
        toml::from_str(&toml).map_err(|e| e.to_string())?;
    preset.apply(&state.dsp);
    set_active_preset(&state, &preset.meta.name);
    Ok(state.dsp_dto())
}
#[tauri::command]
fn preset_delete(state: State<'_, AppState>, name: String) -> Result<(), String> {
    if name == "Обычный голос" {
        return Err("Встроенный пресет нельзя удалить".into());
    }
    state
        .presets
        .delete_preset(&name)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn preset_active(state: State<'_, AppState>) -> Option<String> {
    state
        .active_preset
        .read()
        .expect("active preset lock")
        .clone()
}
fn set_active_preset(state: &State<'_, AppState>, name: &str) {
    *state.active_preset.write().expect("active preset lock") = Some(name.to_string());
}

fn disable_effects(state: &AppState) {
    use std::sync::atomic::Ordering;
    state.dsp.pitch_enabled.store(false, Ordering::Relaxed);
    state.dsp.radio_enabled.store(false, Ordering::Relaxed);
    state
        .dsp
        .formant_shift
        .store(1.0f32.to_bits(), Ordering::Relaxed);
    state
        .dsp
        .eq_low_db
        .store(0.0f32.to_bits(), Ordering::Relaxed);
    state
        .dsp
        .eq_mid_gain_db
        .store(0.0f32.to_bits(), Ordering::Relaxed);
    state
        .dsp
        .eq_high_db
        .store(0.0f32.to_bits(), Ordering::Relaxed);
}

fn apply_next_preset(state: &AppState) {
    let Ok(names) = state.presets.list_presets() else {
        return;
    };
    if names.is_empty() {
        return;
    }
    let active = state
        .active_preset
        .read()
        .ok()
        .and_then(|name| name.clone());
    let index = active
        .as_deref()
        .and_then(|name| names.iter().position(|candidate| candidate == name))
        .map(|index| (index + 1) % names.len())
        .unwrap_or(0);
    let name = &names[index];
    if let Ok(preset) = state.presets.load_preset(name) {
        preset.apply(&state.dsp);
        *state.active_preset.write().expect("active preset lock") = Some(preset.meta.name);
    }
}
#[tauri::command]
fn board_get(state: State<'_, AppState>) -> Soundboard {
    state.sound.snapshot()
}
#[tauri::command]
fn pad_play(state: State<'_, AppState>, bank: usize, slot: usize) -> Result<(), String> {
    state.sound.play(bank, slot).map_err(|e| e.to_string())
}
#[tauri::command]
fn pad_stop(state: State<'_, AppState>, bank: usize, slot: usize) {
    state.sound.stop_pad(bank, slot);
}
#[tauri::command]
fn pad_import(
    state: State<'_, AppState>,
    bank: usize,
    slot: usize,
    path: String,
) -> Result<Pad, String> {
    state
        .sound
        .import(bank, slot, std::path::PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn pad_add(state: State<'_, AppState>, bank: usize) -> Result<Pad, String> {
    state.sound.add_pad(bank).map_err(|e| e.to_string())
}
#[tauri::command]
fn pad_update(state: State<'_, AppState>, bank: usize, pad: Pad) -> Result<(), String> {
    state
        .sound
        .update_pad(bank, pad.clone())
        .map_err(|e| e.to_string())?;
    state.hotkeys.clear_binding(bank, pad.id);
    if let Some(label) = pad.hotkey.as_deref() {
        if !label.trim().is_empty() {
            if let Err(e) = state.hotkeys.rebind_label(bank, pad.id, label) {
                eprintln!("[openwire][hotkeys] {e}");
            }
        }
    }
    Ok(())
}
#[tauri::command]
fn hotkey_rebind(
    state: State<'_, AppState>,
    bank: usize,
    slot: usize,
    label: String,
) -> Result<(), String> {
    state.hotkeys.rebind_label(bank, slot, &label)
}
#[tauri::command]
fn pad_clear(state: State<'_, AppState>, bank: usize, slot: usize) -> Result<(), String> {
    state.sound.clear_pad(bank, slot).map_err(|e| e.to_string())
}
#[tauri::command]
fn pad_waveform(
    state: State<'_, AppState>,
    bank: usize,
    slot: usize,
    points: usize,
) -> Result<Vec<f32>, String> {
    state
        .sound
        .waveform(bank, slot, points.clamp(16, 2048))
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn pads_stop_all(state: State<'_, AppState>) {
    state.sound.stop_all();
}

#[derive(Serialize)]
struct SettingsView {
    settings: Settings,
}
#[tauri::command]
fn settings_get(state: State<'_, AppState>) -> SettingsView {
    SettingsView {
        settings: state.settings.read().expect("settings lock").clone(),
    }
}
#[tauri::command]
fn settings_set(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    state.dsp.ducking_enabled.store(
        settings.ducking_enabled,
        std::sync::atomic::Ordering::Relaxed,
    );
    state.dsp.monitor_enabled.store(
        settings.monitor_enabled,
        std::sync::atomic::Ordering::Relaxed,
    );
    settings.save().map_err(|e| e.to_string())?;
    *state.settings.write().expect("settings lock") = settings;
    Ok(())
}

fn main() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
    {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let open = MenuItemBuilder::with_id("open", "Открыть окно").build(app)?;
            let next_preset =
                MenuItemBuilder::with_id("next-preset", "Следующий пресет").build(app)?;
            let disable_fx =
                MenuItemBuilder::with_id("disable-effects", "Выключить эффекты").build(app)?;
            let stop = MenuItemBuilder::with_id("stop", "Остановить все звуки").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Выйти из OpenWire").build(app)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[&open, &next_preset, &disable_fx, &stop, &separator, &quit],
            )?;

            TrayIconBuilder::with_id("openwire-tray")
                .menu(&menu)
                .tooltip("OpenWire — звуковая доска")
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("application icon"),
                )
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "next-preset" => apply_next_preset(app.state::<AppState>().inner()),
                    "disable-effects" => disable_effects(app.state::<AppState>().inner()),
                    "stop" => app.state::<AppState>().sound.stop_all(),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            app.manage(AppState::bootstrap().expect("failed to bootstrap OpenWire services"));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            meters_poll,
            dsp_get,
            dsp_set,
            presets_list,
            presets_dir,
            preset_load,
            preset_save,
            preset_preview,
            preset_delete,
            preset_active,
            board_get,
            pad_play,
            pad_stop,
            pad_import,
            pad_add,
            pad_update,
            hotkey_rebind,
            pad_clear,
            pad_waveform,
            pads_stop_all,
            settings_get,
            settings_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenWire");
}
