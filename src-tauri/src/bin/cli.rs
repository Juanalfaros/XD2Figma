//! Conversión por línea de comandos: `xd2figma-cli [opciones] <entrada.xd>... -o <carpeta>`.
use std::{path::PathBuf, process::ExitCode};
use xd2figma::{converter::Options, exporter};

const USO: &str = "Uso: xd2figma-cli [--zip] [--strict] [--extract-images] [--no-components]\n\
                   \x20                [--no-auto-layout] [--prototype] -o <carpeta> <archivo.xd>...";

fn main() -> ExitCode {
    let mut options = Options::default();
    let mut output: Option<PathBuf> = None;
    let mut inputs = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USO}");
                return ExitCode::SUCCESS;
            }
            "-o" | "--output" => match args.next() {
                Some(path) => output = Some(path.into()),
                None => return fail("Falta la carpeta después de -o"),
            },
            "--zip" => options.import_package = true,
            "--strict" => options.strict = true,
            "--extract-images" => options.extract_images = true,
            "--no-components" => options.keep_components = false,
            "--no-auto-layout" => options.repeat_grid_auto_layout = false,
            "--prototype" => options.preserve_prototype = true,
            flag if flag.starts_with('-') => return fail(&format!("Opción desconocida: {flag}")),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    let Some(output) = output else {
        return fail("Indica la carpeta de salida con -o");
    };
    if inputs.is_empty() {
        return fail("Indica al menos un archivo .xd");
    }
    let mut failed = false;
    for input in inputs {
        eprint!("{} … ", input.display());
        let mut last = 0;
        match exporter::convert_file(&input, &output, options, |event| {
            if event.level == "warning" {
                eprintln!("\n  aviso: {}", event.message);
            }
            last = event.percent;
        }) {
            Ok(report) => println!(
                "{} ({} artboards, {} capas, {} avisos)",
                report.output_path.display(),
                report.summary.artboards,
                report.summary.layers,
                report.summary.warnings.len()
            ),
            Err(error) => {
                failed = true;
                eprintln!("error ({last}%): {error}");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}\n{USO}");
    ExitCode::from(2)
}
