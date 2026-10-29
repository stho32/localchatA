// Unit-Tests der Gliederung des Verlaufs (R00007 F0029 B0154, F0030 B0158, B0159; node --test). Die Zeiten
// tragen den Versatz des Servers; Uhrzeit und Datum folgen ihm, nie der Zeitzone des Testprozesses.
import test from 'node:test';
import assert from 'node:assert/strict';
import { uhrzeit, zeitTitel, gliederung, folgeGliedern, datumBenennen } from '../../web/gliederung.js';

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

const nachricht = (von, zeit) => ({ von, zeit, text: 'x' });
const ALPHA = 'alpha@127.0.0.2';
const BETA = 'beta@127.0.0.3';

test('Gruppierung als Spanne: bis 5:00,000 nach der vorigen ohne Kopf, ab 5:00,001 mit', () => {
  const vorige = nachricht(ALPHA, '2026-10-27T14:00:00.000+01:00');

  assert.deepEqual(gliederung(vorige, nachricht(ALPHA, '2026-10-27T14:05:00.000+01:00')), { kopf: false, tagestrenner: null });
  assert.deepEqual(gliederung(vorige, nachricht(ALPHA, '2026-10-27T14:05:00.001+01:00')), { kopf: true, tagestrenner: null });
  assert.deepEqual(gliederung(vorige, nachricht(ALPHA, '2026-10-27T14:04:00.000+01:00')), { kopf: false, tagestrenner: null });
  assert.deepEqual(gliederung(vorige, nachricht(ALPHA, '2026-10-27T14:06:00.000+01:00')), { kopf: true, tagestrenner: null });
});

test('anderer Absender, neuer Tag oder keine vorige: mit Kopf', () => {
  assert.deepEqual(gliederung(nachricht(ALPHA, '2026-10-27T14:00:00.000+01:00'), nachricht(BETA, '2026-10-27T14:01:00.000+01:00')),
    { kopf: true, tagestrenner: null });
  assert.deepEqual(gliederung(nachricht(ALPHA, '2026-10-27T23:59:00.000+01:00'), nachricht(ALPHA, '2026-10-28T00:01:00.000+01:00')),
    { kopf: true, tagestrenner: '2026-10-28' });
  assert.deepEqual(gliederung(null, nachricht(ALPHA, '2026-10-27T14:00:00.000+01:00')), { kopf: true, tagestrenner: '2026-10-27' });
});

// R00020 F0067 „Erste neue mit Kopf“ (B0331): die erste gezählte neue trägt ihren Kopf, auch wenn die Gruppierung
// ihn weglassen würde; die folgenden gruppieren wie bisher.
test('Folge mit erster neuer: sie bekommt den Kopf trotz Gruppe, die zweite nicht', () => {
  const vorige = nachricht(BETA, '2026-10-28T15:00:00.000+01:00');
  const erste = nachricht(BETA, '2026-10-28T15:01:00.000+01:00');
  const zweite = nachricht(BETA, '2026-10-28T15:01:10.000+01:00');

  assert.deepEqual(folgeGliedern(vorige, [erste, zweite], erste.zeit), [
    { kopf: true, tagestrenner: null },
    { kopf: false, tagestrenner: null },
  ]);
  assert.deepEqual(folgeGliedern(vorige, [erste, zweite], null), [
    { kopf: false, tagestrenner: null },
    { kopf: false, tagestrenner: null },
  ]);
  assert.deepEqual(folgeGliedern(null, [erste, zweite], zweite.zeit), [
    { kopf: true, tagestrenner: '2026-10-28' },
    { kopf: true, tagestrenner: null },
  ]);
});

test('die Zeitspanne wird als Zeitpunkt gerechnet, über die Zeitumstellung', () => {
  const vorige = nachricht(ALPHA, '2026-10-25T02:58:00.000+02:00');
  assert.deepEqual(gliederung(vorige, nachricht(ALPHA, '2026-10-25T02:02:00.000+01:00')), { kopf: false, tagestrenner: null });
});

test('Datum benennen nach dem heutigen Datum des Servers', () => {
  const heute = '2026-10-28';
  assert.equal(datumBenennen('2026-10-28', heute), 'Heute');
  assert.equal(datumBenennen('2026-10-27', heute), 'Gestern');
  assert.equal(datumBenennen('2026-10-26', heute), 'Montag, 26. Oktober 2026');
  assert.equal(datumBenennen('2026-10-24', heute), 'Samstag, 24. Oktober 2026');
  assert.equal(datumBenennen('2026-02-28', '2026-03-01'), 'Gestern');
  assert.equal(datumBenennen('2025-12-31', '2026-01-01'), 'Gestern');
  assert.equal(datumBenennen('2026-10-28', null), 'Mittwoch, 28. Oktober 2026');
});
