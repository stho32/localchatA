'use strict';
// Vor jedem Lauf bauen, damit nie gegen eine veraltete eingebettete Oberfläche getestet wird; danach einmal je Lauf
// den Channel `gross` (100.000 Nachrichten über 7 Tage, R00007) mit dem Testdaten-Erzeuger anlegen — die Tests
// kopieren ihn in ihren Ordner (LOCALCHATA_E2E_GROSS).
const { execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

module.exports = function bauen() {
  const wurzel = path.resolve(__dirname, '..');
  execFileSync('cargo', ['build', '-q', '--bins', '--examples'], { cwd: wurzel, stdio: 'inherit' });
  const vorlage = path.join(os.tmpdir(), `localchata-e2e-gross-${process.pid}`);
  fs.rmSync(vorlage, { recursive: true, force: true });
  execFileSync(path.join(wurzel, 'target/debug/examples/verlauf-erzeugen'), [vorlage, 'gross', '100000', '7'], {
    stdio: 'ignore',
    env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: '2026-10-28T15:00:00.000+01:00' },
  });
  process.env.LOCALCHATA_E2E_GROSS = path.join(vorlage, 'channels', 'gross');
  return () => fs.rmSync(vorlage, { recursive: true, force: true });
};
