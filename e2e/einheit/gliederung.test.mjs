// Unit-Tests der Gliederung des Verlaufs (R00007 F0029 B0154, F0030 B0158, B0159; node --test). Die Zeiten
// tragen den Versatz des Servers; Uhrzeit und Datum folgen ihm, nie der Zeitzone des Testprozesses.
import test from 'node:test';
import assert from 'node:assert/strict';
import { uhrzeit, zeitTitel } from '../../web/gliederung.js';

test('Uhrzeit HH:MM im Versatz der Nachricht, auch über die Zeitumstellung', () => {
  assert.equal(uhrzeit('2026-10-24T23:30:00.000+02:00'), '23:30');
  assert.equal(uhrzeit('2026-10-28T09:05:59.999+01:00'), '09:05');
});

test('der title nennt Datum und Uhrzeit mit Sekunden', () => {
  assert.equal(zeitTitel('2026-10-28T09:05:59.999+01:00'), '28.10.2026, 09:05:59');
});

test('Uhrzeit hängt nicht an der Zeitzone des Prozesses', async () => {
  const { execFileSync } = await import('node:child_process');
  const skript = "import('../../web/gliederung.js').then((m) => console.log(m.uhrzeit('2026-10-24T23:30:00.000+02:00'), m.uhrzeit('2026-10-28T09:05:59.999+01:00')))";
  for (const zone of ['Asia/Tokyo', 'America/New_York']) {
    const ausgabe = execFileSync(process.execPath, ['--input-type=module', '-e', skript], {
      cwd: new URL('.', import.meta.url), env: { ...process.env, TZ: zone },
    }).toString().trim();
    assert.equal(ausgabe, '23:30 09:05', zone);
  }
});
