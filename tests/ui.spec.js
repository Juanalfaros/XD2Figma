import { test, expect } from '@playwright/test';

// Exercise real React state and Tauri Channel messages without touching user files.
async function desktop(page, settings = {}) {
  await page.addInitScript(({ cancelFolder = false, failFirst = false } = {}) => {
    window.isTauri = true;
    window.qa = { calls: [], revealed: [], cancelFolder, failFirst, failures: 0, listeners: {}, callbacks: {}, next: 0 };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback(callback) { const id = ++window.qa.next; window.qa.callbacks[id] = callback; return id; },
      unregisterCallback(id) { delete window.qa.callbacks[id]; },
      async invoke(command, args) {
        if (command === 'plugin:event|listen') { window.qa.listeners[args.event] = args.handler; return args.handler; }
        if (command === 'plugin:dialog|open') return args.options.directory ? (window.qa.cancelFolder ? null : '/qa/results') : ['/qa/portfolio ui.xd', '/qa/Erothys Dark UI.xd'];
        if (command === 'reveal_output') { window.qa.revealed.push(args.path); return; }
        if (command === 'convert_file') {
          window.qa.calls.push({ source: args.source, options: args.options });
          args.progress.onmessage({ percent: 68, phase: 'Mapeando nodos', element: 'Artboard / Máscara', level: 'info' });
          await new Promise(r => setTimeout(r, 500));
          if (window.qa.failFirst && window.qa.failures++ === 0) throw 'Recurso de imagen ilegible';
          args.progress.onmessage({ percent: 90, phase: 'Generando .fig', element: '', message: 'Fuente no disponible', level: 'warning' });
          return { outputPath: `/qa/results/${args.source.split('/').pop().replace('.xd', args.options.importPackage ? '.zip' : '.fig')}`, imagesPath: args.options.extractImages ? '/qa/results/images' : null, summary: { artboards: 12, layers: 1284, warnings: ['Fuente no disponible'] } };
        }
        return null;
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  }, settings);
  await page.goto('/');
}

test('guided batch, real progress events, warning deduplication and result history', async ({ page }) => {
  const errors = []; page.on('pageerror', e => errors.push(e.message));
  await desktop(page);
  await expect(page.getByRole('heading', { name: 'Empieza con tus archivos XD.' })).toBeVisible();
  await page.screenshot({ path: 'test-results/dashboard-dark.png' });
  await page.getByRole('button', { name: 'Seleccionar archivos' }).click();
  await expect(page.getByRole('heading', { name: 'Prepara tu conversión.' })).toBeVisible();
  await page.getByRole('button', { name: 'Iniciar conversión' }).click();
  await expect(page.getByRole('progressbar')).toHaveAttribute('value', '68');
  await expect(page.getByRole('heading', { name: 'Revisa tu resultado.' })).toBeVisible();
  expect(await page.evaluate(() => window.qa.calls.length)).toBe(2);
  await page.getByRole('button', { name: 'Abrir carpeta de resultados' }).click();
  expect(await page.evaluate(() => window.qa.revealed.at(-1))).toMatch(/\.fig$/);
  await page.getByRole('button', { name: 'Nueva conversión' }).click();
  await page.getByRole('button', { name: /Resultados/ }).click();
  await expect(page.getByRole('button', { name: /portfolio ui.fig/ })).toBeVisible();
  await page.getByRole('button', { name: /Registro/ }).click();
  await expect(page.locator('.log-entries li')).toHaveCount(2);
  expect(await page.locator('.warning-box .vector-icon').evaluate(e => getComputedStyle(e).maskImage)).not.toBe('none');
  expect(errors).toEqual([]);
});

test('canceling destination returns to configuration without starting a conversion', async ({ page }) => {
  await desktop(page, { cancelFolder: true });
  await page.getByRole('button', { name: 'Seleccionar archivos' }).click();
  await page.getByRole('button', { name: 'Iniciar conversión' }).click();
  await expect(page.getByRole('button', { name: 'Iniciar conversión' })).toBeEnabled();
  expect(await page.evaluate(() => window.qa.calls.length)).toBe(0);
  await expect(page.getByRole('heading', { name: 'Prepara tu conversión.' })).toBeVisible();
});

test('stop finishes active file and leaves the rest pending; retry recovers failures', async ({ page }) => {
  await desktop(page, { failFirst: true });
  await page.getByRole('button', { name: 'Seleccionar archivos' }).click();
  await page.getByRole('button', { name: 'Iniciar conversión' }).click();
  await page.getByRole('button', { name: 'Detener después de este' }).click();
  await expect(page.getByRole('heading', { name: 'Revisa tu resultado.' })).toBeVisible();
  expect(await page.evaluate(() => window.qa.calls.length)).toBe(1);
  await page.getByRole('button', { name: 'Cerrar registro' }).click();
  await page.getByRole('button', { name: 'Reintentar archivo' }).click();
  await expect(page.getByRole('heading', { name: 'Revisa tu resultado.' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'portfolio ui.fig' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Convertir archivo pendiente' })).toBeVisible();
  expect(await page.evaluate(() => window.qa.calls.length)).toBe(2);
});

test('preferences persist, output options reach conversion, and narrow light theme fits', async ({ page }) => {
  await desktop(page);
  await page.getByRole('button', { name: 'Preferencias' }).click();
  await page.getByLabel('Extraer imágenes').check();
  await page.getByRole('button', { name: 'Claro', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme','light');
  await page.screenshot({ path: 'test-results/dashboard-light-preferences.png' });
  await page.reload();
  await page.getByRole('button', { name: 'Preferencias' }).click();
  await expect(page.getByLabel('Extraer imágenes')).toBeChecked();
  await page.getByLabel('Formato de salida').selectOption('zip');
  await page.getByRole('button', { name: 'Volver a la conversión' }).click();
  await page.getByRole('button', { name: 'Seleccionar archivos' }).click();
  await page.getByRole('button', { name: 'Iniciar conversión' }).click();
  await expect(page.getByRole('heading', { name: 'Revisa tu resultado.' })).toBeVisible();
  expect(await page.evaluate(() => window.qa.calls[0].options)).toMatchObject({ extractImages: true, importPackage: true });
  await page.setViewportSize({ width: 760, height: 560 });
  await page.screenshot({ path: 'test-results/dashboard-light-narrow.png' });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
