import { defineConfig } from 'vite';

export default defineConfig({
  server: { port: 1420, strictPort: true },
  clearScreen: false,
  // La CSP de la app no permite imágenes data:, así que ningún recurso se incrusta.
  build: { assetsInlineLimit: 0 },
});
