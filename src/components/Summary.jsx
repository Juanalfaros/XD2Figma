import React from 'react';
import Icon from '../Icon';
import { basename } from '../useConversion';

export function Summary({ c }) {
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
