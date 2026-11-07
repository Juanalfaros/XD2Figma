use super::{number, string, Node};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    io::{self, Write},
};

fn xml(text: &str) -> String {
    text.chars()
        .filter(|&c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn color(value: &Value) -> String {
    let rgb = &value["value"];
    format!(
        "rgb({},{},{})",
        number(&rgb["r"], 0.),
        number(&rgb["g"], 0.),
        number(&rgb["b"], 0.)
    )
}

fn paint(
    out: &mut impl Write,
    value: &Value,
    gradients: &Value,
    id: usize,
    warnings: &mut BTreeSet<String>,
) -> io::Result<String> {
    match string(&value["type"]) {
        "solid" => Ok(color(&value["color"])),
        "" | "none" | "pattern" => Ok("none".into()),
        "gradient" => {
            let inline = &value["gradient"];
            let reference = inline["ref"].as_str().or_else(|| inline["href"].as_str());
            let gradient = reference
                .map(|key| &gradients[key.trim_start_matches('#')])
                .unwrap_or(inline);
            let Some(stops) = gradient["stops"].as_array() else {
                warnings.insert("Algunos degradados XD requieren ajustes manuales.".into());
                return Ok("none".into());
            };
            let start = &gradient["start"];
            let end = &gradient["end"];
            let x = number(&start["x"], 0.);
            let y = number(&start["y"], 0.);
            let ex = number(&end["x"], 1.);
            let ey = number(&end["y"], 0.);
            let tag = if gradient["type"].as_str() == Some("radial") {
                write!(
                    out,
                    "<defs><radialGradient id=\"paint{id}\" cx=\"{x}\" cy=\"{y}\" r=\"{}\">",
                    (ex - x).hypot(ey - y)
                )?;
                "radialGradient"
            } else {
                write!(out, "<defs><linearGradient id=\"paint{id}\" x1=\"{x}\" y1=\"{y}\" x2=\"{ex}\" y2=\"{ey}\">")?;
                "linearGradient"
            };
            for stop in stops {
                write!(
                    out,
                    "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
                    number(&stop["offset"], 0.),
                    color(&stop["color"]),
                    number(&stop["color"]["alpha"], 1.)
                )?;
            }
            write!(out, "</{tag}></defs>")?;
            Ok(format!("url(#paint{id})"))
        }
        other => {
            warnings.insert(format!("Relleno o trazo SVG sin equivalencia: {other}"));
            Ok("none".into())
        }
    }
}

fn geometry(node: &Node) -> Option<(&'static str, String)> {
    let g = &node.geometry;
    let n = |key: &str| number(&g[key], 0.);
    let radius = g["r"].get(0).or_else(|| g["r"].as_f64().map(|_| &g["r"]));
    match string(&g["type"]) {
        "rect" => Some((
            "rect",
            format!(
                "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"",
                n("x"),
                n("y"),
                n("width"),
                n("height"),
                radius.map(|r| number(r, 0.)).unwrap_or(0.)
            ),
        )),
        "circle" => Some((
            "circle",
            format!("cx=\"{}\" cy=\"{}\" r=\"{}\"", n("cx"), n("cy"), n("r")),
        )),
        "ellipse" => Some((
            "ellipse",
            format!(
                "cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"",
                n("cx"),
                n("cy"),
                n("rx"),
                n("ry")
            ),
        )),
        "line" => Some((
            "line",
            format!(
                "x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                n("x1"),
                n("y1"),
                n("x2"),
                n("y2")
            ),
        )),
        "path" | "compound" | "polygon" if g["path"].is_string() => {
            Some(("path", format!("d=\"{}\"", xml(string(&g["path"])))))
        }
        _ => None,
    }
}

pub(super) fn fallback(node: &Node, gradients: &Value) -> (String, bool) {
    let mut content = Vec::new();
    let mut warnings = BTreeSet::new();
    let fill = paint(
        &mut content,
        &node.style["fill"],
        gradients,
        0,
        &mut warnings,
    )
    .unwrap_or_else(|_| "none".into());
    let stroke = paint(
        &mut content,
        &node.style["stroke"],
        gradients,
        1,
        &mut warnings,
    )
    .unwrap_or_else(|_| "none".into());
    let g = &node.geometry;
    let bounds = &node.visual_bounds;
    let rx = number(&g["rx"], number(&g["r"], 0.));
    let ry = number(&g["ry"], number(&g["r"], 0.));
    let x = number(&g["x"], number(&g["cx"], rx) - rx);
    let y = number(&g["y"], number(&g["cy"], ry) - ry);
    let w = number(&g["width"], number(&bounds["width"], (rx * 2.).max(32.))).max(1.);
    let h = number(&g["height"], number(&bounds["height"], (ry * 2.).max(24.))).max(1.);
    let approximated = if let Some((tag, attrs)) = geometry(node) {
        let fill = if fill == "none" && stroke == "none" {
            "#ae76de"
        } else {
            &fill
        };
        let _ = write!(
            content,
            "<{tag} {attrs} fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{}\"/>",
            node.appearance.stroke_weight
        );
        true
    } else if let Some(raw) = g["rawText"].as_str() {
        let size = number(&node.style["font"]["size"], 16.);
        let _ = write!(
            content,
            "<text x=\"0\" y=\"{size}\" font-size=\"{size}\">{}</text>",
            xml(raw)
        );
        true
    } else {
        let _ = write!(content, "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#ae76de\" fill-opacity=\".18\" stroke=\"#ae76de\" stroke-dasharray=\"3 2\"/>");
        false
    };
    (format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{x} {y} {w} {h}\"><title>{}</title>{}</svg>", xml(&node.name), String::from_utf8_lossy(&content)), approximated)
}
