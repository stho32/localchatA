'use strict';
// E2E gegen das echte Programm: jeder Test startet localchatA in einem eigenen Ordner (tests/hilfen.js).
const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests',
  globalSetup: require.resolve('./bauen.js'),
  timeout: 30000,
  reporter: [['list']],
  use: { headless: true, locale: 'de-DE', timezoneId: 'Europe/Berlin' },
});
