use super::CmdResult;
use crate::{
    cmd::StringifyErr as _,
    core::{
        SilentUpdater,
        updater::{DownloadEvent, InstallUpdateOutcome},
    },
    feat,
    utils::dirs,
};
use tauri::{AppHandle, Manager as _, ipc::Channel};
use tristr::alias::String;

#[derive(serde::Deserialize, serde::Serialize, specta::Type)]
pub struct WindowsApp {
    pub name: std::string::String,
    pub path: std::string::String,
}

#[tauri::command]
#[specta::specta]
pub async fn list_windows_apps() -> CmdResult<Vec<WindowsApp>> {
    #[cfg(windows)]
    {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use std::{path::PathBuf, time::Duration};
        use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

        let system_root = std::env::var_os("SystemRoot").ok_or("Windows system directory is unavailable")?;
        let powershell = PathBuf::from(system_root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        let script = include_str!("windows_apps.ps1");
        let encoded = STANDARD.encode(script.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>());
        let output = tokio::time::timeout(
            Duration::from_secs(20),
            tokio::process::Command::new(powershell)
                .creation_flags(CREATE_NO_WINDOW)
                .kill_on_drop(true)
                .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
                .output(),
        )
        .await
        .map_err(|_| "Timed out reading Windows applications")?
        .stringify_err()?;

        if !output.status.success() {
            return Err(super::CommandFailure::plain(
                std::string::String::from_utf8_lossy(&output.stderr).trim(),
            ));
        }
        serde_json::from_slice(&output.stdout).stringify_err()
    }
    #[cfg(not(windows))]
    Ok(Vec::new())
}

#[tauri::command]
#[specta::specta]
pub async fn open_app_dir() -> CmdResult<()> {
    let app_dir = dirs::app_home_dir().stringify_err()?;
    open::that(app_dir).stringify_err()
}

#[tauri::command]
#[specta::specta]
pub async fn open_core_dir() -> CmdResult<()> {
    let core_dir = tauri::utils::platform::current_exe().stringify_err()?;
    let core_dir = core_dir.parent().ok_or("failed to get core dir")?;
    open::that(core_dir).stringify_err()
}

#[tauri::command]
#[specta::specta]
pub async fn open_logs_dir() -> CmdResult<()> {
    let log_dir = dirs::app_logs_dir().stringify_err()?;
    open::that(log_dir).stringify_err()
}

#[tauri::command]
#[specta::specta]
pub fn open_devtools(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if !window.is_devtools_open() {
            window.open_devtools();
        } else {
            window.close_devtools();
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn exit_app() {
    feat::quit().await;
}

#[tauri::command]
#[specta::specta]
pub async fn restart_app() -> CmdResult<()> {
    feat::restart_app().await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn install_update(
    app_handle: AppHandle,
    version: std::string::String,
    on_event: Channel<DownloadEvent>,
) -> CmdResult<InstallUpdateOutcome> {
    SilentUpdater::global()
        .install_update(&app_handle, &version, |event| {
            let _ = on_event.send(event);
        })
        .await
        .stringify_err()
}

#[tauri::command]
#[specta::specta]
pub fn cancel_update_download() {
    SilentUpdater::global().cancel_download();
}

#[tauri::command]
#[specta::specta]
pub fn get_app_dir() -> CmdResult<std::string::String> {
    let app_home_dir = dirs::app_home_dir().stringify_err()?.to_string_lossy().into();
    Ok(app_home_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn download_icon_cache(
    url: std::string::String,
    name: std::string::String,
) -> CmdResult<std::string::String> {
    let url: String = url.into();
    let name: String = name.into();
    feat::download_icon_cache(url, name).await.map(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_icon_file(path: std::string::String, icon_info: feat::IconInfo) -> CmdResult<std::string::String> {
    let path: String = path.into();
    feat::copy_icon_file(path, icon_info).await.map(Into::into)
}
