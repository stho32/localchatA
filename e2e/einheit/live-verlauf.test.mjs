// Unit-Tests der fremden Nachrichten im offenen Verlauf (R00011 F0044; ADR-0037 Regel 6, ADR-0028 Regel 5;
// node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { einordnen } from '../../web/live-verlauf.js';

const t = (sekunde) => `2026-10-28T12:00:${String(sekunde).padStart(2, '0')}.000+01:00`;
const n = (sekunde) => ({ zeit: t(sekunde), von: 'beta@127.0.0.1', art: 'agent', text: `n${sekunde}` });
const zeiten = (nachrichten) => nachrichten.map((nachricht) => nachricht.zeit);

// B0223 „Einordnen“: Block 8, 9, 10; vor dem Block gepuffert 9 und 11 → 11 kommt dazu; danach 10 und 12 → 12.
test('übernommen wird nur, was jünger ist als die jüngste bekannte Zeit', () => {
  const block = [n(8), n(9), n(10)];

  const nachDemBlock = einordnen(block[2].zeit, [n(9), n(11)]);
  const danach = einordnen(nachDemBlock.juengste, [n(10), n(12)]);

  assert.deepEqual(zeiten([...block, ...nachDemBlock.neue, ...danach.neue]), [8, 9, 10, 11, 12].map(t));
  assert.equal(danach.juengste, t(12));
});

test('ohne bekannte Zeit (leerer Channel) wird alles übernommen, gleiche Zeiten nur einmal', () => {
  const { neue, juengste } = einordnen(null, [n(3), n(3), n(4)]);

  assert.deepEqual(zeiten(neue), [t(3), t(4)]);
  assert.equal(juengste, t(4));
});

test('verglichen wird der Zeitpunkt, nicht der Text: 02:00+01:00 ist jünger als 02:59+02:00', () => {
  const sommer = '2026-10-25T02:59:59.999+02:00';
  const winter = { ...n(0), zeit: '2026-10-25T02:00:00.000+01:00' };

  assert.deepEqual(zeiten(einordnen(sommer, [winter]).neue), [winter.zeit]);
});
