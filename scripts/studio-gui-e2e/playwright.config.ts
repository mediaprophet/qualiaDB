import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: process.env.QUALIA_PORTAL_URL ?? 'http://127.0.0.1:8080',
    trace: 'on-first-retry',
  },
});