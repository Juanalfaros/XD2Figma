use serde::Serialize;
use svgtypes::{SimplePathSegment as Path, SimplifyingPathParser};

#[derive(Clone, Copy, Serialize, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn difference(self, origin: Self, scale: f64) -> Self {
        Self {
            x: (self.x - origin.x) * scale,
            y: (self.y - origin.y) * scale,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Segment {
    start: usize,
    end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    tangent_start: Option<Point>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tangent_end: Option<Point>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    winding_rule: &'static str,
    loops: Vec<Vec<usize>>,
}

#[derive(Default, Serialize)]
pub struct VectorNetwork {
    vertices: Vec<Point>,
    segments: Vec<Segment>,
    regions: Vec<Region>,
}

impl VectorNetwork {
    pub fn parse(
        path: &str,
        even_odd: bool,
        filled: bool,
        stroked: bool,
    ) -> Result<Self, &'static str> {
        let mut network = Self::default();
        let mut loops = Vec::new();
        let (mut start, mut current, mut first_segment) = (0, 0, 0);
        for item in SimplifyingPathParser::from(path) {
            let item = item.map_err(|_| "Path SVG inválido")?;
            if let Path::MoveTo { x, y } = item {
                if filled && network.segments.len() > first_segment {
                    loops.push(network.close_contour(start, current, first_segment, stroked)?);
                }
                start = network.vertices.len();
                current = start;
                first_segment = network.segments.len();
                network.vertices.push(Point { x, y });
                continue;
            }
            let from = *network
                .vertices
                .get(current)
                .ok_or("Path sin punto inicial")?;
            let (target, tangent_start, tangent_end) = match item {
                Path::LineTo { x, y } => (Point { x, y }, None, None),
                Path::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let target = Point { x, y };
                    (
                        target,
                        Some(Point { x: x1, y: y1 }.difference(from, 1.)),
                        Some(Point { x: x2, y: y2 }.difference(target, 1.)),
                    )
                }
                Path::Quadratic { x1, y1, x, y } => {
                    let target = Point { x, y };
                    let control = Point { x: x1, y: y1 };
                    (
                        target,
                        Some(control.difference(from, 2. / 3.)),
                        Some(control.difference(target, 2. / 3.)),
                    )
                }
                Path::ClosePath => (network.vertices[start], None, None),
                Path::MoveTo { .. } => unreachable!(),
            };
            let closing = matches!(item, Path::ClosePath);
            let end = if closing {
                start
            } else {
                network.vertices.push(target);
                network.vertices.len() - 1
            };
            if current != end {
                network.segments.push(Segment {
                    start: current,
                    end,
                    tangent_start,
                    tangent_end,
                });
            }
            current = end;
            if closing && network.segments.len() > first_segment {
                loops.push((first_segment..network.segments.len()).collect());
                first_segment = network.segments.len();
            }
        }
        if network.segments.is_empty() {
            return Err("Path sin segmentos");
        }
        if filled && network.segments.len() > first_segment {
            loops.push(network.close_contour(start, current, first_segment, stroked)?);
        }
        if filled && !loops.is_empty() {
            network.regions.push(Region {
                winding_rule: if even_odd { "EVENODD" } else { "NONZERO" },
                loops,
            });
        }
        if network
            .vertices
            .iter()
            .chain(
                network
                    .segments
                    .iter()
                    .flat_map(|s| s.tangent_start.iter().chain(s.tangent_end.iter())),
            )
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return Err("Coordenadas vectoriales fuera de rango");
        }
        Ok(network)
    }
    fn close_contour(
        &mut self,
        start: usize,
        end: usize,
        first: usize,
        stroked: bool,
    ) -> Result<Vec<usize>, &'static str> {
        if self.vertices[start] == self.vertices[end] {
            self.segments
                .last_mut()
                .ok_or("Contorno sin segmentos")?
                .end = start;
        } else if stroked {
            // An implicit fill edge must not introduce a new visible stroke.
            return Err("Contorno relleno sin cierre explícito");
        } else {
            self.segments.push(Segment {
                start: end,
                end: start,
                tangent_start: None,
                tangent_end: None,
            });
        }
        Ok((first..self.segments.len()).collect())
    }
}

impl VectorNetwork {
    pub(crate) fn to_figcraft(&self) -> figcraft_doc::VectorNetwork {
        use figcraft_doc::{Region, Segment, Vertex, WindingRule};
        let tangent = |point: Option<Point>| point.map(|p| (p.x, p.y)).unwrap_or_default();
        figcraft_doc::VectorNetwork {
            vertices: self
                .vertices
                .iter()
                .map(|p| Vertex::new(p.x, p.y))
                .collect(),
            segments: self
                .segments
                .iter()
                .map(|s| Segment {
                    a: s.start,
                    b: s.end,
                    ta: tangent(s.tangent_start),
                    tb: tangent(s.tangent_end),
                })
                .collect(),
            regions: self
                .regions
                .iter()
                .map(|r| Region {
                    loops: r.loops.clone(),
                    winding: if r.winding_rule == "EVENODD" {
                        WindingRule::EvenOdd
                    } else {
                        WindingRule::NonZero
                    },
                    ..Region::default()
                })
                .collect(),
        }
    }
}
