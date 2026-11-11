use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use xd2figma::{
    converter::{Options, CANCELLED},
    exporter,
    parser::{Document, Extracted},
};
use zip::{write::SimpleFileOptions, ZipWriter};

const ARTWORK: &str = r#"{"children":[{"type":"artboard","id":"a","artboard":{"width":100,"height":80,"children":[
    {"type":"shape","name":"Caja","shape":{"type":"rect","width":10,"height":10}}]}}]}"#;

fn write_zip(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join(name);
    let mut zip = ZipWriter::new(File::create(&path).unwrap());
    for (entry, bytes) in entries {
        zip.start_file(*entry, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    path
}

fn valid(dir: &Path) -> PathBuf {
    write_zip(
        dir,
        "ok.xd",
        &[
            ("manifest.json", br#"{"name":"Prueba"}"#),
            ("artwork/scene.agc", ARTWORK.as_bytes()),
        ],
    )
}

fn package() -> Options {
    Options {
        import_package: true,
        ..Options::default()
    }
}

#[test]
fn opens_a_minimal_container() {
    let dir = tempfile::tempdir().unwrap();
    let document = Document::open(&valid(dir.path())).unwrap();
    assert_eq!(document.boards.len(), 1);
}

#[test]
fn rejects_path_traversal_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_zip(
        dir.path(),
        "evil.xd",
        &[
            ("manifest.json", b"{}"),
            ("../escape.agc", ARTWORK.as_bytes()),
        ],
    );
    let error = Document::open(&path).err().unwrap();
    assert!(error.contains("inválida"), "{error}");
}

#[test]
fn rejects_windows_reserved_characters() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_zip(dir.path(), "bad.xd", &[("artwork/a:b.agc", b"{}")]);
    assert!(Document::open(&path).is_err());
}

#[test]
fn rejects_containers_without_layers() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_zip(dir.path(), "empty.xd", &[("manifest.json", b"{}")]);
    let error = Document::open(&path).err().unwrap();
    assert!(error.contains("capas"), "{error}");
}

#[test]
fn rejects_files_that_are_not_zip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake.xd");
    std::fs::write(&path, b"no soy un zip").unwrap();
    assert!(Document::open(&path).is_err());
}

#[test]
fn tolerant_mode_skips_broken_json_but_strict_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_zip(
        dir.path(),
        "broken.xd",
        &[
            ("manifest.json", b"{}"),
            ("artwork/a.agc", ARTWORK.as_bytes()),
            ("artwork/b.agc", b"{ esto no es json"),
        ],
    );
    let mut warnings = Vec::new();
    let tolerant = Extracted::open(&path, |_, _, _| Ok(()))
        .unwrap()
        .parse(false, |_| {}, |w| warnings.push(w))
        .unwrap();
    assert_eq!(tolerant.boards.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert!(Document::open(&path).is_err());
}

#[test]
fn rejects_unsupported_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diseno.pdf");
    std::fs::write(&path, b"x").unwrap();
    let error = exporter::convert_file(&path, dir.path(), Options::default(), |_| {})
        .err()
        .unwrap();
    assert!(error.contains(".xd"));
}

#[test]
fn cancellation_stops_conversion_and_leaves_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = valid(dir.path());
    let output = tempfile::tempdir().unwrap();
    let polls = AtomicUsize::new(0);
    let error = exporter::convert_file_cancellable(
        &source,
        output.path(),
        package(),
        &|| polls.fetch_add(1, Ordering::SeqCst) >= 1,
        |_| {},
    )
    .err()
    .unwrap();
    assert_eq!(error, CANCELLED);
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}

#[test]
fn does_not_overwrite_existing_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let source = valid(dir.path());
    let output = tempfile::tempdir().unwrap();
    let first = exporter::convert_file(&source, output.path(), package(), |_| {}).unwrap();
    let second = exporter::convert_file(&source, output.path(), package(), |_| {}).unwrap();
    assert_ne!(first.output_path, second.output_path);
    assert!(first.output_path.exists() && second.output_path.exists());
}

#[test]
fn only_extracts_the_assets_that_are_used() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_zip(
        dir.path(),
        "assets.xd",
        &[
            ("manifest.json", b"{}"),
            ("artwork/a.agc", ARTWORK.as_bytes()),
            ("resources/unused.bin", &[0; 1024]),
            ("resources/used.bin", &[7, 8, 9]),
        ],
    );
    let document = Document::open(&path).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut document.open_asset("used.bin").unwrap(), &mut bytes).unwrap();
    assert_eq!(bytes, [7, 8, 9]);
    assert!(document.open_asset("missing.bin").is_err());
}
