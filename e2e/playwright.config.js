'use strict';
// E2E gegen das echte Programm: jeder Test startet localchatA in einem eigenen Ordner (tests/hilfen.js).
// Der Direktlink auf Medien und der Player im Verlauf (R00030 F0165) laufen zusätzlich in Google Chrome stable (ADR-0014
// Ergänzung „Karte #71 (Chrome 154)“, C6): Chrome lädt Medien anders als Playwright-Chromium.
const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests',
  globalSetup: require.resolve('./bauen.js'),
  timeout: 30000,
  reporter: [['list']],
  use: { headless: true, locale: 'de-DE', timezoneId: 'Europe/Berlin', trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    {
      name: 'chrome-stabil',
      use: { browserName: 'chromium', channel: 'chrome' },
      testMatch: ['r00029-direktlink-spielt.spec.js', 'r00029-direktlink-kein-skript.spec.js', 'r00029-direktlink-fremdseite.spec.js', 'r00030-player-spielt.spec.js'],
    },
  ],
});
