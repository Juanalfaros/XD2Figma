mod style;
mod svg;
mod vector;

use crate::parser::Document;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use style::Appearance;
use vector::VectorNetwork;

/// Mensaje con el que se señala una conversión cancelada por el usuario.
pub const CANCELLED: &str = "Conversión cancelada";
const MAX_NODES: usize = 100_000;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub keep_components: bool,
    pub preserve_prototype: bool,
    pub repeat_grid_auto_layout: bool,
    pub extract_images: bool,
    pub strict: bool,
    pub import_package: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            keep_components: true,
            preserve_prototype: false,
            repeat_grid_auto_layout: true,
            extract_images: false,
            strict: false,
            import_package: false,
        }
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub artboards: usize,
    pub layers: usize,
    pub warnings: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeType {
    Frame,
    Group,
    Rectangle,
    Text,
    Vector,
    Component,
    Instance,
    Ellipse,
    Line,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLayout {
    pub layout_mode: &'static str,
    pub layout_wrap: &'static str,
    pub columns: usize,
    pub item_spacing: f64,
    pub counter_axis_spacing: f64,
}

#[derive(Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: NodeType,
    pub visible: bool,
    pub transform: [f64; 6],
    pub children: Vec<Node>,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub geometry: Value,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub style: Value,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub component: Value,
    pub appearance: Appearance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_network: Option<VectorNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_layout: Option<AutoLayout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_svg: Option<String>,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub visual_bounds: Value,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub content_padding: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clip_shapes: Vec<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_axis: Option<bool>,
}

#[derive(Serialize)]
pub struct Board {
    pub node: Node,
    pub bounds: Value,
}

#[derive(Serialize)]
pub struct Design {
    pub schema_version: u32,
    pub strict: bool,
    pub name: String,
    pub artboards: Vec<Board>,
    pub pasteboard: Vec<Node>,
    pub components: Value,
    pub gradients: Value,
    pub prototype: Value,
    pub asset_ids: Vec<String>,
    pub summary: Summary,
}

pub(super) fn string(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

pub(super) fn number(value: &Value, default: f64) -> f64 {
    value.as_f64().unwrap_or(default)
}

fn index_symbols<'a>(value: &'a Value, index: &mut BTreeMap<&'a str, &'a Value>) {
    if let Some(values) = value.as_array() {
        for child in values {
            index_symbols(child, index);
        }
    } else if let Some(fields) = value.as_object() {
        if let Some(id) = value["id"].as_str().filter(|_| value["type"].is_string()) {
            index.insert(id, value);
        }
        for child in fields.values() {
            index_symbols(child, index);
        }
    }
}

struct Mapping<'a> {
    options: Options,
    symbols: BTreeMap<&'a str, &'a Value>,
    gradients: &'a Value,
    summary: Summary,
    notify: &'a mut dyn FnMut(&str, Option<&str>),
    cancelled: &'a dyn Fn() -> bool,
    visited: usize,
    fonts: BTreeSet<String>,
}

impl Mapping<'_> {
    fn warn(&mut self, element: &str, message: String) {
        if self.summary.warnings.insert(message.clone()) {
            (self.notify)(element, Some(&message));
        }
    }

    fn node(&mut self, source: &Value, reference_depth: u8) -> Result<Node, String> {
        self.visited += 1;
        if self.visited > MAX_NODES {
            return Err(format!(
                "El árbol expandido supera {MAX_NODES} nodos; divide el diseño en archivos más pequeños"
            ));
        }
        if self.visited.is_multiple_of(256) && (self.cancelled)() {
            return Err(CANCELLED.into());
        }
        let xd_type = string(&source["type"]);
        let name = source["name"].as_str().unwrap_or(xd_type);
        if xd_type == "syncRef" && reference_depth < 8 {
            if let Some(&original) = self.symbols.get(string(&source["syncSourceGuid"])) {
                let mut layer = self.node(original, reference_depth + 1)?;
                layer.id = string(&source["guid"]).to_owned();
                if self.options.keep_components {
                    layer.component = serde_json::json!({"source": source["syncSourceGuid"], "guid": source["guid"]});
                }
                return Ok(layer);
            }
        }
        let data = &source[xd_type];
        let ux = &source["meta"]["ux"];
        let shape = &source["shape"];
        let repeat = if xd_type == "repeatGrid" {
            data
        } else {
            &ux["repeatGrid"]
        };
        let mut unsupported = None;
        let mut kind = match xd_type {
            "artboard" => NodeType::Frame,
            "group" | "repeatGrid" => NodeType::Group,
            "text" => NodeType::Text,
            "shape" => match string(&shape["type"]) {
                "rect" => NodeType::Rectangle,
                "circle" | "ellipse" => NodeType::Ellipse,
                "line" => NodeType::Line,
                "path" | "compound" | "polygon" => NodeType::Vector,
                other => {
                    unsupported = Some(format!("Geometría XD sin equivalencia: {other}"));
                    NodeType::Vector
                }
            },
            other => {
                unsupported = Some(format!("Capa XD sin equivalencia: {other}"));
                NodeType::Vector
            }
        };
        let component = if self.options.keep_components && !ux["symbolId"].is_null() {
            kind = if ux["isMaster"].as_bool() == Some(true)
                || ux["componentType"].as_str() == Some("master")
                || self.symbols.contains_key(string(&source["id"]))
            {
                NodeType::Component
            } else {
                NodeType::Instance
            };
            serde_json::json!({"id":ux["symbolId"], "role":ux["componentType"], "overrides":ux["symbolOverrides"], "syncMap":ux["syncMap"]})
        } else {
            Value::Null
        };
        let children: Vec<_> = data["children"]
            .as_array()
            .or_else(|| source["group"]["children"].as_array())
            .or_else(|| source["children"].as_array())
            .into_iter()
            .flatten()
            .map(|child| self.node(child, reference_depth))
            .collect::<Result<_, _>>()?;
        let auto_layout = if self.options.repeat_grid_auto_layout
            && (!repeat.is_null() || xd_type == "repeatGrid")
        {
            let columns = repeat["columns"]
                .as_u64()
                .or_else(|| repeat["numColumns"].as_u64())
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(1)
                .clamp(1, children.len().max(1));
            let horizontal = columns > 1;
            kind = NodeType::Frame;
            let horizontal_gap = number(
                &repeat["horizontalPadding"],
                number(&repeat["columnGap"], 0.),
            )
            .max(0.);
            let vertical_gap =
                number(&repeat["verticalPadding"], number(&repeat["rowGap"], 0.)).max(0.);
            Some(AutoLayout {
                layout_mode: if horizontal { "HORIZONTAL" } else { "VERTICAL" },
                layout_wrap: if horizontal && children.len() > columns {
                    "WRAP"
                } else {
                    "NO_WRAP"
                },
                columns,
                item_spacing: if horizontal {
                    horizontal_gap
                } else {
                    vertical_gap
                },
                counter_axis_spacing: vertical_gap,
            })
        } else {
            None
        };
        let t = &source["transform"];
        let style = &source["style"];
        let appearance = Appearance::from_xd(style, self.gradients);
        if xd_type == "text" {
            if let Some(family) = style["font"]["family"].as_str() {
                if !self.fonts.contains(&family.to_lowercase()) {
                    self.warn(name, format!("Fuente {family} no encontrada localmente; Figma comprobará su disponibilidad al importar."));
                }
            }
        }
        let geometry = if xd_type == "text" {
            let mut text = data.clone();
            if let Some(ranges) = ux["rangedStyles"].as_array() {
                text["rangedStyles"] = Value::Array(ranges.clone());
            }
            text
        } else if !shape.is_null() {
            shape.clone()
        } else {
            Value::Null
        };
        let network = if kind == NodeType::Vector && unsupported.is_none() {
            let even_odd = string(&shape["fillRule"]).eq_ignore_ascii_case("evenodd");
            match VectorNetwork::parse(
                string(&shape["path"]),
                even_odd,
                !appearance.fills.is_empty(),
                !appearance.strokes.is_empty(),
            ) {
                Ok(network) => Some(network),
                Err("Contorno relleno sin cierre explícito") => None,
                Err(error) => {
                    unsupported = Some(error.into());
                    None
                }
            }
        } else {
            None
        };
        let mut reasons = appearance.unsupported.clone();
        reasons.extend(unsupported);
        let component = if component.is_object() {
            let mut component = component;
            component["width"] = ux["width"].clone();
            component["height"] = ux["height"].clone();
            component["stateId"] = ux["stateId"].clone();
            component
        } else {
            component
        };
        let clip_shapes = ux["clipPathResources"]["children"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|shape| self.node(shape, reference_depth))
            .collect::<Result<Vec<_>, _>>()?;
        let mut layer = Node {
            id: string(&source["id"]).to_owned(),
            name: name.to_owned(),
            kind,
            visible: source["visible"].as_bool().unwrap_or(true),
            transform: [
                number(&t["a"], 1.),
                number(&t["b"], 0.),
                number(&t["c"], 0.),
                number(&t["d"], 1.),
                number(&t["tx"], 0.),
                number(&t["ty"], 0.),
            ],
            children,
            clip_shapes,
            stack_axis: if self.options.repeat_grid_auto_layout {
                match ux["contentStackType"].as_str() {
                    Some("horizontal") => Some(false),
                    Some("vertical") => Some(true),
                    _ => None,
                }
            } else {
                None
            },
            geometry,
            style: style.clone(),
            component,
            appearance,
            vector_network: network,
            auto_layout,
            fallback_svg: None,
            visual_bounds: source["visualBounds"].clone(),
            content_padding: if ux["isContentAware"] == true && ux["hasContentBackground"] == true {
                ux["contentPadding"].clone()
            } else {
                Value::Null
            },
        };
        if !reasons.is_empty() {
            if self.options.strict {
                return Err(format!("{name}: {}", reasons.join(", ")));
            }
            let (svg, approximated) = svg::fallback(&layer, self.gradients);
            layer.fallback_svg = Some(svg);
            let detail = if approximated {
                "aproximación SVG"
            } else {
                "marcador SVG; faltan datos para reproducir el original"
            };
            self.warn(
                name,
                format!("{name} [{}]: {} · {detail}", layer.id, reasons.join(", ")),
            );
        }
        if xd_type != "artboard" {
            self.summary.layers += 1;
        }
        Ok(layer)
    }
}

pub fn convert(document: &Document, options: Options) -> Result<Design, String> {
    convert_with_progress(document, options, |_, _| {})
}

pub fn convert_with_progress(
    document: &Document,
    options: Options,
    notify: impl FnMut(&str, Option<&str>),
) -> Result<Design, String> {
    convert_cancellable(document, options, &|| false, notify)
}

/// Igual que `convert_with_progress`, pero consulta `cancelled` mientras recorre el árbol.
pub fn convert_cancellable(
    document: &Document,
    options: Options,
    cancelled: &dyn Fn() -> bool,
    mut notify: impl FnMut(&str, Option<&str>),
) -> Result<Design, String> {
    let mut artboards = Vec::new();
    let mut pasteboard = Vec::new();
    let resource = document.resources.as_ref();
    let resources = resource.map(|r| &r.resources).unwrap_or(&Value::Null);
    let definitions = &resources["meta"]["ux"]["symbols"];
    let mut symbols = BTreeMap::new();
    index_symbols(definitions, &mut symbols);
    let mut font_database = fontdb::Database::new();
    font_database.load_system_fonts();
    let fonts = font_database
        .faces()
        .flat_map(|face| face.families.iter())
        .map(|(family, _)| family.to_lowercase())
        .collect();
    let mut mapping = Mapping {
        options,
        symbols,
        gradients: &resources["gradients"],
        summary: Summary::default(),
        notify: &mut notify,
        cancelled,
        visited: 0,
        fonts,
    };
    for graphics in &document.boards {
        for source in &graphics.children {
            let is_board = source["type"] == "artboard";
            if is_board {
                let id = source["artboard"]["ref"]
                    .as_str()
                    .unwrap_or(string(&source["id"]));
                let name = resource
                    .and_then(|r| r.artboards.get(id))
                    .or_else(|| graphics.artboards.get(id))
                    .and_then(|b| b["name"].as_str())
                    .or_else(|| source["name"].as_str())
                    .unwrap_or(id);
                (mapping.notify)(name, None);
            }
            let mut layer = mapping.node(source, 0)?;
            if is_board {
                let id = source["artboard"]["ref"].as_str().unwrap_or(&layer.id);
                let bounds = resource
                    .and_then(|r| r.artboards.get(id))
                    .or_else(|| graphics.artboards.get(id))
                    .unwrap_or(&source["artboard"]);
                if number(&bounds["width"], 0.) <= 0. || number(&bounds["height"], 0.) <= 0. {
                    if options.strict {
                        return Err(format!("Mesa sin dimensiones válidas: {id}"));
                    }
                    mapping.warn(id, format!("Mesa sin dimensiones válidas: {id}; capas conservadas en el área de trabajo"));
                    pasteboard.push(layer);
                    continue;
                }
                if let Some(name) = bounds["name"].as_str() {
                    layer.name = name.to_owned();
                }
                artboards.push(Board {
                    node: layer,
                    bounds: bounds.clone(),
                });
            } else {
                pasteboard.push(layer);
            }
        }
    }
    let mut summary = mapping.summary;
    summary.artboards = artboards.len();
    let components = if options.keep_components {
        definitions.clone()
    } else {
        Value::Null
    };
    if !components.is_null() {
        summary.warnings.insert("Las definiciones de componentes se conservan en JSON; algunas instancias se reconstruyen como marcos editables.".into());
    }
    let prototype = if options.preserve_prototype {
        document.interactions.clone().unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    if !prototype.is_null() {
        summary
            .warnings
            .insert("Las interacciones se conservan en JSON y deben recrearse en Figma.".into());
    }
    Ok(Design {
        schema_version: 2,
        strict: options.strict,
        name: document.manifest["name"]
            .as_str()
            .unwrap_or("Adobe XD")
            .to_owned(),
        artboards,
        pasteboard,
        components,
        gradients: resources["gradients"].clone(),
        prototype,
        asset_ids: document.assets.keys().cloned().collect(),
        summary,
    })
}
