import React from 'react';
import { Button } from './common';

export function Preferences({ c }) {
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
