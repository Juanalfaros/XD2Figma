import React from 'react';
import Icon from '../Icon';
import { basename, native } from '../useConversion';
import { Badge } from './common';

export function LogDrawer({ c }) {
  return c.drawerOpen && <section className="log-drawer" aria-label="Registro de sesión"><header><h2><Icon name="log" size={17}/>Registro de la sesión <Badge>{c.logs.length}</Badge></h2><div className="inline"><button className="plain" disabled={!native || !c.logs.length} onClick={c.exportLog}>Exportar</button><button className="plain" disabled={c.busy || !c.logs.length} onClick={() => c.setLogs([])}>Limpiar registro</button><button className="icon-button" aria-label="Cerrar registro" onClick={() => c.setDrawerOpen(false)}><Icon name="close" size={18}/></button></div></header><div className="log-entries" role="log" aria-live="polite">{!c.logs.length ? <p>Sin advertencias durante esta sesión.</p> : <ol>{c.logs.map((l,i) => <li key={i} className={l.level}><Icon name={l.level === 'error' ? 'close' : 'warning'} size={16}/><div><strong>{basename(l.path)}</strong><p>{l.message}</p></div></li>)}</ol>}</div></section>;
}
