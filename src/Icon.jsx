import React from 'react';
import warning from './assets/icons/warning.svg?no-inline';

export default function Icon({ name, size = 20, className = '' }) {
  const paths = {
    file: <><path d="M6 3h8l4 4v14H6z"/><path d="M14 3v5h4M9 13h6m-6 4h6"/></>,
    upload: <><path d="M12 16V4m-5 5 5-5 5 5"/><path d="M4 15v5h16v-5"/></>,
    arrow: <path d="M5 12h14m-6-6 6 6-6 6"/>,
    back: <path d="M19 12H5m6-6-6 6 6 6"/>,
    settings: <><path d="M4 7h16M4 17h16"/><circle cx="9" cy="7" r="2" fill="currentColor"/><circle cx="15" cy="17" r="2" fill="currentColor"/></>,
    folder: <path d="M3 6h7l2 2h9v11H3z"/>,
    layers: <><path d="m12 3 9 5-9 5-9-5z"/><path d="m3 12 9 5 9-5m-18 4 9 5 9-5"/></>,
    close: <path d="m6 6 12 12M18 6 6 18"/>,
    check: <path d="m5 12 5 5 9-10"/>,
    clock: <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
    retry: <><path d="M4 10a8 8 0 1 1 1 7M4 4v6h6"/></>,
    shield: <><path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6z"/><path d="m8 12 3 3 5-6"/></>,
    log: <><path d="M5 4h14v16H5zM9 8h6m-6 4h6m-6 4h4"/></>,
    chevron: <path d="m9 5 7 7-7 7"/>,
  };
  if (name === 'warning') return <span className={`vector-icon ${className}`} style={{ width: size, height: size, maskImage: `url(${JSON.stringify(warning)})`, WebkitMaskImage: `url(${JSON.stringify(warning)})` }} aria-hidden="true"/>;
  if (name === 'spinner') return <span className={`spinner ${className}`} style={{ width: size, height: size }} aria-hidden="true"/>;
  return <svg className={className} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>;
}
