import React from 'react';
import { createRoot } from 'react-dom/client';
import { version } from '../package.json';
import Icon from './Icon';
import { useConversion, native, basename } from './useConversion';
import './style.css';

const steps = ['Importar', 'Configurar', 'Convertir', 'Revisar'];
const titles = ['Empieza con tus archivos XD.', 'Prepara tu conversión.', 'Estamos convirtiendo tu diseño.', 'Revisa tu resultado.'];
const descriptions = ['Añade uno o varios archivos. Después podrás revisar las opciones.', 'Confirma el destino y los ajustes antes de comenzar.', 'Puedes seguir el avance de cada archivo sin perder el contexto.', 'Revisa los archivos generados y los detalles de la conversión.'];
const statuses = { waiting: 'En espera', active: 'Convirtiendo', done: 'Completado', error: 'Error' };
const number = value => new Intl.NumberFormat('es-CL').format(value);
function Button({ icon, children, primary, className = '', ...props }) {
  return <button className={`${primary ? 'primary' : 'secondary'} ${className}`} {...props}>{icon && <Icon name={icon} size={17}/>} {children}</button>;
}
function Badge({ status, children }) { return <span className={`badge ${status || ''}`}>{children}</span>; }
function FileRow({ file, c, removable = false }) {
  return <div className={`file-row ${file.status}`}>
    <Icon name="file" size={24}/><div className="file-info"><strong title={file.path}>{basename(file.path)}</strong><small>{file.status === 'error' ? file.message : file.report ? `${number(file.report.summary.artboards)} artboards · ${number(file.report.summary.layers)} capas` : file.status === 'active' ? file.phase : 'Archivo preparado'}</small></div>
    {removable ? <button className="icon-button" title="Quitar de la cola" aria-label={`Quitar ${basename(file.path)}`} disabled={c.busy} onClick={() => c.remove(file.path)}><Icon name="close" size={16}/></button> : <Badge status={file.status}>{statuses[file.status]}</Badge>}
  </div>;
}
function Summary({ c }) {
  const completed = c.files.filter(f => f.status === 'done').length;
  const failed = c.files.filter(f => f.status === 'error').length;
  return <aside className="session-summary panel" aria-label="Resumen de sesión">
    <p className="eyebrow">RESUMEN DE SESIÓN</p>
    <h2>{c.files.length ? `${c.files.length} ${c.files.length === 1 ? 'archivo' : 'archivos'} en la sesión` : 'Aún no hay archivos'}</h2>
    <p className="summary-file" title={c.active?.path || c.files[0]?.path}>{c.active ? basename(c.active.path) : c.files.length === 1 ? basename(c.files[0].path) : c.files.length ? `${completed} completados · ${c.pending.length} pendientes` : 'Aquí verás la cola y el resultado.'}</p>
    <hr/><p className="eyebrow">SALIDA</p><strong>{c.options.importPackage ? 'Paquete para el plugin (.zip)' : 'Figma nativo (.fig)'}</strong>
    <p>{c.options.importPackage ? 'JSON, recursos e importador para Figma.' : 'Textos, vectores y componentes editables.'}</p>
    <hr/><p className="eyebrow">DESTINO</p><strong className="path" title={c.destination || undefined}>{c.destination || 'Elegir al iniciar la conversión'}</strong><p>Los originales se conservan.</p>
    <hr/><div className="summary-status" role="status"><Icon name={c.running ? 'spinner' : failed ? 'warning' : completed ? 'check' : 'clock'} size={15}/>{c.running ? c.active ? `Convirtiendo · ${c.active.percent}%` : 'Preparando conversión' : failed ? `${failed} ${failed === 1 ? 'archivo con error' : 'archivos con error'}` : completed ? `${completed} completados` : c.files.length ? 'Listo para empezar' : 'Esperando archivos'}</div>
    {c.running && c.files.length > 1 && <small>{completed} de {c.files.length} completados</small>}
  </aside>;
}
function ImportStep({ c }) {
  return <section className={`panel import-card ${c.dragging ? 'dragging' : ''}`} aria-label="Zona de importación">
    <Icon name="upload" size={36}/><h2>{c.dragging ? 'Suelta tus archivos para añadirlos' : 'Arrastra tus archivos .xd aquí'}</h2>
    <p>Diseños editables en Figma, procesados localmente.</p><Button primary onClick={c.chooseFiles} disabled={!native || c.busy}>{c.picker === 'files' ? 'Abriendo selector…' : 'Seleccionar archivos'}</Button>
    <p>También puedes añadir varios archivos a la vez.</p><div className="card-footnote">Compatible con archivos Adobe XD (.xd)</div>
  </section>;
}
function ConfigureStep({ c }) {
  return <section className="panel configure-card">
    <div className="file-list">{c.files.map(file => <FileRow key={file.path} file={file} c={c} removable/>)}</div>
    <div className="card-section folder-section"><p>Guardar el resultado en</p><div className="inline"><Icon name="folder"/><strong className="path" title={c.destination || undefined}>{c.destination || 'Seleccionar carpeta de salida'}</strong><button className="plain" disabled={!native || c.busy} onClick={c.chooseDestination}>{c.picker === 'folder' ? 'Abriendo…' : 'Cambiar'}</button></div></div>
    <div className="card-section recommended"><h3>Conversión recomendada</h3><div className="option-summary">{c.options.keepComponents && <span><Icon name="check" size={15}/>Componentes nativos</span>}{c.options.repeatGridAutoLayout && <span><Icon name="check" size={15}/>Repeat Grids a Auto Layout</span>}</div><p>{c.options.importPackage ? 'Paquete para el plugin (.zip)' : 'Figma nativo (.fig)'} · {c.options.strict ? 'Modo estricto' : 'Modo tolerante'}</p><div className="inline"><Button onClick={() => c.setView('preferences')}>Opciones avanzadas</Button><button className="plain" onClick={c.chooseFiles} disabled={!native || c.busy}><Icon name="upload" size={16}/>Añadir archivos</button></div></div>
  </section>;
}
function ConvertStep({ c }) {
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
function ResultDetails({ result, c }) {
  const report = result.report;
  return <section className="panel result-card"><div className="file-heading"><Icon name="file" size={26}/><div className="file-info"><h3 title={report.outputPath}>{basename(report.outputPath)}</h3><small className="path">{report.outputPath}</small></div><Badge status="done">Completado</Badge></div>
    <div className="metrics"><div><strong>{number(report.summary.artboards)}</strong><small>Artboards</small></div><div><strong>{number(report.summary.layers)}</strong><small>Capas convertidas</small></div><div><strong>{result.elapsed.toFixed(1)} s</strong><small>Tiempo total</small></div></div>
    {report.summary.warnings.length ? <div className="warning-box"><h3><Icon name="warning" size={18}/>{report.summary.warnings.length} {report.summary.warnings.length === 1 ? 'detalle requiere' : 'detalles requieren'} tu revisión</h3><ul>{report.summary.warnings.slice(0,2).map(w => <li key={w}>{w}</li>)}</ul><button className="plain" onClick={() => c.setDrawerOpen(true)}>Ver el registro completo <Icon name="arrow" size={15}/></button></div> : <div className="success-message"><Icon name="check" size={18}/>Conversión completada sin advertencias.</div>}
    {report.imagesPath && <button className="plain path" onClick={() => c.reveal(report.imagesPath)}><Icon name="folder" size={16}/>Mostrar imágenes extraídas</button>}
    {!report.outputPath.endsWith('.fig') && <details className="plugin-help"><summary>Cómo obtener el .fig desde el paquete</summary><ol><li>Descomprime el ZIP.</li><li>En Figma Desktop, importa <code>figma-importer/manifest.json</code> desde Plugins → Development.</li><li>Ejecuta XD2Figma Importer y selecciona la carpeta que contiene <code>design.json</code>.</li><li>Guarda el resultado con File → Save local copy.</li></ol></details>}
  </section>;
}
function ReviewStep({ c }) {
  const currentResults = c.files.filter(f => f.report);
  const result = currentResults.find(f => f.path === c.selected) || currentResults.at(-1);
  return <div className="step-stack">{currentResults.length > 1 && <label className="result-selector">Resultado<select value={result?.path || ''} onChange={e => c.setSelected(e.target.value)}>{currentResults.map(f => <option key={f.path} value={f.path}>{basename(f.report.outputPath)}</option>)}</select></label>}
    {result && <ResultDetails result={result} c={c}/>}
    {c.files.filter(f => f.status === 'error' || f.status === 'waiting').map(f => <section className="panel unresolved" key={f.path}><FileRow file={f} c={c}/><Button icon={f.status === 'error' ? 'retry' : 'arrow'} onClick={() => c.convert([f])} disabled={c.busy || !native}>{f.status === 'error' ? 'Reintentar archivo' : 'Convertir archivo pendiente'}</Button></section>)}
    {!c.files.length && <EmptyResults c={c}/>}
  </div>;
}
function EmptyResults({ c }) { return <div className="panel empty-results"><Icon name="folder" size={32}/><h2>Aquí encontrarás tus resultados</h2><p>Los archivos que conviertas aparecerán durante esta sesión.</p><Button onClick={() => c.setView('conversion')}>Ir a conversiones</Button></div>; }
function Preferences({ c }) {
  return <><section className="panel preferences-panel"><h2>Conversión y recursos</h2><div className="preference-fields">{[
    ['keepComponents','Conservar componentes','Mantén los elementos reutilizables editables.'],
    ['repeatGridAutoLayout','Repeat Grids a Auto Layout','Reconstruye las filas y columnas de Adobe XD.'],
    ['extractImages','Extraer imágenes','Guarda una carpeta con los recursos originales.'],
    ['strict','Modo estricto','Detén el archivo si una capa requiere una conversión aproximada.'],
    ...(c.options.importPackage ? [['preservePrototype','Preservar prototipado','Incluye los datos de interacciones en el paquete del plugin.']] : []),
  ].map(([key,label,hint]) => <label className="preference-row" key={key}><span><strong>{label}</strong><small>{hint}</small></span><input type="checkbox" checked={c.options[key]} disabled={c.busy} onChange={e => c.setOptions({ ...c.options, [key]: e.target.checked })}/></label>)}</div>
  <div className="preference-row output-preference"><span><strong>Formato de salida</strong><small>El archivo .fig es el formato recomendado.</small></span><select aria-label="Formato de salida" value={c.options.importPackage ? 'zip' : 'fig'} disabled={c.busy} onChange={e => c.setOptions({...c.options, importPackage:e.target.value === 'zip'})}><option value="fig">Figma nativo (.fig)</option><option value="zip">Paquete para el plugin (.zip)</option></select></div>
  <div className="preference-row appearance"><span><strong>Apariencia</strong><small>Una preferencia personal, fuera del flujo principal.</small></span><div className="theme-options" role="group" aria-label="Apariencia">{[['system','Sistema'],['light','Claro'],['dark','Oscuro']].map(([value,label]) => <button key={value} className={c.theme === value ? 'primary' : 'secondary'} aria-pressed={c.theme === value} onClick={() => c.setTheme(value)}>{label}</button>)}</div></div></section>
  <div className="action-bar"><p>Las preferencias se guardan automáticamente.</p><Button primary onClick={() => c.setView('conversion')}>Volver a la conversión</Button></div></>;
}
function LogDrawer({ c }) {
  return c.drawerOpen && <section className="log-drawer" aria-label="Registro de sesión"><header><h2><Icon name="log" size={17}/>Registro de la sesión <Badge>{c.logs.length}</Badge></h2><div className="inline"><button className="plain" disabled={c.busy || !c.logs.length} onClick={() => c.setLogs([])}>Limpiar registro</button><button className="icon-button" aria-label="Cerrar registro" onClick={() => c.setDrawerOpen(false)}><Icon name="close" size={18}/></button></div></header><div className="log-entries" role="log" aria-live="polite">{!c.logs.length ? <p>Sin advertencias durante esta sesión.</p> : <ol>{c.logs.map((l,i) => <li key={i} className={l.level}><Icon name={l.level === 'error' ? 'close' : 'warning'} size={16}/><div><strong>{basename(l.path)}</strong><p>{l.message}</p></div></li>)}</ol>}</div></section>;
}
function App() {
  const c = useConversion();
  const reviewResult = c.files.find(f => f.path === c.selected && f.report) || c.files.filter(f => f.report).at(-1);
  const preferences = c.view === 'preferences'; const resultsView = c.view === 'results';
  const title = preferences ? 'Preferencias' : resultsView ? 'Resultados' : 'Nueva conversión';
  function changeStep(index) { if (c.busy || index === 2 || (index > 0 && !c.files.length) || (index === 3 && c.step !== 3)) return; c.setStep(index); }
  return <div className={`app-shell ${c.dragging ? 'drag-over' : ''}`}>
    <a className="skip-link" href="#main-content">Saltar al contenido</a>
    <nav className="navigation" aria-label="Navegación principal"><div className="brand">XD2Figma</div><p className="eyebrow">ESPACIO LOCAL</p><button className={`nav-item ${c.view === 'conversion' ? 'selected' : ''}`} aria-current={c.view === 'conversion' ? 'page' : undefined} onClick={() => c.setView('conversion')}><Icon name="layers"/>Conversiones{c.running && <Icon name="spinner" size={13}/>}</button><button className={`nav-item ${resultsView ? 'selected' : ''}`} aria-current={resultsView ? 'page' : undefined} onClick={() => c.setView('results')}><Icon name="folder"/>Resultados{c.history.length > 0 && <span className="nav-count">{c.history.length}</span>}</button>
      <div className="navigation-bottom"><button className={`nav-item ${preferences ? 'selected' : ''}`} aria-current={preferences ? 'page' : undefined} onClick={() => c.setView('preferences')}><Icon name="settings"/>Preferencias</button><button className="nav-item log-toggle" aria-expanded={c.drawerOpen} onClick={() => c.setDrawerOpen(!c.drawerOpen)}><Icon name="log"/>Registro{c.logs.length > 0 && <span className="nav-count">{c.logs.length}</span>}</button><small>Todo permanece local.</small><span className="version">v{version}</span></div>
    </nav>
    <main id="main-content" className="main-content"><header className="page-header"><div><h1>{title}</h1><p>{preferences ? 'Configura cómo quieres trabajar.' : resultsView ? 'Archivos generados durante esta sesión.' : `Sesión local${c.files.length ? ` · ${c.files.length} ${c.files.length === 1 ? 'archivo' : 'archivos'}` : ''}`}</p></div><Badge><Icon name="shield" size={13}/>Solo en este equipo</Badge></header>
      {!native && <div className="notice preview-notice" role="status"><Icon name="shield" size={18}/>Vista previa de la interfaz. Abre la app de escritorio para convertir archivos locales.</div>}
      {c.error && <div className="notice error-notice" role="alert"><Icon name="warning" size={18}/><p>{c.error}</p><button className="icon-button" aria-label="Cerrar aviso" onClick={() => c.setError('')}><Icon name="close" size={15}/></button></div>}
      {preferences ? <Preferences c={c}/> : resultsView ? <><div className="results-layout">{!c.history.length ? <EmptyResults c={c}/> : <><section className="panel result-library"><h2>Archivos generados</h2>{c.history.map(f => <button className={`result-list-item ${c.result?.path === f.path ? 'selected' : ''}`} key={f.path} onClick={() => c.setSelected(f.path)}><Icon name="file"/><span><strong>{basename(f.report.outputPath)}</strong><small>{number(f.report.summary.artboards)} artboards · {f.report.summary.warnings.length} advertencias</small></span><Icon name="chevron" size={15}/></button>)}</section><div className="step-stack"><ResultDetails result={c.result} c={c}/><Button icon="folder" primary onClick={() => c.reveal(c.result.report.outputPath)}>Mostrar archivo en su carpeta</Button></div></>}</div></> : <>
      <ol className="stepper" aria-label="Pasos de conversión">{steps.map((label,i) => <li key={label} className={i === c.step ? 'current' : i < c.step ? 'complete' : ''}><button aria-current={i === c.step ? 'step' : undefined} disabled={c.busy || i === 2 || (i > 0 && !c.files.length) || (i === 3 && c.step !== 3)} onClick={() => changeStep(i)}><span className="step-number">{i < c.step ? <Icon name="check" size={14}/> : i+1}</span><span>{label}</span></button>{i < 3 && <Icon name="arrow" size={14}/>}</li>)}</ol>
      <div className="conversion-layout"><div className="stage"><div className="stage-heading"><h2>{titles[c.step]}</h2><p>{descriptions[c.step]}</p></div>{c.step === 0 ? <ImportStep c={c}/> : c.step === 1 ? <ConfigureStep c={c}/> : c.step === 2 ? <ConvertStep c={c}/> : <ReviewStep c={c}/>}</div><Summary c={c}/></div>
      <div className="action-bar"><p role="status">{c.picker === 'folder' ? 'Selecciona una carpeta de salida.' : c.stopping ? 'Terminando el archivo actual. El resto quedará pendiente.' : c.step === 0 ? 'Añade un archivo para continuar.' : c.step === 1 ? 'Tus ajustes están listos.' : c.step === 2 ? 'Puedes detener el lote al terminar el archivo actual.' : reviewResult ? reviewResult.report.outputPath.endsWith('.fig') ? 'Importa el .fig desde el explorador de archivos de Figma.' : 'Importa el paquete mediante el plugin de Figma.' : 'Revisa los errores e inténtalo de nuevo.'}</p><div className="action-buttons">
        {c.step === 0 && <Button primary disabled={!c.files.length || c.busy} onClick={() => c.setStep(1)}>Continuar<Icon name="arrow" size={16}/></Button>}
        {c.step === 1 && <><Button disabled={c.busy} onClick={() => c.setStep(0)}>Volver</Button><Button primary disabled={!native || c.busy || !c.pending.length} onClick={() => c.convert(c.pending)}>{c.picker ? 'Seleccionando carpeta…' : 'Iniciar conversión'}</Button></>}
        {c.step === 2 && <Button disabled={c.stopping || Boolean(c.picker)} onClick={c.stopBatch}>{c.stopping ? 'Deteniendo lote…' : 'Detener después de este'}</Button>}
        {c.step === 3 && <><Button disabled={c.busy} onClick={c.newSession}>Nueva conversión</Button>{reviewResult ? <Button primary icon="folder" onClick={() => c.reveal(reviewResult.report.outputPath)}>Abrir carpeta de resultados</Button> : <Button primary disabled={!native || c.busy || !c.pending.length} onClick={() => c.convert(c.pending)}>Reintentar lote</Button>}</>}
      </div></div></>}
    </main><LogDrawer c={c}/>
    {c.dragging && <div className="drag-overlay" aria-live="polite"><Icon name="upload" size={40}/><h2>Suelta tus archivos .xd para añadirlos</h2></div>}
  </div>;
}
createRoot(document.getElementById('root')).render(<App/>);
