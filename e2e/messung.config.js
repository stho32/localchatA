'use strict';
// Messwerte M1 und M2 aus R00007 (Entwicklertest, kein Gate): npx playwright test --config messung.config.js
const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './messung',
  globalSetup: require.resolve('./bauen.js'),
  timeout: 600000,
  workers: 1,
  reporter: [['list']],
  use: { headless: true, locale: 'de-DE', timezoneId: 'Europe/Berlin', viewport: { width: 1280, height: 800 } },
});
