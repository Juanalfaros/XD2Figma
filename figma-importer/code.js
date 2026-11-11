figma.showUI(__html__, { width: 460, height: 390, themeColors: true });

const pending = new Map();
let requestId = 0;
let importing = false;

function requestAsset(id) {
  return new Promise((resolve, reject) => {
    const request = ++requestId;
    const timeout = setTimeout(() => {
      pending.delete(request);
      reject(new Error('Tiempo de lectura agotado: ' + id));
    }, 60000);
    pending.set(request, message => {
      clearTimeout(timeout);
      if (message.error) reject(new Error(message.error));
      else resolve(new Uint8Array(message.bytes));
    });
    figma.ui.postMessage({ type: 'asset', id, request });
  });
}

const n = (value, fallback = 0) => Number.isFinite(value) ? value : fallback;
const dimension = value => Math.max(0.01, n(value, 1));
const identity = [1, 0, 0, 1, 0, 0];

function transform(node, matrix, x = 0, y = 0, angle = 0) {
  const [a, b, c, d, tx, ty] = matrix;
  const cos = Math.cos(angle), sin = Math.sin(angle);
  node.relativeTransform = [
    [a * cos + c * sin, c * cos - a * sin, tx + a * x + c * y],
    [b * cos + d * sin, d * cos - b * sin, ty + b * x + d * y],
  ];
}

async function importDesign(design) {
  if (design?.schema_version !== 2 || !Array.isArray(design.artboards) || !Array.isArray(design.pasteboard)) {
    throw new Error('Versión de payload incompatible.');
  }
  const fontList = await figma.listAvailableFontsAsync();
  const loadedFonts = new Map();
  const images = new Map();
  const warnings = new Set();
  const roots = [];
  const created = new Set();
  const masters = new Map();

  // Huella FNV-1a de 64 bits (dos semillas) del subárbol: evita construir cadenas JSON
  // anidadas enormes al comparar instancias con su componente maestro.
  function fingerprint(text, seed) {
    let h = seed;
    for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 16777619);
    return h >>> 0;
  }
  // Igual que antes, solo cuentan los hijos: el componente y su instancia difieren en tipo y nombre.
  function signature(source) {
    const text = JSON.stringify((source.children ?? []).map(child => [
      child.type, child.name, child.visible, child.transform, child.geometry, child.style, signature(child),
    ]));
    return fingerprint(text, 2166136261).toString(36) + fingerprint(text, 3266489917).toString(36) + ':' + text.length;
  }
  const warn = message => {
    if (warnings.has(message)) return;
    warnings.add(message);
    figma.ui.postMessage({ type: 'log', text: message });
  };

  function create(node, parent) {
    created.add(node);
    parent.appendChild(node);
    return node;
  }

  async function font(source, layer) {
    const family = source?.family || 'Inter';
    const style = source?.style || 'Regular';
    const same = (a, b) => a.toLowerCase().replace(/[^a-z0-9]/g, '') === b.toLowerCase().replace(/[^a-z0-9]/g, '');
    let selected = fontList.find(item => same(item.fontName.family, family) && same(item.fontName.style, style))?.fontName;
    if (!selected) {
      selected = fontList.find(item => item.fontName.family === 'Inter' && same(item.fontName.style, style))?.fontName
        || { family: 'Inter', style: 'Regular' };
      warn(layer + ': fuente ' + family + ' ' + style + ' no disponible; se usó ' + selected.family + ' ' + selected.style + '.');
    }
    const key = JSON.stringify(selected);
    if (!loadedFonts.has(key)) loadedFonts.set(key, figma.loadFontAsync(selected));
    await loadedFonts.get(key);
    return selected;
  }

  async function paints(list, name) {
    const mapped = [];
    for (const paint of list ?? []) {
      if (paint.type !== 'IMAGE') {
        mapped.push(paint);
        continue;
      }
      try {
        if (!images.has(paint.assetId)) {
          images.set(paint.assetId, requestAsset(paint.assetId).then(bytes => figma.createImage(bytes).hash));
        }
        mapped.push({ type: 'IMAGE', scaleMode: paint.scaleMode, imageHash: await images.get(paint.assetId) });
      } catch (error) {
        if (design.strict) throw error;
        warn(name + ': relleno de imagen omitido. ' + error.message);
      }
    }
    return mapped;
  }

  async function appearance(node, source) {
    const style = source.appearance;
    if (!style) return;
    if ('fills' in node && (source.type !== 'TEXT' || style.fills.length)) node.fills = await paints(style.fills, source.name);
    if ('strokes' in node) {
      node.strokes = await paints(style.strokes, source.name);
      node.strokeWeight = style.strokeWeight;
      if ('strokeAlign' in node) node.strokeAlign = style.strokeAlign;
    }
    if ('effects' in node) node.effects = style.effects;
    if ('opacity' in node) node.opacity = style.opacity;
  }

  async function textNode(node, source) {
    const geometry = source.geometry ?? {};
    const style = source.style ?? {};
    const attrs = style.textAttributes ?? {};
    const size = dimension(n(style.font?.size, 16));
    const lines = (geometry.paragraphs ?? []).flatMap(p => p.lines ?? []).filter(line => line.length);
    node.fontName = await font(style.font, source.name);
    node.fontSize = size;
    node.characters = geometry.rawText ?? '';
    node.textAlignHorizontal = { left: 'LEFT', right: 'RIGHT', center: 'CENTER', justify: 'JUSTIFIED' }[
      String(attrs.paragraphAlign ?? attrs.alignment ?? geometry.frame?.align ?? '').toLowerCase()] ?? 'LEFT';
    const lineHeight = n(attrs.lineHeight ?? attrs.lineSpacing, lines.length > 1 ? n(lines[1][0].y) - n(lines[0][0].y) : 0);
    if (lineHeight > 0) node.lineHeight = { unit: 'PIXELS', value: lineHeight };
    if (source.appearance.letterSpacing) node.letterSpacing = source.appearance.letterSpacing;
    for (const run of lines.flat()) {
      if (!run.style || !Number.isInteger(run.from) || !Number.isInteger(run.to)
          || run.from < 0 || run.from >= run.to || run.to > node.characters.length) continue;
      if (run.style.font) {
        const f = await font({ ...style.font, ...run.style.font }, source.name);
        node.setRangeFontName(run.from, run.to, f);
        if (Number.isFinite(run.style.font.size)) node.setRangeFontSize(run.from, run.to, dimension(run.style.font.size));
      }
    }
    const width = geometry.frame?.width ?? geometry.frame?.bounds?.width;
    if (Number.isFinite(width) && width > 0) {
      node.textAutoResize = 'HEIGHT';
      node.resize(width, node.height);
    } else {
      node.textAutoResize = 'WIDTH_AND_HEIGHT';
    }
    return [n(lines[0]?.[0]?.x), n(lines[0]?.[0]?.y) - size * 0.8];
  }

  async function layer(source, parent, matrix = source.transform, bounds) {
    let node;
    let x = 0, y = 0, angle = 0;
    const g = source.geometry ?? {};
    try {
      if (source.fallback_svg) {
        node = create(figma.createNodeFromSvg(source.fallback_svg), parent);
        warn(source.name + ': se usó el fallback SVG del payload.');
      } else {
        switch (source.type) {
          case 'RECTANGLE':
            node = create(figma.createRectangle(), parent);
            node.resize(dimension(g.width), dimension(g.height));
            [x, y] = [n(g.x), n(g.y)];
            if (Array.isArray(g.r) && g.r.length === 4) {
              [node.topLeftRadius, node.topRightRadius, node.bottomRightRadius, node.bottomLeftRadius] = g.r.map(value => Math.max(0, n(value)));
            } else if (Number.isFinite(g.r)) node.cornerRadius = Math.max(0, g.r);
            break;
          case 'ELLIPSE': {
            node = create(figma.createEllipse(), parent);
            const rx = n(g.rx, n(g.r)), ry = n(g.ry, n(g.r));
            node.resize(dimension(rx * 2), dimension(ry * 2));
            [x, y] = [n(g.cx) - rx, n(g.cy) - ry];
            break;
          }
          case 'LINE': {
            node = create(figma.createLine(), parent);
            [x, y] = [n(g.x1), n(g.y1)];
            const dx = n(g.x2) - x, dy = n(g.y2) - y;
            node.resize(dimension(Math.hypot(dx, dy)), 0);
            angle = Math.atan2(dy, dx);
            break;
          }
          case 'TEXT':
            node = create(figma.createText(), parent);
            [x, y] = await textNode(node, source);
            break;
          case 'VECTOR':
            node = create(figma.createVector(), parent);
            if (source.vector_network) {
              try {
                await node.setVectorNetworkAsync(source.vector_network);
              } catch (error) {
                if (!g.path) throw error;
                node.vectorPaths = [{ windingRule: String(g.fillRule).toLowerCase() === 'evenodd' ? 'EVENODD' : 'NONZERO', data: g.path }];
                warn(source.name + ': se conservó como vector editable mediante VectorPaths. ' + error.message);
              }
              [x, y] = [node.x, node.y];
            } else if (g.path) {
              node.vectorPaths = [{ windingRule: String(g.fillRule).toLowerCase() === 'evenodd' ? 'EVENODD' : 'NONZERO', data: g.path }];
              [x, y] = [node.x, node.y];
            } else throw new Error('Geometría vectorial ausente');
            break;
          case 'COMPONENT':
            node = create(figma.createComponent(), parent);
            node.clipsContent = false;
            break;
          case 'INSTANCE': {
            const master = masters.get(source.component?.id);
            if (master && master.stateId === source.component.stateId && master.signature === signature(source)
                && !source.component.overrides) {
              node = create(master.node.createInstance(), parent);
            } else {
              node = create(figma.createFrame(), parent);
              node.clipsContent = false;
              warn(source.name + ': instancia reconstruida como marco editable para conservar su apariencia.');
            }
            break;
          }
          case 'FRAME':
          case 'GROUP':
            node = create(figma.createFrame(), parent);
            node.clipsContent = Boolean(bounds);
            break;
          default: throw new Error('Tipo de nodo inesperado: ' + source.type);
        }
        await appearance(node, source);
      }

      if (source.children?.length && node.type !== 'INSTANCE') {
        if (!('appendChild' in node)) throw new Error('El nodo no admite capas hijas');
        for (const child of source.children) {
          const childMatrix = [...(child.transform ?? identity)];
          if (bounds) {
            childMatrix[4] -= n(bounds.x);
            childMatrix[5] -= n(bounds.y);
          }
          await layer(child, node, childMatrix);
        }
      }
      if (bounds) {
        node.resize(dimension(bounds.width), dimension(bounds.height));
      } else if ((node.type === 'COMPONENT' || node.type === 'INSTANCE')
          && n(source.component?.width) > 0 && n(source.component?.height) > 0) {
        node.resize(source.component.width, source.component.height);
      } else if (source.auto_layout && 'layoutMode' in node) {
        const layout = source.auto_layout;
        const children = node.children;
        const columns = Math.min(layout.columns, children.length);
        const width = children.slice(0, columns).reduce((sum, child) => sum + child.width, 0)
          + Math.max(0, columns - 1) * layout.itemSpacing;
        node.layoutMode = layout.layoutMode;
        node.itemSpacing = layout.itemSpacing;
        if (layout.layoutWrap === 'WRAP' && width > 0) {
          node.layoutWrap = 'WRAP';
          node.counterAxisSpacing = layout.counterAxisSpacing;
          node.primaryAxisSizingMode = 'FIXED';
          node.resize(width + 0.01, node.height);
        } else node.primaryAxisSizingMode = 'AUTO';
        node.counterAxisSizingMode = 'AUTO';
      } else if (source.type === 'GROUP' && !source.fallback_svg && node.children.length) {
        const group = create(figma.group([...node.children], parent), parent);
        [x, y] = [group.x, group.y];
        node.remove();
        node = group;
        await appearance(node, source);
      }
      node.name = source.name || source.type;
      node.visible = source.visible !== false;
      transform(node, matrix ?? identity, x, y, angle);
      if (node.type === 'COMPONENT' && source.component?.id) {
        masters.set(source.component.id, { node, stateId: source.component.stateId, signature: signature(source) });
      }
      return node;
    } catch (error) {
      if (design.strict) throw error;
      warn(source.name + ': ' + error.message);
      if (!node) return undefined;
      node.name = source.name || source.type;
      try { transform(node, matrix ?? identity, x, y, angle); } catch (transformError) {
        warn(source.name + ': ' + transformError.message);
      }
      return node;
    }
  }

  try {
    for (const board of design.artboards) {
      const { bounds, node } = board;
      const root = await layer(node, figma.currentPage, [1, 0, 0, 1, n(bounds.x), n(bounds.y)], bounds);
      if (root) roots.push(root);
    }
    for (const node of design.pasteboard) {
      const root = await layer(node, figma.currentPage);
      if (root) roots.push(root);
    }
  } catch (error) {
    for (const node of Array.from(created).reverse()) if (!node.removed) node.remove();
    throw error;
  }
  if (roots.length) {
    figma.currentPage.selection = roots;
    figma.viewport.scrollAndZoomIntoView(roots);
  }
  return roots.length + ' capas raíz creadas. Guarda el documento con File → Save local copy (.fig).';
}

figma.ui.onmessage = async message => {
  if (message?.type === 'asset') {
    const resolve = pending.get(message.request);
    pending.delete(message.request);
    if (resolve) resolve(message);
  } else if (message?.type === 'import' && !importing) {
    importing = true;
    try {
      const text = await importDesign(message.design);
      figma.ui.postMessage({ type: 'done', text });
    } catch (error) {
      figma.ui.postMessage({ type: 'done', text: 'Error: ' + error.message });
    } finally {
      importing = false;
    }
  }
};
