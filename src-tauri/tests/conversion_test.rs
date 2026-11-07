use quick_xml::{events::Event, Reader};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};
use xd2figma::{converter::Options, exporter};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn fixture() -> PathBuf {
    std::env::var_os("XD_FIXTURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Flipeame - Landing Page.xd")
        })
}

fn print_tree(node: &Value, depth: usize, counts: &mut BTreeMap<String, usize>) {
    let kind = node["type"].as_str().expect("taxonomía Figma");
    let name = node["name"].as_str().unwrap_or_default();
    *counts.entry(kind.to_owned()).or_default() += 1;
    println!("{:indent$}{kind} {name}", "", indent = depth * 2);
    if let Some(network) = node["vector_network"].as_object() {
        let vertices = network["vertices"].as_array().unwrap();
        let segments = network["segments"].as_array().unwrap();
        for segment in segments {
            assert!((segment["start"].as_u64().unwrap() as usize) < vertices.len());
            assert!((segment["end"].as_u64().unwrap() as usize) < vertices.len());
        }
        *counts.entry("VectorNetwork".into()).or_default() += 1;
    }
    if let Some(svg) = node["fallback_svg"].as_str() {
        let mut xml = Reader::from_str(svg);
        assert!(
            matches!(xml.read_event().unwrap(), Event::Start(tag) if tag.name().as_ref() == "svg")
        );
        while !matches!(xml.read_event().expect("SVG bien formado"), Event::Eof) {}
        *counts.entry("Fallback SVG".into()).or_default() += 1;
    }
    for child in node["children"].as_array().unwrap() {
        print_tree(child, depth + 1, counts);
    }
}

#[test]
#[ignore = "Requiere un XD local mediante XD_FIXTURE; no se distribuyen diseños de terceros"]
fn converts_local_xd_to_editable_figma_payload() {
    let start = Instant::now();
    let output = tempfile::tempdir().unwrap();
    let options = Options {
        import_package: true,
        preserve_prototype: true,
        extract_images: true,
        ..Options::default()
    };
    let mut events = Vec::new();
    let mut phases = BTreeMap::new();
    let report = exporter::convert_file(&fixture(), output.path(), options, |event| {
        phases.entry(event.phase).or_insert_with(|| start.elapsed());
        events.push(event);
    })
    .expect("convertir el XD real");
    assert_eq!(report.summary.artboards, 2);
    assert!(report.summary.layers > 100);
    assert_eq!(events.last().unwrap().percent, 100);
    assert!(events.windows(2).all(|p| p[0].percent <= p[1].percent));
    assert!(events
        .iter()
        .any(|e| e.phase == "Mapeando nodos" && !e.element.is_empty()));
    assert!(report.images_path.unwrap().read_dir().unwrap().count() > 0);

    let mut zip = ZipArchive::new(File::open(report.output_path).unwrap()).unwrap();
    let design: Value = serde_json::from_reader(zip.by_name("design.json").unwrap()).unwrap();
    assert_eq!(design["schema_version"], 2);
    assert!(design["prototype"].is_object());
    assert!(!design["components"].is_null());
    assert!(zip.by_name("figma-importer/manifest.json").is_ok());
    assert!(zip.by_name("figma-importer/code.js").is_ok());
    assert!(!zip.file_names().any(|name| name.starts_with("artboards/")));
    for id in design["asset_ids"].as_array().unwrap() {
        let mut asset = zip
            .by_name(&format!("assets/{}", id.as_str().unwrap()))
            .unwrap();
        assert!(std::io::copy(&mut asset, &mut std::io::sink()).unwrap() > 0);
    }
    let mut counts = BTreeMap::new();
    for board in design["artboards"].as_array().unwrap() {
        assert_eq!(board["node"]["type"], "FRAME");
        print_tree(&board["node"], 0, &mut counts);
    }
    for node in design["pasteboard"].as_array().unwrap() {
        print_tree(node, 0, &mut counts);
    }
    for kind in [
        "FRAME",
        "TEXT",
        "RECTANGLE",
        "VECTOR",
        "COMPONENT",
        "VectorNetwork",
    ] {
        assert!(counts[kind] > 0, "{kind}");
    }
    println!(
        "Nodos: {counts:?}\nTiempos acumulados: {phases:?}\nVerificación completa: {:?}",
        start.elapsed()
    );
}

#[test]
fn maps_repeat_grid_and_limits_svg_to_unsupported_nodes() {
    let output = tempfile::tempdir().unwrap();
    let source = output.path().join("repeat.xd");
    let mut zip = ZipWriter::new(File::create(&source).unwrap());
    let rect =
        json!({"type":"shape","name":"Celda","shape":{"type":"rect","width":40,"height":20}});
    let artwork = json!({"children":[{"type":"artboard","id":"a","artboard":{"width":120,"height":90,"children":[
        {"type":"repeatGrid","name":"Tarjetas","repeatGrid":{"columns":2,"horizontalPadding":8,"verticalPadding":12,
            "children":[rect,rect,rect]}},
        {"type":"shape","name":"Curva","shape":{"type":"path","path":"M0 0 Q10 10 20 0 A10 10 0 0 1 40 0 Z"},
            "style":{"fill":{"type":"solid","color":{"value":{"r":255,"g":0,"b":0}}},
                "filters":[{"type":"blur","params":{"radius":3}}]}},
        {"type":"proprietary","id":"unsupported","name":"Propietario","visualBounds":{"width":48,"height":24}}
    ]}}]});
    for (name, bytes) in [
        ("manifest.json", br#"{"name":"Prueba"}"#.to_vec()),
        ("artwork/scene.agc", serde_json::to_vec(&artwork).unwrap()),
        ("resources/patterns/example.bin", vec![1, 2, 3]),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
    let mut warnings = Vec::new();
    let report = exporter::convert_file(
        &source,
        output.path(),
        Options {
            import_package: true,
            ..Options::default()
        },
        |event| {
            if event.level == "warning" {
                warnings.push(event);
            }
        },
    )
    .unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("Propietario"));
    let mut archive = ZipArchive::new(File::open(report.output_path).unwrap()).unwrap();
    let design: Value = serde_json::from_reader(archive.by_name("design.json").unwrap()).unwrap();
    let nodes = &design["artboards"][0]["node"]["children"];
    assert_eq!(nodes[0]["auto_layout"]["layoutWrap"], "WRAP");
    assert_eq!(nodes[0]["auto_layout"]["counterAxisSpacing"], 12.);
    assert_eq!(nodes[1]["appearance"]["effects"][0]["type"], "LAYER_BLUR");
    assert!(
        nodes[1]["vector_network"]["segments"]
            .as_array()
            .unwrap()
            .len()
            >= 3
    );
    assert!(nodes[1]["fallback_svg"].is_null());
    assert!(nodes[2]["fallback_svg"].is_string());
    let mut nested = Vec::new();
    archive
        .by_name("assets/patterns/example.bin")
        .unwrap()
        .read_to_end(&mut nested)
        .unwrap();
    assert_eq!(nested, [1, 2, 3]);

    let strict = Options {
        strict: true,
        ..Options::default()
    };
    let error = exporter::convert_file(&source, output.path(), strict, |_| {})
        .err()
        .unwrap();
    assert!(error.contains("Propietario"));
    assert!(!output.path().join("repeat (2).xd2figma.zip").exists());
}

#[test]
#[ignore = "Requiere un XD local mediante XD_FIXTURE; no se distribuyen diseños de terceros"]
fn exports_xd_as_editable_fig() {
    use figcraft_doc::NodeKind;

    let temporary = tempfile::tempdir().unwrap();
    let destination = std::env::var_os("XD_FIG_OUTPUT_DIR").map(PathBuf::from);
    let report = exporter::convert_file(
        &fixture(),
        destination.as_deref().unwrap_or(temporary.path()),
        Options::default(),
        |_| {},
    )
    .expect("exportación binaria");
    assert_eq!(
        report.output_path.extension().and_then(|s| s.to_str()),
        Some("fig")
    );
    let mut archive = ZipArchive::new(File::open(&report.output_path).unwrap()).unwrap();
    let mut magic = [0; 8];
    archive
        .by_name("canvas.fig")
        .unwrap()
        .read_exact(&mut magic)
        .unwrap();
    assert_eq!(&magic, b"fig-kiwi");
    assert!(archive.by_name("meta.json").is_ok());
    assert!(archive.file_names().any(|name| name.starts_with("images/")));
    let bytes = std::fs::read(&report.output_path).unwrap();
    let (doc, warnings) = figcraft_format::fig::import(&bytes, "XD convertido").unwrap();
    for label in ["Inicio", "Servicios", "Faqs", "Nuestras Marcas"] {
        assert!(
            doc.nodes()
                .any(|node| matches!(&node.kind, NodeKind::Text(text)
            if text.characters == label && text.style.case == figcraft_doc::TextCase::Upper)),
            "Mayúsculas editables para {label}"
        );
    }
    let hero = doc
        .nodes()
        .find_map(|node| match &node.kind {
            NodeKind::Text(text) if text.characters.starts_with("Somos una StartUp Chilena") => {
                Some(text)
            }
            _ => None,
        })
        .expect("titular editable");
    assert_eq!(hero.style.font_style.replace(" ", ""), "SemiBold");
    for (start, end) in [(10, 26), (35, 49)] {
        assert!(hero.runs.iter().any(|run| run.start == start
            && run.end == end
            && run.style.font_style.replace(" ", "") == "SemiBold"
            && run.style.fills.is_some()));
    }
    assert!(doc
        .nodes()
        .any(|node| node.blend == figcraft_doc::color::BlendMode::Multiply));
    assert!(doc
        .nodes()
        .any(|node| node.blend == figcraft_doc::color::BlendMode::Overlay));
    assert!(doc
        .images
        .values()
        .all(|image| image.width.max(image.height) <= 4096));
    let counts = doc.nodes().fold([0; 3], |mut count, node| {
        match &node.kind {
            NodeKind::Text(text) if !text.characters.is_empty() => count[0] += 1,
            NodeKind::Vector { network } if !network.segments.is_empty() => count[1] += 1,
            NodeKind::Component { .. } => count[2] += 1,
            _ => {}
        }
        count
    });
    assert!(
        counts[0] >= 100 && counts[1] >= 180 && counts[2] >= 10,
        "{counts:?}"
    );
    println!(
        "{} | {counts:?} | exportación: {:?} | lectura: {warnings:?}",
        report.output_path.display(),
        report.summary.warnings
    );
}
