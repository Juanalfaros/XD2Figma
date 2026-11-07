import { useEffect, useRef, useState } from 'react';
import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open } from '@tauri-apps/plugin-dialog';

export const native = isTauri();
export const basename = (path) => path?.split(/[/\\]/).pop() || path;
export const defaults = { keepComponents: true, preservePrototype: false, repeatGridAutoLayout: true, extractImages: false, strict: false, importPackage: false };
function saved() {
  try { return JSON.parse(localStorage.getItem('xd2figma-preferences') || '{}') || {}; } catch { return {}; }
}
export function useConversion() {
  const [theme, setTheme] = useState(() => {
    try { return ['light', 'dark', 'system'].includes(localStorage.getItem('xd2figma-theme')) ? localStorage.getItem('xd2figma-theme') : 'dark'; } catch { return 'dark'; }
  });
  const [options, setOptions] = useState(() => Object.fromEntries(Object.entries(defaults).map(([k, v]) => [k, typeof saved().options?.[k] === 'boolean' ? saved().options[k] : v])));
  const [destination, setDestination] = useState(() => typeof saved().destination === 'string' ? saved().destination : null);
  const [files, setFiles] = useState([]);
  const [history, setHistory] = useState([]);
  const [selected, setSelected] = useState(null);
  const [view, setView] = useState('conversion');
  const [step, setStep] = useState(0);
  const [running, setRunning] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [picker, setPicker] = useState(null);
  const [dragging, setDragging] = useState(false);
  const [error, setError] = useState('');
  const [logs, setLogs] = useState([]);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const lock = useRef(false);
  const stop = useRef(false);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (native) void getCurrentWindow().setTheme(theme === 'system' ? null : theme).catch(console.warn);
    try { localStorage.setItem('xd2figma-theme', theme); } catch {}
  }, [theme]);
  useEffect(() => {
    try { localStorage.setItem('xd2figma-preferences', JSON.stringify({ options, destination })); } catch {}
  }, [options, destination]);
  function enqueue(paths) {
    // A native conversion owns its batch until it finishes, including drag/drop.
    if (lock.current) { setError('Espera a que termine el lote para añadir archivos.'); return; }
    const valid = paths.filter(p => /\.xd$/i.test(p));
    setError(valid.length === paths.length ? '' : 'Solo se pueden añadir archivos .xd.');
    if (!valid.length) return;
    setFiles(current => [...current, ...[...new Set(valid)].filter(path => !current.some(f => f.path === path)).map(path => ({ path, status: 'waiting', percent: 0 }))]);
    setView('conversion'); setStep(1);
  }
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const subscription = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (disposed) return;
      setDragging(payload.type === 'enter' || payload.type === 'over');
      if (payload.type === 'drop') enqueue(payload.paths);
    });
    subscription.catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; subscription.then(off => off()).catch(() => {}); };
  }, []);
  async function chooseFiles() {
    if (!native || lock.current || picker) return;
    setPicker('files');
    try { const paths = await open({ multiple: true, filters: [{ name: 'Adobe XD', extensions: ['xd'] }] }); if (paths) enqueue(Array.isArray(paths) ? paths : [paths]); }
    catch (e) { setError(String(e)); } finally { setPicker(null); }
  }
  async function chooseDestination() {
    setPicker('folder');
    try { const path = await open({ directory: true, title: 'Seleccionar carpeta de salida' }); if (path) setDestination(path); return path; }
    catch (e) { setError(String(e)); return null; } finally { setPicker(null); }
  }
  function log(path, message, level = 'warning') {
    setLogs(current => [...current, { path, message, level, time: Date.now() }]);
    if (level === 'error') setDrawerOpen(true);
  }
  function update(path, fields) { setFiles(current => current.map(f => f.path === path ? { ...f, ...fields } : f)); }
  async function convert(batch) {
    if (!native || lock.current || picker || !batch.length) return;
    lock.current = true; stop.current = false;
    setRunning(true); setStopping(false); setError('');
    let started = false;
    try {
      const output = destination || await chooseDestination();
      if (!output) return;
      started = true; setStep(2); setView('conversion');
      const batchOptions = { ...options };
      for (const file of batch) {
        if (stop.current) break;
        update(file.path, { status: 'active', percent: 0, phase: 'Iniciando', element: '', message: '', report: undefined });
        const start = performance.now();
        try {
          const progress = new Channel(); const logged = new Set();
          progress.onmessage = event => {
            if (event.level === 'warning') { logged.add(event.message); log(file.path, event.message); }
            update(file.path, { percent: Math.max(0, Math.min(100, event.percent)), phase: event.phase, element: event.element });
          };
          const report = await invoke('convert_file', { source: file.path, destination: output, options: batchOptions, progress });
          for (const warning of report.summary.warnings) if (!logged.has(warning)) log(file.path, warning);
          const result = { ...file, status: 'done', percent: 100, report, elapsed: (performance.now() - start) / 1000, message: '' };
          update(file.path, result);
          setHistory(current => [...current.filter(f => f.path !== file.path), result]);
          setSelected(file.path);
        } catch (e) { update(file.path, { status: 'error', message: String(e) }); log(file.path, String(e), 'error'); }
      }
    } catch (e) { setError(String(e)); }
    finally { lock.current = false; setRunning(false); setStopping(false); if (started) setStep(3); }
  }
  const active = files.find(f => f.status === 'active');
  const pending = files.filter(f => f.status === 'waiting' || f.status === 'error');
  const result = history.find(f => f.path === selected) || history.at(-1);
  return { theme, setTheme, options, setOptions, destination, files, history, selected, setSelected, view, setView, step, setStep,
    running, stopping, picker, dragging, error, setError, logs, setLogs, drawerOpen, setDrawerOpen, active, pending, result,
    busy: running || Boolean(picker), chooseFiles, chooseDestination, convert,
    stopBatch() { stop.current = true; setStopping(true); },
    remove(path) { if (lock.current) return; setFiles(current => current.filter(f => f.path !== path)); },
    newSession() { if (lock.current) return; setFiles([]); setSelected(null); setStep(0); setView('conversion'); setError(''); },
    reveal(path) { invoke('reveal_output', { path }).catch(e => setError(String(e))); },
  };
}
