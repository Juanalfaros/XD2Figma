use crate::{
    converter::{Design, Node, NodeType},
    parser::Document as Xd,
};
use figcraft_doc::{
    color::{Color, ColorStop, Gradient, GradientKind, ImageScaleMode, Paint, PaintKind},
    geom::Affine,
    *,
};
use image::{ImageFormat, ImageReader};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
};

pub fn export(design: &Design, xd: &Xd) -> Result<(Vec<u8>, Vec<String>), String> {
    let doc = Document::new(design.name.replace(|c: char| c.is_control(), ""));
    let page = doc.pages[0];
    let mut state = Mapping {
        doc,
        xd,
        images: BTreeMap::new(),
        warnings: BTreeSet::new(),
        strict: design.strict,
    };
    for board in &design.artboards {
        state.insert(&board.node, page, Some(&board.bounds), (0., 0.))?;
    }
    for node in &design.pasteboard {
        state.insert(node, page, None, (0., 0.))?;
    }
    let (bytes, warnings) = figcraft_format::fig::export(&state.doc, xd.thumbnail().as_deref())
        .map_err(|e| e.to_string())?;
    state.warnings.extend(warnings);
    Ok((bytes, state.warnings.into_iter().collect()))
}

struct Mapping<'a> {
    doc: Document,
    xd: &'a Xd,
    images: BTreeMap<String, Option<String>>,
    warnings: BTreeSet<String>,
    strict: bool,
}

fn num(value: &Value, fallback: f64) -> f64 {
    value.as_f64().unwrap_or(fallback)
}

fn color(value: &Value) -> Color {
    Color::new(
        num(&value["r"], 0.) as f32,
        num(&value["g"], 0.) as f32,
        num(&value["b"], 0.) as f32,
        num(&value["a"], 1.) as f32,
    )
}

impl Mapping<'_> {
    fn paints(&mut self, paints: &[Value], width: f64, height: f64) -> Result<Vec<Paint>, String> {
        let mut result = Vec::new();
        for paint in paints {
            let mut mapped = match paint["type"].as_str() {
                Some("SOLID") => Paint::solid(color(&paint["color"])),
                Some("IMAGE") => {
                    let id = paint["assetId"].as_str().unwrap_or_default();
                    if !self.images.contains_key(id) {
                        let loaded = (|| -> Result<String, String> {
                            let file = self.xd.open_asset(id)?;
                            figcraft_format::add_image(&mut self.doc, compatible_image(file)?)
                                .map(|image| image.0)
                                .map_err(|e| e.to_string())
                        })();
                        let hash = match loaded {
                            Ok(hash) => Some(hash),
                            Err(error) if self.strict => {
                                return Err(format!("Imagen {id}: {error}"))
                            }
                            Err(error) => {
                                self.warnings.insert(format!("Imagen {id}: {error}"));
                                None
                            }
                        };
                        self.images.insert(id.to_owned(), hash);
                    }
                    let Some(hash) = &self.images[id] else {
                        continue;
                    };
                    let mut image_paint = Paint::image(hash, ImageScaleMode::Fill);
                    if let PaintKind::Image(image) = &mut image_paint.kind {
                        let source = &self.doc.images[hash];
                        if let Some(crop) =
                            image_crop(paint, width, height, source.width, source.height)
                        {
                            image.scale_mode = ImageScaleMode::Crop;
                            image.crop = Some(crop);
                        }
                    }
                    image_paint
                }
                Some(kind @ ("GRADIENT_LINEAR" | "GRADIENT_RADIAL")) => {
                    let transform = &paint["gradientTransform"];
                    let matrix = Affine::new([
                        num(&transform[0][0], 1.),
                        num(&transform[1][0], 0.),
                        num(&transform[0][1], 0.),
                        num(&transform[1][1], 1.),
                        num(&transform[0][2], 0.),
                        num(&transform[1][2], 0.),
                    ])
                    .inverse();
                    let radial = kind == "GRADIENT_RADIAL";
                    let handle = |x, y| {
                        let p = matrix * figcraft_doc::geom::Point::new(x, y);
                        (p.x, p.y)
                    };
                    Paint::gradient(Gradient {
                        kind: if radial {
                            GradientKind::Radial
                        } else {
                            GradientKind::Linear
                        },
                        from: handle(if radial { 0.5 } else { 0. }, 0.5),
                        to: handle(1., 0.5),
                        width: handle(if radial { 0.5 } else { 0. }, 1.),
                        stops: paint["gradientStops"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|stop| ColorStop {
                                position: num(&stop["position"], 0.) as f32,
                                color: color(&stop["color"]),
                            })
                            .collect(),
                    })
                }
                _ => continue,
            };
            mapped.opacity *= num(&paint["opacity"], 1.) as f32;
            result.push(mapped);
        }
        Ok(result)
    }

    fn insert(
        &mut self,
        src: &Node,
        parent: NodeId,
        board: Option<&Value>,
        origin: (f64, f64),
    ) -> Result<NodeId, String> {
        let g = &src.geometry;
        let (mut x, mut y) = (num(&g["x"], 0.), num(&g["y"], 0.));
        let (mut w, mut h) = (
            num(&g["width"], num(&src.visual_bounds["width"], 1.)),
            num(&g["height"], num(&src.visual_bounds["height"], 1.)),
        );
        let mut angle = 0.;
        let kind = match src.kind {
            NodeType::Frame => NodeKind::Frame,
            NodeType::Group => NodeKind::Group,
            NodeType::Component => {
                w = num(&src.component["width"], w);
                h = num(&src.component["height"], h);
                NodeKind::Component {
                    description: String::new(),
                    props: BTreeMap::new(),
                    prop_order: Vec::new(),
                }
            }
            NodeType::Instance => {
                self.warnings.insert(format!(
                    "{}: instancia convertida a marco editable para conservar sus capas.",
                    src.name
                ));
                w = num(&src.component["width"], w);
                h = num(&src.component["height"], h);
                NodeKind::Frame
            }
            NodeType::Rectangle => NodeKind::Rectangle,
            NodeType::Ellipse => {
                let rx = num(&g["rx"], num(&g["r"], 0.));
                let ry = num(&g["ry"], num(&g["r"], 0.));
                (x, y, w, h) = (
                    num(&g["cx"], 0.) - rx,
                    num(&g["cy"], 0.) - ry,
                    rx * 2.,
                    ry * 2.,
                );
                NodeKind::Ellipse {
                    start: 0.,
                    sweep: std::f64::consts::TAU,
                    inner: 0.,
                }
            }
            NodeType::Line => {
                (x, y) = (num(&g["x1"], 0.), num(&g["y1"], 0.));
                let (dx, dy) = (num(&g["x2"], 0.) - x, num(&g["y2"], 0.) - y);
                (w, h, angle) = (dx.hypot(dy), 0., dy.atan2(dx));
                NodeKind::Line
            }
            NodeType::Text => {
                let mut text = text_data(src);
                let first = &g["paragraphs"][0]["lines"][0][0];
                let line_height = text.style.line_height.resolve(text.style.font_size, 1.2);
                let last_baseline = g["paragraphs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|p| p["lines"].as_array().into_iter().flatten())
                    .filter_map(|line| line[0]["y"].as_f64())
                    .fold(num(&first["y"], 0.), f64::max);
                if h <= 1. {
                    h = line_height.max(text.style.font_size) + last_baseline
                        - num(&first["y"], 0.);
                }
                (x, y) = (
                    num(&first["x"], 0.),
                    num(&first["y"], 0.) - text.style.font_size * 0.8,
                );
                if let Some(width) = g["frame"]["width"].as_f64() {
                    w = width;
                    x = match text.align_h {
                        TextAlignH::Center => -w / 2.,
                        TextAlignH::Right => -w,
                        _ => 0.,
                    };
                }
                if !g["frame"]["width"].is_number()
                    && matches!(text.align_h, TextAlignH::Center | TextAlignH::Right)
                {
                    let left = g["paragraphs"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .flat_map(|p| p["lines"].as_array().into_iter().flatten())
                        .filter_map(|line| line[0]["x"].as_f64())
                        .fold(0., f64::min);
                    if left < 0. {
                        let centered = text.align_h == TextAlignH::Center;
                        w = (-left * if centered { 2. } else { 1. }).ceil() + 2.;
                        x = -w / if centered { 2. } else { 1. };
                        text.auto_resize = TextAutoResize::Height;
                    }
                }
                NodeKind::Text(text)
            }
            NodeType::Vector => {
                if let Some(network) = &src.vector_network {
                    let mut network = network.to_figcraft();
                    if let Some(bounds) = network.bounds() {
                        (x, y, w, h) = (bounds.x0, bounds.y0, bounds.width(), bounds.height());
                        network.translate(-x, -y);
                    }
                    NodeKind::Vector { network }
                } else {
                    self.warnings.insert(format!(
                        "{}: geometría no representable como red; se guardó un marco editable.",
                        src.name
                    ));
                    NodeKind::Frame
                }
            }
        };
        let mut transform =
            Affine::new(src.transform) * Affine::translate((x, y)) * Affine::rotate(angle);
        if let Some(bounds) = board {
            w = num(&bounds["width"], w);
            h = num(&bounds["height"], h);
            transform = Affine::translate((num(&bounds["x"], 0.), num(&bounds["y"], 0.)));
        } else {
            transform = Affine::translate((-origin.0, -origin.1)) * transform;
        }
        let id = self.doc.alloc_id();
        let mut node = figcraft_doc::Node::new(
            id,
            kind,
            &src.name,
            w.max(0.01),
            if src.kind == NodeType::Line {
                0.
            } else {
                h.max(0.01)
            },
        );
        node.transform = transform;
        node.visible = src.visible;
        node.opacity = src.appearance.opacity as f32;
        if let Some(blend) = src.style["blendMode"]
            .as_str()
            .and_then(figcraft_doc::color::BlendMode::from_name)
        {
            node.blend = blend;
        }
        node.clip = board.is_some();
        node.fills = self.paints(&src.appearance.fills, node.width, node.height)?;
        if src.kind == NodeType::Text && node.fills.is_empty() && src.style["fill"].is_null() {
            node.fills.push(Paint::solid(Color::BLACK));
        }
        node.strokes = self.paints(&src.appearance.strokes, node.width, node.height)?;
        node.stroke.weight = src.appearance.stroke_weight;
        node.stroke.align = match src.appearance.stroke_align {
            "INSIDE" => StrokeAlign::Inside,
            "OUTSIDE" => StrokeAlign::Outside,
            _ => StrokeAlign::Center,
        };
        node.effects = src.appearance.effects.iter().filter_map(effect).collect();
        if let Some(radii) = g["r"].as_array().filter(|r| r.len() == 4) {
            node.corners.radii = std::array::from_fn(|i| num(&radii[i], 0.).max(0.));
        } else if src.kind == NodeType::Rectangle {
            node.corners.radii = [num(&g["r"], 0.).max(0.); 4];
        }
        if let Some(layout) = &src.auto_layout {
            node.layout = Some(AutoLayout {
                mode: if layout.layout_mode == "VERTICAL" {
                    LayoutMode::Vertical
                } else {
                    LayoutMode::Horizontal
                },
                wrap: layout.layout_wrap == "WRAP",
                gap: layout.item_spacing,
                cross_gap: layout.counter_axis_spacing,
                padding: [0.; 4],
                ..AutoLayout::default()
            });
        }
        self.doc
            .insert(node, parent, None)
            .map_err(|e| e.to_string())?;
        let child_origin = board
            .map(|b| (num(&b["x"], 0.), num(&b["y"], 0.)))
            .unwrap_or_default();
        // XD compound shapes already have the final Boolean geometry in their vector network.
        if matches!(
            src.kind,
            NodeType::Frame | NodeType::Group | NodeType::Component | NodeType::Instance
        ) || (src.kind == NodeType::Vector && src.vector_network.is_none())
        {
            for shape in &src.clip_shapes {
                let mask_id = self.insert(shape, id, None, child_origin)?;
                let mask = self.doc.get_mut(mask_id).map_err(|e| e.to_string())?;
                mask.is_mask = true;
                mask.mask_type = MaskType::Vector;
            }
            let label_box = button_label_box(src);
            for child in &src.children {
                let child_id = self.insert(child, id, None, child_origin)?;
                if child.kind == NodeType::Text {
                    if let Some([x, y, width, height]) = label_box {
                        let label = self.doc.get_mut(child_id).map_err(|e| e.to_string())?;
                        label.transform =
                            Affine::translate((x - child_origin.0, y - child_origin.1));
                        label.width = width;
                        label.height = height;
                        if let NodeKind::Text(text) = &mut label.kind {
                            text.auto_resize = TextAutoResize::None;
                            text.align_h = TextAlignH::Center;
                            text.align_v = TextAlignV::Middle;
                        }
                    }
                }
            }
        }
        if src.kind == NodeType::Group && !src.children.is_empty() {
            self.doc.fit_to_children(id).map_err(|e| e.to_string())?;
            if let Some(vertical) = src
                .stack_axis
                .filter(|_| src.clip_shapes.is_empty() && src.content_padding.is_null())
            {
                self.apply_stack(id, vertical)?;
            }
        }
        Ok(id)
    }

    fn apply_stack(&mut self, id: NodeId, vertical: bool) -> Result<(), String> {
        let group = self.doc.get(id).map_err(|e| e.to_string())?;
        let mut children = Vec::new();
        for &child in &group.children {
            let node = self.doc.get(child).map_err(|e| e.to_string())?;
            let [a, b, c, d, x, y] = node.transform.as_coeffs();
            if !node.visible
                || matches!(node.kind, NodeKind::Text(_))
                || [a, b, c, d] != [1., 0., 0., 1.]
            {
                return Ok(());
            }
            let (start, length, cross) = if vertical {
                (y, node.height, x)
            } else {
                (x, node.width, y)
            };
            if cross.abs() > 0.1 || length <= 0. {
                return Ok(());
            }
            children.push((child, start, length));
        }
        if children.len() < 2 {
            return Ok(());
        }
        children.sort_by(|a, b| a.1.total_cmp(&b.1));
        let gap = children[1].1 - children[0].1 - children[0].2;
        if gap < 0.
            || children
                .windows(2)
                .any(|pair| (pair[1].1 - pair[0].1 - pair[0].2 - gap).abs() > 0.1)
        {
            return Ok(());
        }
        let group = self.doc.get_mut(id).map_err(|e| e.to_string())?;
        group.kind = NodeKind::Frame;
        group.children = children.into_iter().map(|(child, _, _)| child).collect();
        group.layout = Some(AutoLayout {
            mode: if vertical {
                LayoutMode::Vertical
            } else {
                LayoutMode::Horizontal
            },
            gap,
            padding: [0.; 4],
            strokes_included: false,
            ..AutoLayout::default()
        });
        Ok(())
    }
}

fn button_label_box(node: &Node) -> Option<[f64; 4]> {
    let padding = &node.content_padding;
    let [background, label] = node.children.as_slice() else {
        return None;
    };
    if background.kind != NodeType::Rectangle
        || label.kind != NodeType::Text
        || !background.visible
        || !label.visible
        || background.transform[..4] != [1., 0., 0., 1.]
        || label.transform[..4] != [1., 0., 0., 1.]
        || label.geometry["textPath"].is_object()
        || label.geometry["paragraphs"].as_array()?.len() != 1
        || label.geometry["paragraphs"][0]["lines"].as_array()?.len() != 1
    {
        return None;
    }
    let left = padding["left"].as_f64()?.max(0.);
    let right = padding["right"].as_f64()?.max(0.);
    let top = padding["top"].as_f64()?.max(0.);
    let bottom = padding["bottom"].as_f64()?.max(0.);
    let shape = &background.geometry;
    let width = shape["width"].as_f64()? - left - right;
    let height = shape["height"].as_f64()? - top - bottom;
    (width > 0. && height > 0.).then_some([
        background.transform[4] + num(&shape["x"], 0.) + left,
        background.transform[5] + num(&shape["y"], 0.) + top,
        width,
        height,
    ])
}

fn text_data(src: &Node) -> TextData {
    let mut text = TextData::new(src.geometry["rawText"].as_str().unwrap_or_default());
    let style = &src.style;
    let font = &style["font"];
    let attrs = &style["textAttributes"];
    text.style.font_family = font["family"].as_str().unwrap_or("Inter").into();
    text.style.font_style = font["style"].as_str().unwrap_or("Regular").into();
    text.style.font_size = num(&font["size"], 16.);
    if let Some(height) = attrs["lineHeight"].as_f64().filter(|h| *h > 0.) {
        text.style.line_height = LineHeight::Pixels(height);
    }
    text.style.letter_spacing =
        LetterSpacing::Percent(num(&src.appearance.letter_spacing["value"], 0.));
    text.auto_resize = if src.geometry["frame"]["width"].is_number() {
        TextAutoResize::Height
    } else {
        TextAutoResize::WidthAndHeight
    };
    text.align_h = match attrs["paragraphAlign"]
        .as_str()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "center" => TextAlignH::Center,
        "right" => TextAlignH::Right,
        "justify" | "justified" => TextAlignH::Justified,
        _ => TextAlignH::Left,
    };
    let mut offsets = vec![0];
    for character in text.characters.chars() {
        offsets.push(offsets.last().copied().unwrap_or(0) + character.len_utf16());
    }
    let ranges = src.geometry["rangedStyles"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    // XD may retain the previous character count after editing uniformly styled text.
    let uniform = ranges.len() == 1;
    let mut offset = 0usize;
    for range in ranges {
        let Some(length) = range["length"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
        else {
            continue;
        };
        let from = offset;
        offset = if uniform {
            offsets.last().copied().unwrap_or(0)
        } else {
            offset.saturating_add(length)
        };
        let (Ok(start), Ok(end)) = (offsets.binary_search(&from), offsets.binary_search(&offset))
        else {
            continue;
        };
        if start >= end {
            continue;
        }
        let mut style = text.style.clone();
        if let Some(family) = range["fontFamily"]
            .as_str()
            .filter(|family| !family.is_empty())
        {
            style.font_family = family.into();
        }
        if let Some(weight) = range["fontStyle"]
            .as_str()
            .filter(|weight| !weight.is_empty())
        {
            style.font_style = weight.into();
        }
        style.font_size = num(&range["fontSize"], style.font_size);
        if let Some(spacing) = range["charSpacing"].as_f64() {
            style.letter_spacing = LetterSpacing::Percent(spacing / 10.);
        }
        if let Some(argb) = range["fill"]["value"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
        {
            let component = |shift: u32| ((argb >> shift) & 255) as f32 / 255.;
            style.fills = Some(vec![Paint::solid(Color::new(
                component(16),
                component(8),
                component(0),
                component(24),
            ))]);
        }
        style.case = match range["textTransform"].as_str() {
            Some("uppercase") => TextCase::Upper,
            Some("lowercase") => TextCase::Lower,
            _ => style.case,
        };
        if range["underline"] == true {
            style.decoration = TextDecoration::Underline;
        }
        if range["strikethrough"] == true {
            style.decoration = TextDecoration::Strikethrough;
        }
        if uniform {
            text.style = style;
        } else if style != text.style {
            text.runs.push(StyleRun { start, end, style });
        }
    }
    text
}

fn effect(value: &Value) -> Option<Effect> {
    let radius = num(&value["radius"], 0.);
    let kind = match value["type"].as_str()? {
        kind @ ("DROP_SHADOW" | "INNER_SHADOW") => {
            let shadow = Shadow {
                color: color(&value["color"]),
                offset: (
                    num(&value["offset"]["x"], 0.),
                    num(&value["offset"]["y"], 0.),
                ),
                radius,
                spread: num(&value["spread"], 0.),
                ..Shadow::default()
            };
            if kind == "DROP_SHADOW" {
                EffectKind::DropShadow(shadow)
            } else {
                EffectKind::InnerShadow(shadow)
            }
        }
        "LAYER_BLUR" => EffectKind::LayerBlur(Blur {
            radius,
            ..Blur::default()
        }),
        "BACKGROUND_BLUR" => EffectKind::BackgroundBlur(Blur {
            radius,
            ..Blur::default()
        }),
        _ => return None,
    };
    Some(Effect {
        kind,
        visible: value["visible"].as_bool().unwrap_or(true),
    })
}

fn image_crop(
    paint: &Value,
    width: f64,
    height: f64,
    pixel_width: u32,
    pixel_height: u32,
) -> Option<[f64; 6]> {
    let zoom = num(&paint["xdScale"], 1.);
    let offset_x = num(&paint["xdOffsetX"], 0.);
    let offset_y = num(&paint["xdOffsetY"], 0.);
    if zoom <= 0. || (zoom == 1. && offset_x.abs() < 1e-9 && offset_y.abs() < 1e-9) {
        return None;
    }
    let iw = num(&paint["sourceWidth"], f64::from(pixel_width));
    let ih = num(&paint["sourceHeight"], f64::from(pixel_height));
    if iw <= 0. || ih <= 0. {
        return None;
    }
    // XD's offsets are fractions of the drawn image. Figma stores the inverse
    // mapping, from the shape's unit box to coordinates within the image.
    let primary_offset = offset_x.abs().max(offset_y.abs());
    let secondary_offset = offset_x.abs().min(offset_y.abs());
    let split =
        zoom < 1. && secondary_offset < 0.005 && (primary_offset - (1. - zoom) / 2.).abs() < 0.005;
    let (u, v) = if split {
        // Split crops keep the whole perpendicular axis and select a fraction
        // of the bitmap along the axis on which XD positioned the cut.
        if offset_x.abs() > offset_y.abs() {
            (zoom, 1.)
        } else {
            (1., zoom)
        }
    } else {
        let cover = (width / iw).max(height / ih) * zoom.max(1. / zoom);
        (width / (iw * cover), height / (ih * cover))
    };
    let crop = [
        u,
        0.,
        0.,
        v,
        (1. - u) / 2. - offset_x,
        (1. - v) / 2. - offset_y,
    ];
    crop.iter().all(|value| value.is_finite()).then_some(crop)
}

fn compatible_image(file: File) -> Result<Vec<u8>, String> {
    const MAX_DIMENSION: u32 = 4096;
    const MAX_ENCODED: u64 = 64 * 1024 * 1024;
    let mut file = BufReader::new(file);
    let probe = ImageReader::new(&mut file)
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let format = probe.format();
    let (width, height) = probe.into_dimensions().map_err(|e| e.to_string())?;
    file.rewind().map_err(|e| e.to_string())?;
    if width.max(height) <= MAX_DIMENSION
        && matches!(format, Some(ImageFormat::Png | ImageFormat::Jpeg))
        && file.get_ref().metadata().map_err(|e| e.to_string())?.len() <= MAX_ENCODED
    {
        let mut bytes = Vec::new();
        file.take(MAX_ENCODED + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_ENCODED {
            return Err("El recurso supera el límite de lectura".into());
        }
        return Ok(bytes);
    }
    let image = ImageReader::new(file)
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?
        .thumbnail(MAX_DIMENSION, MAX_DIMENSION);
    let format = if format == Some(ImageFormat::Jpeg) {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Png
    };
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, format)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::image_crop;
    use serde_json::json;

    #[test]
    fn preserves_top_aligned_and_split_image_crops() {
        let paint = json!({
            "sourceWidth": 1920, "sourceHeight": 5320,
            "xdScale": 1, "xdOffsetY": 0.3824612498283386
        });
        let [u, _, _, v, x, y] = image_crop(&paint, 1176., 766., 1478, 4096).unwrap();
        assert!((u - 1.).abs() < 1e-6);
        assert!((v - 766. / (5320. * 1176. / 1920.)).abs() < 1e-6);
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6);

        let mut split = paint;
        split["xdScale"] = json!(0.49);
        split["xdOffsetY"] = json!(-0.255);
        let [u, _, _, v, _, y] = image_crop(&split, 595., 3433., 1478, 4096).unwrap();
        assert_eq!(u, 1.);
        assert!((v - 0.49).abs() < 1e-6);
        assert!((y + v - 1.).abs() < 1e-6);
    }
}
