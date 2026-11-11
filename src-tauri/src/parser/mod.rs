use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::ZipArchive;

const MAX_JSON: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;

#[derive(Deserialize)]
pub struct Graphics {
    #[serde(default)]
    pub children: Vec<Value>,
    #[serde(default)]
    pub artboards: BTreeMap<String, Value>,
    #[serde(default)]
    pub resources: Value,
}

pub struct Extracted {
    directory: TempDir,
    entries: BTreeMap<String, usize>,
    archive: RefCell<ZipArchive<File>>,
}

impl Extracted {
    /// Valida el contenedor sin descomprimirlo: cada entrada se extrae a disco
    /// solo cuando alguien la necesita (JSON de capas o un recurso usado).
    pub fn open(
        path: &Path,
        mut progress: impl FnMut(usize, usize, &str) -> Result<(), String>,
    ) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("No se pudo abrir el XD: {e}"))?;
        let mut archive =
            ZipArchive::new(file).map_err(|e| format!("Contenedor XD inválido: {e}"))?;
        let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
        let mut entries = BTreeMap::new();
        let mut declared: u64 = 0;
        let total = archive.len();
        if total > MAX_ENTRIES {
            return Err("El contenedor XD tiene demasiadas entradas".into());
        }
        for index in 0..total {
            let entry = archive.by_index(index).map_err(|e| e.to_string())?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_owned();
            if !name.split('/').all(|part| {
                !part.is_empty()
                    && !matches!(part, "." | "..")
                    && !part.chars().any(|c| {
                        c.is_control()
                            || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                    })
            }) {
                return Err(format!("Ruta de entrada XD inválida: {name}"));
            }
            if entries.insert(name.clone(), index).is_some() {
                return Err(format!("Entrada XD duplicada: {name}"));
            }
            declared = declared
                .checked_add(entry.size())
                .filter(|total| *total <= MAX_EXPANDED)
                .ok_or("El contenedor supera el límite de 8 GiB descomprimidos")?;
            progress(index, total, &name)?;
        }
        Ok(Self {
            directory,
            entries,
            archive: RefCell::new(archive),
        })
    }

    /// Ruta en disco de la entrada, extrayéndola la primera vez que se pide.
    /// El nombre original nunca se usa para escribir: el archivo es el índice.
    fn path(&self, name: &str) -> Result<PathBuf, String> {
        let index = *self
            .entries
            .get(name)
            .ok_or_else(|| format!("Entrada ausente: {name}"))?;
        let target = self.directory.path().join(index.to_string());
        if target.exists() {
            return Ok(target);
        }
        let mut archive = self.archive.borrow_mut();
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("{name}: {e}"))?;
        let size = entry.size();
        let mut output = File::create(&target).map_err(|e| format!("{name}: {e}"))?;
        let copied = io::copy(&mut (&mut entry).take(size + 1), &mut output)
            .map_err(|e| format!("{name}: {e}"));
        let result = match copied {
            Ok(copied) if copied == size => Ok(()),
            Ok(_) => Err(format!("Tamaño ZIP incorrecto: {name}")),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            drop(output);
            let _ = fs::remove_file(&target);
            return Err(error);
        }
        Ok(target)
    }

    fn json<T: DeserializeOwned>(&self, name: &str) -> Result<T, String> {
        let path = self.path(name)?;
        let file = File::open(path).map_err(|e| format!("{name}: {e}"))?;
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_JSON {
            return Err(format!("{name} excede el límite de 128 MiB para un JSON de capas; divide el diseño en archivos más pequeños"));
        }
        serde_json::from_reader(io::BufReader::new(file)).map_err(|e| format!("{name}: {e}"))
    }

    pub fn parse(
        self,
        strict: bool,
        mut progress: impl FnMut(&str),
        mut warn: impl FnMut(String),
    ) -> Result<Document, String> {
        let manifest_name = if self.entries.contains_key("manifest.json") {
            "manifest.json"
        } else {
            "manifest"
        };
        progress(manifest_name);
        let manifest: Value = self.json(manifest_name)?;
        if !manifest.is_object() {
            return Err("El manifest de XD debe ser un objeto JSON".into());
        }
        let mut boards = Vec::new();
        let mut resources = None;
        let mut interactions = None;
        let mut assets = BTreeMap::new();
        for name in self.entries.keys() {
            let result = if name.starts_with("artwork/")
                && (name.ends_with(".agc") || name.ends_with("/articulate.json"))
            {
                progress(name);
                self.json(name).map(|data| boards.push(data))
            } else if name.starts_with("resources/") && name.ends_with(".agc") {
                progress(name);
                self.json::<Graphics>(name).map(|mut data| {
                    if let Some(combined) = resources.as_mut() {
                        merge_resources(combined, &mut data);
                    } else {
                        resources = Some(data);
                    }
                })
            } else if name == "interactions/interactions.json" {
                progress(name);
                self.json(name).map(|data| interactions = Some(data))
            } else {
                if let Some(id) = name.strip_prefix("resources/") {
                    assets.insert(id.to_owned(), name.clone());
                }
                continue;
            };
            if let Err(error) = result {
                if strict {
                    return Err(error);
                }
                warn(format!("Se omitió {error}"));
            }
        }
        if boards.is_empty() {
            return Err("El XD no contiene árboles de capas legibles".into());
        }
        Ok(Document {
            manifest,
            boards,
            resources,
            interactions,
            assets,
            extracted: self,
        })
    }
}

fn merge_resources(combined: &mut Graphics, data: &mut Graphics) {
    fn merge(target: &mut Value, incoming: Value) {
        match (target, incoming) {
            (Value::Object(target), Value::Object(incoming)) => {
                for (key, value) in incoming {
                    merge(target.entry(key).or_insert(Value::Null), value);
                }
            }
            (Value::Array(target), Value::Array(mut incoming)) => target.append(&mut incoming),
            (target, value) if target.is_null() => *target = value,
            _ => {}
        }
    }
    combined.children.append(&mut data.children);
    combined.artboards.append(&mut data.artboards);
    merge(&mut combined.resources, std::mem::take(&mut data.resources));
}

pub struct Document {
    pub manifest: Value,
    pub boards: Vec<Graphics>,
    pub resources: Option<Graphics>,
    pub interactions: Option<Value>,
    pub assets: BTreeMap<String, String>,
    extracted: Extracted,
}

impl Document {
    pub fn open(path: &Path) -> Result<Self, String> {
        Extracted::open(path, |_, _, _| Ok(()))?.parse(true, |_| {}, |_| {})
    }

    pub(crate) fn thumbnail(&self) -> Option<Vec<u8>> {
        for name in ["thumbnail.png", "preview.png"] {
            let Ok(path) = self.extracted.path(name) else {
                continue;
            };
            let Ok(file) = File::open(path) else { continue };
            let mut bytes = Vec::new();
            if file
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .is_ok()
                && bytes.len() <= 2 * 1024 * 1024
                && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            {
                return Some(bytes);
            }
        }
        None
    }

    pub fn open_asset(&self, id: &str) -> Result<File, String> {
        let name = self
            .assets
            .get(id)
            .ok_or_else(|| format!("Recurso ausente: {id}"))?;
        File::open(self.extracted.path(name)?).map_err(|e| format!("{name}: {e}"))
    }
}
