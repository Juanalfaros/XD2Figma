use crate::{
    converter::Options,
    exporter::{self, Progress, Report},
};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{ipc::Channel, Emitter, WebviewWindow};

#[derive(Clone, Serialize)]
struct ProgressEvent {
    source: PathBuf,
    #[serde(flatten)]
    progress: Progress,
}

#[tauri::command]
pub async fn convert_file(
    window: WebviewWindow,
    source: PathBuf,
    destination: PathBuf,
    options: Options,
    progress: Channel<Progress>,
) -> Result<Report, String> {
    tauri::async_runtime::spawn_blocking(move || {
        exporter::convert_file(&source, &destination, options, |update| {
            let _ = window.emit_to(
                window.label(),
                "conversion-progress",
                ProgressEvent {
                    source: source.clone(),
                    progress: update.clone(),
                },
            );
            let _ = progress.send(update);
        })
    })
    .await
    .map_err(|e| format!("El proceso de conversión se interrumpió: {e}"))?
}

#[tauri::command]
pub async fn reveal_output(path: PathBuf) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = path
            .canonicalize()
            .map_err(|e| format!("El archivo ya no está disponible: {e}"))?;
        if !path.is_file() {
            return Err("Selecciona un archivo generado".into());
        }
        #[cfg(target_os = "macos")]
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status();
        #[cfg(target_os = "windows")]
        let status = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .status();
        #[cfg(target_os = "linux")]
        let status = std::process::Command::new("xdg-open")
            .arg(
                path.parent()
                    .ok_or("El archivo no tiene carpeta de origen")?,
            )
            .status();
        let status = status.map_err(|e| format!("No se pudo abrir la carpeta: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("El sistema no pudo mostrar el archivo".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
