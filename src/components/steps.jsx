import React from 'react';
import Icon from '../Icon';
import { native, basename } from '../useConversion';
import { Button, Badge, FileRow, number } from './common';

export function ImportStep({ c }) {
  return <section className={`panel import-card ${c.dragging ? 'dragging' : ''}`} aria-label="Zona de importación">
    <Icon name="upload" size={36}/><h2>{c.dragging ? 'Suelta tus archivos para añadirlos' : 'Arrastra tus archivos .xd aquí'}</h2>
    <p>Diseños editables en Figma, procesados localmente.</p><Button primary onClick={c.chooseFiles} disabled={!native || c.busy}>{c.picker === 'files' ? 'Abriendo selector…' : 'Seleccionar archivos'}</Button>
    <p>También puedes añadir varios archivos a la vez.</p><div className="card-footnote">Compatible con archivos Adobe XD (.xd)</div>
  </section>;
}
export function ConfigureStep({ c }) {
  return <section className="panel configure-card">
    <div className="file-list">{c.files.map(file => <FileRow key={file.path} file={file} c={c} removable/>)}</div>
    <div className="card-section folder-section"><p>Guardar el resultado en</p><div className="inline"><Icon name="folder"/><strong className="path" title={c.destination || undefined}>{c.destination || 'Seleccionar carpeta de salida'}</strong><button className="plain" disabled={!native || c.busy} onClick={c.chooseDestination}>{c.picker === 'folder' ? 'Abriendo…' : 'Cambiar'}</button></div></div>
    <div className="card-section recommended"><h3>Conversión recomendada</h3><div className="option-summary">{c.options.keepComponents && <span><Icon name="check" size={15}/>Componentes nativos</span>}{c.options.repeatGridAutoLayout && <span><Icon name="check" size={15}/>Repeat Grids a Auto Layout</span>}</div><p>{c.options.importPackage ? 'Paquete para el plugin (.zip)' : 'Figma nativo (.fig)'} · {c.options.strict ? 'Modo estricto' : 'Modo tolerante'}</p><div className="inline"><Button onClick={() => c.setView('preferences')}>Opciones avanzadas</Button><button className="plain" onClick={c.chooseFiles} disabled={!native || c.busy}><Icon name="upload" size={16}/>Añadir archivos</button></div></div>
  </section>;
}
export function ConvertStep({ c }) {
  const active = c.active;
  const percent = active?.percent || 0;
  return <div className="step-stack"><section className="panel conversion-card" aria-live="polite">
    <div className="file-heading"><Icon name="file" size={25}/><strong className="path">{active ? basename(active.path) : 'Preparando el siguiente archivo'}</strong><Badge status="active">En curso</Badge></div>
    <div className="progress-heading"><span>{active?.phase || 'Iniciando'}</span><strong>{percent}%</strong></div><progress max="100" value={percent} aria-label="Progreso del archivo actual"/><p className="current-element">{active?.element || 'Preparando la estructura del documento'}</p>
    <div className="pipeline">{[[25, 'Contenedor leído y recursos extraídos'], [85, 'Reconstruir capas editables'], [100, `Generar y guardar el ${c.options.importPackage ? 'paquete .zip' : 'archivo .fig'}`]].map(([limit, label], i) => {
      const done = percent >= limit; const current = !done && (i === 0 || percent >= [0,25,85][i]);
      return <div key={limit} className={current ? 'current' : ''}><Icon name={done ? 'check' : current ? 'spinner' : 'clock'} size={15}/>{label}</div>;
    })}</div>
  </section>{c.files.length > 1 && <section className="panel queue-panel"><h3>Cola del lote</h3>{c.files.map(f => <FileRow file={f} key={f.path} c={c}/>)}</section>}</div>;
}
export function ResultDetails({ result, c }) {
  const report = result.report;
  return <section className="panel result-card"><div className="file-heading"><Icon name="file" size={26}/><div className="file-info"><h3 title={report.outputPath}>{basename(report.outputPath)}</h3><small className="path">{report.outputPath}</small></div><Badge status="done">Completado</Badge></div>
    <div className="metrics"><div><strong>{number(report.summary.artboards)}</strong><small>Artboards</small></div><div><strong>{number(report.summary.layers)}</strong><small>Capas convertidas</small></div><div><strong>{result.elapsed.toFixed(1)} s</strong><small>Tiempo total</small></div></div>
    {report.summary.warnings.length ? <div className="warning-box"><h3><Icon name="warning" size={18}/>{report.summary.warnings.length} {report.summary.warnings.length === 1 ? 'detalle requiere' : 'detalles requieren'} tu revisión</h3><ul>{report.summary.warnings.slice(0,2).map(w => <li key={w}>{w}</li>)}</ul><button className="plain" onClick={() => c.setDrawerOpen(true)}>Ver el registro completo <Icon name="arrow" size={15}/></button></div> : <div className="success-message"><Icon name="check" size={18}/>Conversión completada sin advertencias.</div>}
    {report.imagesPath && <button className="plain path" onClick={() => c.reveal(report.imagesPath)}><Icon name="folder" size={16}/>Mostrar imágenes extraídas</button>}
    {!report.outputPath.endsWith('.fig') && <details className="plugin-help"><summary>Cómo obtener el .fig desde el paquete</summary><ol><li>Descomprime el ZIP.</li><li>En Figma Desktop, importa <code>figma-importer/manifest.json</code> desde Plugins → Development.</li><li>Ejecuta XD2Figma Importer y selecciona la carpeta que contiene <code>design.json</code>.</li><li>Guarda el resultado con File → Save local copy.</li></ol></details>}
  </section>;
}
export function ReviewStep({ c }) {
  const currentResults = c.files.filter(f => f.report);
  const result = currentResults.find(f => f.path === c.selected) || currentResults.at(-1);
  return <div className="step-stack">{currentResults.length > 1 && <label className="result-selector">Resultado<select value={result?.path || ''} onChange={e => c.setSelected(e.target.value)}>{currentResults.map(f => <option key={f.path} value={f.path}>{basename(f.report.outputPath)}</option>)}</select></label>}
    {result && <ResultDetails result={result} c={c}/>}
    {c.files.filter(f => f.status === 'error' || f.status === 'waiting').map(f => <section className="panel unresolved" key={f.path}><FileRow file={f} c={c}/><Button icon={f.status === 'error' ? 'retry' : 'arrow'} onClick={() => c.convert([f])} disabled={c.busy || !native}>{f.status === 'error' ? 'Reintentar archivo' : 'Convertir archivo pendiente'}</Button></section>)}
    {!c.files.length && <EmptyResults c={c}/>}
  </div>;
}
export function EmptyResults({ c }) { return <div className="panel empty-results"><Icon name="folder" size={32}/><h2>Aquí encontrarás tus resultados</h2><p>Los archivos que conviertas quedarán guardados aquí.</p><Button onClick={() => c.setView('conversion')}>Ir a conversiones</Button></div>; }
