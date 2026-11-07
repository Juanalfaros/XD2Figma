import { defineConfig } from '@playwright/test';
import { existsSync } from 'node:fs';
const edge = '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge';
export default defineConfig({
  testDir: './tests',
  testMatch: '*.spec.js',
  fullyParallel: true,
  use: { baseURL: 'http://127.0.0.1:1420', viewport: { width: 1200, height: 820 },
    launchOptions: existsSync(edge) ? { executablePath: edge } : {},
  },
  webServer: { command: 'pnpm dev', url: 'http://127.0.0.1:1420', reuseExistingServer: !process.env.CI },
});
