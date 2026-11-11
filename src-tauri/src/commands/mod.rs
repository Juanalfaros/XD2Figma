use crate::{
    converter::Options,
    exporter::{self, Progress, Report},
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{ipc::Channel, State};

/// Señal de cancelación compartida por el lote en curso.
#[derive(Default)]
pub struct Cancel(Arc<AtomicBool>);

#[tauri::command]
pub fn cancel_conversion(cancel: State<'_, Cancel>) {
    cancel.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn convert_file(
    cancel: State<'_, Cancel>,
    source: PathBuf,
    destination: PathBuf,
    options: Options,
    progress: Channel<Progress>,
) -> Result<Report, String> {
    // Cada archivo arranca sin cancelación pendiente; el usuario puede volver a cancelar.
    cancel.0.store(false, Ordering::SeqCst);
    let flag = Arc::clone(&cancel.0);
    tauri::async_runtime::spawn_blocking(move || {
        exporter::convert_file_cancellable(
            &source,
            &destination,
            options,
            &|| flag.load(Ordering::SeqCst),
            |update| {
                let _ = progress.send(update);
            },
        )
    })
    .await
    .map_err(|e| format!("El proceso de conversión se interrumpió: {e}"))?
}

#[tauri::command]
pub async fn save_log(path: PathBuf, contents: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if contents.len() > 8 * 1024 * 1024 {
            return Err("El registro es demasiado grande para exportarlo".to_owned());
        }
        fs::write(&path, contents).map_err(|e| format!("No se pudo guardar el registro: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn reveal_output(path: PathBuf) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = path
            .canonicalize()
            .map_err(|e| format!("El archivo ya no está disponible: {e}"))?;
        if !path.is_dir() && !path.is_file() {
            return Err("Selecciona un archivo o carpeta generados".into());
        }
        let target = path;
        #[cfg(target_os = "macos")]
        let status = if target.is_dir() {
            std::process::Command::new("open").arg(&target).status()
        } else {
            std::process::Command::new("open")
                .arg("-R")
                .arg(&target)
                .status()
        };
        #[cfg(target_os = "windows")]
        let status = {
            // explorer.exe devuelve 1 aunque abra la ventana, así que solo se comprueba el arranque.
            let mut command = std::process::Command::new("explorer.exe");
            if target.is_dir() {
                command.arg(&target);
            } else {
                command.arg(format!("/select,{}", target.display()));
            }
            return command
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("No se pudo abrir la carpeta: {e}"));
        };
        #[cfg(target_os = "linux")]
        let status = {
            let folder = if target.is_dir() {
                Some(target.as_path())
            } else {
                target.parent()
            };
            std::process::Command::new("xdg-open")
                .arg(folder.ok_or("El archivo no tiene carpeta de origen")?)
                .status()
        };
        #[cfg(not(target_os = "windows"))]
        {
            let status = status.map_err(|e| format!("No se pudo abrir la carpeta: {e}"))?;
            if status.success() {
                Ok(())
            } else {
                Err("El sistema no pudo mostrar el archivo".into())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
