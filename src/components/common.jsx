import React from 'react';
import Icon from '../Icon';
import { basename } from '../useConversion';
export const statuses = { waiting: 'En espera', active: 'Convirtiendo', done: 'Completado', error: 'Error' };
export const number = value => new Intl.NumberFormat('es-CL').format(value);

export function Button({ icon, children, primary, className = '', ...props }) {
  return <button className={`${primary ? 'primary' : 'secondary'} ${className}`} {...props}>{icon && <Icon name={icon} size={17}/>} {children}</button>;
}
export function Badge({ status, children }) { return <span className={`badge ${status || ''}`}>{children}</span>; }
export function FileRow({ file, c, removable = false }) {
  return <div className={`file-row ${file.status}`}>
    <Icon name="file" size={24}/><div className="file-info"><strong title={file.path}>{basename(file.path)}</strong><small>{file.status === 'error' ? file.message : file.report ? `${number(file.report.summary.artboards)} artboards · ${number(file.report.summary.layers)} capas` : file.status === 'active' ? file.phase : 'Archivo preparado'}</small></div>
    {removable ? <button className="icon-button" title="Quitar de la cola" aria-label={`Quitar ${basename(file.path)}`} disabled={c.busy} onClick={() => c.remove(file.path)}><Icon name="close" size={16}/></button> : <Badge status={file.status}>{statuses[file.status]}</Badge>}
  </div>;
}
