mod native;

use crate::{
    converter::{self, Options, Summary},
    parser::Extracted,
};
use serde::Serialize;
use std::{
    cell::RefCell,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use tempfile::Builder;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub percent: u8,
    pub phase: &'static str,
    pub element: String,
    pub message: String,
    pub level: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub output_path: PathBuf,
    pub images_path: Option<PathBuf>,
    pub summary: Summary,
}

pub fn convert_file(
    source: &Path,
    destination: &Path,
    options: Options,
    progress: impl FnMut(Progress),
) -> Result<Report, String> {
    if !source
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("xd"))
    {
        return Err("Selecciona un archivo con extensión .xd".into());
    }
    let progress = RefCell::new(progress);
    let event = |percent, phase, element: &str, warning: Option<&str>| {
        (progress.borrow_mut())(Progress {
            percent,
            phase,
            element: element.to_owned(),
            message: warning.unwrap_or(phase).to_owned(),
            level: if warning.is_some() { "warning" } else { "info" },
        });
    };
    let extracted = Extracted::open(source, |index, total, name| {
        event(
            (25 * index / total.max(1)) as u8,
            "Descomprimiendo",
            name,
            None,
        );
    })?;
    let mut parse_warnings = Vec::new();
    let document = extracted.parse(
        options.strict,
        |name| event(26, "Parseando AGC", name, None),
        |warning| {
            event(26, "Parseando AGC", "", Some(&warning));
            parse_warnings.push(warning);
        },
    )?;
    let count = document
        .boards
        .iter()
        .map(|g| {
            g.children
                .iter()
                .filter(|n| n["type"] == "artboard")
                .count()
        })
        .sum::<usize>()
        .max(1);
    let mut board_index = 0;
    let mut percent = 40;
    event(percent, "Mapeando nodos", "", None);
    let mut design = converter::convert_with_progress(&document, options, |element, warning| {
        if warning.is_none() {
            percent = 40 + (30 * board_index / count) as u8;
            board_index += 1;
        }
        event(percent, "Mapeando nodos", element, warning);
    })?;
    design.summary.warnings.extend(parse_warnings);
    let mut temp = Builder::new()
        .prefix(".xd2figma-")
        .tempfile_in(destination)
        .map_err(|e| format!("Carpeta de salida: {e}"))?;
    let mut write_package = || -> Result<(), Box<dyn std::error::Error>> {
        let mut zip = ZipWriter::new(temp.as_file_mut());
        let compressed =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (i, id) in design.asset_ids.iter().enumerate() {
            event(
                72 + (18 * i / design.asset_ids.len()) as u8,
                "Generando paquete para Figma",
                id,
                None,
            );
            let mut asset = document.open_asset(id)?;
            zip.start_file(
                format!("assets/{id}"),
                stored.large_file(asset.metadata()?.len() > u32::MAX.into()),
            )?;
            io::copy(&mut asset, &mut zip)?;
        }
        for (path, contents) in [
            ("figma-importer/manifest.json", include_str!("../../../figma-importer/manifest.json")),
            ("figma-importer/code.js", include_str!("../../../figma-importer/code.js")),
            ("figma-importer/ui.html", include_str!("../../../figma-importer/ui.html")),
            ("IMPORTAR.txt", "Extrae el ZIP. En Figma Desktop registra figma-importer/manifest.json como plugin de desarrollo y ejecútalo. Selecciona la carpeta extraída y, tras importar, usa File > Save local copy para guardar el .fig.\n"),
        ] {
            zip.start_file(path, compressed)?;
            zip.write_all(contents.as_bytes())?;
        }
        event(92, "Generando paquete para Figma", "design.json", None);
        zip.start_file("design.json", compressed)?;
        serde_json::to_writer(&mut zip, &design)?;
        zip.finish()?;
        Ok(())
    };
    let extension = if options.import_package {
        write_package().map_err(|e| format!("Error al generar el paquete: {e}"))?;
        "xd2figma.zip"
    } else {
        event(75, "Codificando .fig", "", None);
        let (bytes, warnings) = native::export(&design, &document)?;
        for warning in warnings {
            if design.summary.warnings.insert(warning.clone()) {
                event(92, "Codificando .fig", "", Some(&warning));
            }
        }
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        "fig"
    };
    event(95, "Guardando resultado", "", None);
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("diseno");
    for index in 0..1000 {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(" ({})", index + 1)
        };
        let base = format!("{stem}{suffix}");
        let path = destination.join(format!("{base}.{extension}"));
        let folder = destination.join(format!("{base}.images"));
        if options.extract_images {
            match fs::create_dir(&folder) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("No se pudo crear la carpeta de imágenes: {e}")),
            }
            let copy_images = || -> Result<(), Box<dyn std::error::Error>> {
                for (index, id) in design.asset_ids.iter().enumerate() {
                    let mut file = document.open_asset(id)?;
                    let mut signature = [0; 12];
                    let mut peek = (&mut file).take(12);
                    let mut header = Vec::new();
                    peek.read_to_end(&mut header)?;
                    signature[..header.len()].copy_from_slice(&header);
                    if let Some(extension) = image_extension(&signature) {
                        let name = format!(
                            "{:05}-{name}.{extension}",
                            index + 1,
                            name = id.rsplit('/').next().unwrap_or("image")
                        );
                        let mut output = fs::File::create(folder.join(name))?;
                        output.write_all(&header)?;
                        io::copy(&mut file, &mut output)?;
                    }
                }
                Ok(())
            };
            if let Err(error) = copy_images() {
                let _ = fs::remove_dir_all(&folder);
                return Err(format!("No se pudieron extraer las imágenes: {error}"));
            }
        }
        match temp.persist_noclobber(&path) {
            Ok(_) => {
                event(100, "Conversión completada", "", None);
                return Ok(Report {
                    output_path: path,
                    images_path: options.extract_images.then_some(folder),
                    summary: design.summary,
                });
            }
            Err(error) => {
                if options.extract_images {
                    fs::remove_dir_all(&folder).map_err(|e| e.to_string())?;
                }
                if error.error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(format!("No se pudo guardar el paquete: {}", error.error));
                }
                temp = error.file;
            }
        }
    }
    Err("Se agotaron los nombres disponibles para el paquete".into())
}

fn image_extension(bytes: &[u8; 12]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"RIFF") && &bytes[8..] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}
