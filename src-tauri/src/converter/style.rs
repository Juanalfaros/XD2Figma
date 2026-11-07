use super::{number, string};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub fills: Vec<Value>,
    pub strokes: Vec<Value>,
    pub stroke_weight: f64,
    pub stroke_align: &'static str,
    pub effects: Vec<Value>,
    pub opacity: f64,
    pub letter_spacing: Value,
    pub unsupported: Vec<String>,
}

fn rgb(color: &Value) -> Value {
    let value = &color["value"];
    json!({"r":number(&value["r"],0.)/255., "g":number(&value["g"],0.)/255., "b":number(&value["b"],0.)/255.})
}

fn rgba(color: &Value) -> Value {
    let mut rgb = rgb(color);
    rgb["a"] = json!(number(&color["alpha"], 1.).clamp(0., 1.));
    rgb
}

fn paint(value: &Value, gradients: &Value) -> Result<Option<Value>, String> {
    Ok(match string(&value["type"]) {
        "solid" => Some(
            json!({"type":"SOLID","color":rgb(&value["color"]),"opacity":number(&value["color"]["alpha"],1.).clamp(0.,1.)}),
        ),
        "" | "none" => None,
        "pattern" => {
            let pattern = &value["pattern"];
            let id = pattern["meta"]["ux"]["uid"]
                .as_str()
                .unwrap_or(string(&pattern["href"]))
                .trim_start_matches("/resources/");
            if id.is_empty() {
                return Err("Patrón sin recurso".into());
            }
            let ux = &pattern["meta"]["ux"];
            Some(json!({
                "type":"IMAGE", "assetId":id, "scaleMode":"FILL",
                "sourceWidth":pattern["width"], "sourceHeight":pattern["height"],
                "xdScale":ux["scale"], "xdOffsetX":ux["offsetX"], "xdOffsetY":ux["offsetY"]
            }))
        }
        "gradient" => {
            let inline = &value["gradient"];
            let gradient = inline["ref"]
                .as_str()
                .or_else(|| inline["href"].as_str())
                .map(|key| &gradients[key.trim_start_matches('#')])
                .unwrap_or(inline);
            let resources = &gradient["meta"]["ux"]["gradientResources"];
            let stops = gradient["stops"]
                .as_array()
                .or_else(|| resources["stops"].as_array())
                .filter(|s| s.len() >= 2)
                .ok_or("Degradado sin paradas válidas")?;
            let start = &gradient["start"];
            let end = &gradient["end"];
            let x = number(&start["x"], number(&gradient["x1"], 0.));
            let y = number(&start["y"], number(&gradient["y1"], 0.));
            let dx = number(&end["x"], number(&gradient["x2"], 1.)) - x;
            let dy = number(&end["y"], number(&gradient["y2"], 0.)) - y;
            let length = dx * dx + dy * dy;
            if !length.is_finite() || length <= f64::EPSILON {
                return Err("Degradado sin dirección".into());
            }
            let kind = if gradient["type"]
                .as_str()
                .or_else(|| resources["type"].as_str())
                == Some("radial")
            {
                "GRADIENT_RADIAL"
            } else {
                "GRADIENT_LINEAR"
            };
            let center = if kind == "GRADIENT_RADIAL" { 0.5 } else { 0. };
            let stops: Vec<_> = stops.iter().map(|s| json!({"position":number(&s["offset"],0.).clamp(0.,1.),"color":rgba(&s["color"])})).collect();
            Some(json!({"type":kind, "gradientStops":stops,
                "gradientTransform":[[dx/length,dy/length,center-(dx*x+dy*y)/length],[-dy/length,dx/length,0.5+(dy*x-dx*y)/length]]}))
        }
        other => return Err(format!("Pintura XD sin equivalencia: {other}")),
    })
}

impl Appearance {
    pub fn from_xd(style: &Value, gradients: &Value) -> Self {
        let stroke = &style["stroke"];
        let mut appearance = Self {
            stroke_weight: number(&stroke["width"], 1.).max(0.),
            stroke_align: match string(&stroke["align"]) {
                "inside" => "INSIDE",
                "outside" => "OUTSIDE",
                _ => "CENTER",
            },
            opacity: number(&style["opacity"], 1.).clamp(0., 1.),
            ..Self::default()
        };
        let attrs = &style["textAttributes"];
        if let Some(spacing) = attrs["charSpacing"]
            .as_f64()
            .or_else(|| attrs["letterSpacing"].as_f64())
            .or_else(|| style["charSpacing"].as_f64())
        {
            appearance.letter_spacing = json!({"unit":"PERCENT","value":spacing/10.});
        }
        for (value, target) in [
            (&style["fill"], &mut appearance.fills),
            (stroke, &mut appearance.strokes),
        ] {
            match paint(value, gradients) {
                Ok(Some(paint)) => target.push(paint),
                Ok(None) => (),
                Err(error) => appearance.unsupported.push(error),
            }
        }
        for effect in style["filters"].as_array().into_iter().flatten() {
            let params = &effect["params"];
            if effect["type"] == "uxdesign#blur"
                && params["backgroundEffect"] == true
                && effect["visible"] != false
            {
                if let Some(opacity) = params["fillOpacity"].as_f64() {
                    for fill in &mut appearance.fills {
                        fill["opacity"] =
                            json!(number(&fill["opacity"], 1.) * opacity.clamp(0., 1.));
                    }
                }
            }
            match string(&effect["type"]) {
                "dropShadow" | "innerShadow" => {
                    let inner = effect["type"] == "innerShadow";
                    let key = if inner { "innerShadows" } else { "dropShadows" };
                    for shadow in params[key].as_array().into_iter().flatten() {
                        appearance.effects.push(json!({"type":if inner {"INNER_SHADOW"} else {"DROP_SHADOW"},
                            "visible":effect["visible"].as_bool().unwrap_or(true), "blendMode":"NORMAL",
                            "color":rgba(&shadow["color"]), "radius":number(&shadow["r"],0.).max(0.),
                            "spread":number(&shadow["spread"],0.), "offset":{"x":number(&shadow["dx"],0.), "y":number(&shadow["dy"],0.)}}));
                    }
                }
                "blur" | "gaussianBlur" | "backgroundBlur" | "uxdesign#blur" => appearance.effects.push(json!({
                    "type":if effect["type"] == "backgroundBlur" || params["backgroundEffect"] == true {"BACKGROUND_BLUR"} else {"LAYER_BLUR"},
                    "visible":effect["visible"].as_bool().unwrap_or(true),
                    "radius":number(&params["radius"],number(&params["r"],number(&params["blurAmount"],0.))).max(0.)
                })),
                other => appearance.unsupported.push(format!("Efecto XD sin equivalencia: {other}")),
            }
        }
        appearance
    }
}
