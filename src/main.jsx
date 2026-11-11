import React from 'react';
import { createRoot } from 'react-dom/client';
import { version } from '../package.json';
import Icon from './Icon';
import { useConversion, native, basename } from './useConversion';
import { Button, Badge, number } from './components/common';
import { Summary } from './components/Summary';
import { ImportStep, ConfigureStep, ConvertStep, ReviewStep, ResultDetails, EmptyResults } from './components/steps';
import { Preferences } from './components/Preferences';
import { LogDrawer } from './components/LogDrawer';
import './style.css';

const steps = ['Importar', 'Configurar', 'Convertir', 'Revisar'];
const titles = ['Empieza con tus archivos XD.', 'Prepara tu conversión.', 'Estamos convirtiendo tu diseño.', 'Revisa tu resultado.'];
const descriptions = ['Añade uno o varios archivos. Después podrás revisar las opciones.', 'Confirma el destino y los ajustes antes de comenzar.', 'Puedes seguir el avance de cada archivo sin perder el contexto.', 'Revisa los archivos generados y los detalles de la conversión.'];
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
    <main id="main-content" className="main-content"><header className="page-header"><div><h1>{title}</h1><p>{preferences ? 'Configura cómo quieres trabajar.' : resultsView ? 'Archivos generados, guardados entre sesiones.' : `Sesión local${c.files.length ? ` · ${c.files.length} ${c.files.length === 1 ? 'archivo' : 'archivos'}` : ''}`}</p></div><Badge><Icon name="shield" size={13}/>Solo en este equipo</Badge></header>
      {!native && <div className="notice preview-notice" role="status"><Icon name="shield" size={18}/>Vista previa de la interfaz. Abre la app de escritorio para convertir archivos locales.</div>}
      {c.error && <div className="notice error-notice" role="alert"><Icon name="warning" size={18}/><p>{c.error}</p><button className="icon-button" aria-label="Cerrar aviso" onClick={() => c.setError('')}><Icon name="close" size={15}/></button></div>}
      {preferences ? <Preferences c={c}/> : resultsView ? <><div className="results-layout">{!c.history.length ? <EmptyResults c={c}/> : <><section className="panel result-library"><h2>Archivos generados</h2><button className="plain" disabled={c.busy} onClick={c.clearHistory}>Borrar historial</button>{c.history.map(f => <button className={`result-list-item ${c.result?.path === f.path ? 'selected' : ''}`} key={f.path} onClick={() => c.setSelected(f.path)}><Icon name="file"/><span><strong>{basename(f.report.outputPath)}</strong><small>{number(f.report.summary.artboards)} artboards · {f.report.summary.warnings.length} advertencias</small></span><Icon name="chevron" size={15}/></button>)}</section><div className="step-stack"><ResultDetails result={c.result} c={c}/><Button icon="folder" primary onClick={() => c.reveal(c.result.report.outputPath)}>Mostrar archivo en su carpeta</Button></div></>}</div></> : <>
      <ol className="stepper" aria-label="Pasos de conversión">{steps.map((label,i) => <li key={label} className={i === c.step ? 'current' : i < c.step ? 'complete' : ''}><button aria-current={i === c.step ? 'step' : undefined} disabled={c.busy || i === 2 || (i > 0 && !c.files.length) || (i === 3 && c.step !== 3)} onClick={() => changeStep(i)}><span className="step-number">{i < c.step ? <Icon name="check" size={14}/> : i+1}</span><span>{label}</span></button>{i < 3 && <Icon name="arrow" size={14}/>}</li>)}</ol>
      <div className="conversion-layout"><div className="stage"><div className="stage-heading"><h2>{titles[c.step]}</h2><p>{descriptions[c.step]}</p></div>{c.step === 0 ? <ImportStep c={c}/> : c.step === 1 ? <ConfigureStep c={c}/> : c.step === 2 ? <ConvertStep c={c}/> : <ReviewStep c={c}/>}</div><Summary c={c}/></div>
      <div className="action-bar"><p role="status">{c.picker === 'folder' ? 'Selecciona una carpeta de salida.' : c.stopping ? 'Cancelando la conversión. Los archivos restantes quedan pendientes.' : c.step === 0 ? 'Añade un archivo para continuar.' : c.step === 1 ? 'Tus ajustes están listos.' : c.step === 2 ? 'Puedes cancelar la conversión; los archivos sin terminar quedan pendientes.' : reviewResult ? reviewResult.report.outputPath.endsWith('.fig') ? 'Importa el .fig desde el explorador de archivos de Figma.' : 'Importa el paquete mediante el plugin de Figma.' : 'Revisa los errores e inténtalo de nuevo.'}</p><div className="action-buttons">
        {c.step === 0 && <Button primary disabled={!c.files.length || c.busy} onClick={() => c.setStep(1)}>Continuar<Icon name="arrow" size={16}/></Button>}
        {c.step === 1 && <><Button disabled={c.busy} onClick={() => c.setStep(0)}>Volver</Button><Button primary disabled={!native || c.busy || !c.pending.length} onClick={() => c.convert(c.pending)}>{c.picker ? 'Seleccionando carpeta…' : 'Iniciar conversión'}</Button></>}
        {c.step === 2 && <Button disabled={c.stopping || Boolean(c.picker)} onClick={c.stopBatch}>{c.stopping ? 'Cancelando…' : 'Cancelar conversión'}</Button>}
        {c.step === 3 && <><Button disabled={c.busy} onClick={c.newSession}>Nueva conversión</Button>{reviewResult ? <Button primary icon="folder" onClick={() => c.reveal(reviewResult.report.outputPath)}>Abrir carpeta de resultados</Button> : <Button primary disabled={!native || c.busy || !c.pending.length} onClick={() => c.convert(c.pending)}>Reintentar lote</Button>}</>}
      </div></div></>}
    </main><LogDrawer c={c}/>
    {c.dragging && <div className="drag-overlay" aria-live="polite"><Icon name="upload" size={40}/><h2>Suelta tus archivos .xd para añadirlos</h2></div>}
  </div>;
}
createRoot(document.getElementById('root')).render(<App/>);
