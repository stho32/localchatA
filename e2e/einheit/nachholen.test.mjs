// Unit-Tests des Nachholens nach dem Wiederverbinden (R00012 F0047 „Nachholen“, B0243; ADR-0037 Regel 7; node --test)
// mit einem falschen Port statt der API.
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachholen } from '../../web/nachholen.js';

const nachricht = (nummer) => ({ zeit: `z${nummer}`, von: 'beta@127.0.0.1', art: 'agent', text: `n${nummer}` });

// Liefert je Abruf den nächsten der vorbereiteten Blöcke und merkt sich die Anfragen.
function falscherPort(bloecke) {
  const anfragen = [];
  const abrufen = async (cursor, n) => {
    anfragen.push({ cursor, n });
    return { art: 'block', block: bloecke[anfragen.length - 1] };
  };
  return { abrufen, anfragen };
}

const reihe = (von, anzahl) => Array.from({ length: anzahl }, (_, index) => nachricht(von + index));

test('200 mit neuer: true, dann 5 mit neuer: false → 205 nachgeholt in zwei Abrufen zu n=200 ab der jüngsten Zeit', async () => {
  const erster = reihe(11, 200);
  const zweiter = reihe(211, 5);
  const port = falscherPort([
    { nachrichten: erster, aelter: true, neuer: true },
    { nachrichten: zweiter, aelter: true, neuer: false },
  ]);

  const ergebnis = await nachholen('z10', port.abrufen);

  assert.equal(ergebnis.art, 'nachgeholt');
  assert.equal(ergebnis.nachrichten.length, 205);
  assert.deepEqual(ergebnis.nachrichten, [...erster, ...zweiter]);
  assert.deepEqual(port.anfragen, [{ cursor: { nach: 'z10' }, n: 200 }, { cursor: { nach: 'z210' }, n: 200 }]);
});

test('liefert der Port immer neuer: true → genau 5 Abrufe, dann neu öffnen (Mutanten 4 und 6 Blöcke werden rot)', async () => {
  const port = falscherPort(Array.from({ length: 7 }, (_, block) => ({ nachrichten: reihe(block * 200, 200), aelter: true, neuer: true })));

  const ergebnis = await nachholen('z0', port.abrufen);

  assert.deepEqual(ergebnis, { art: 'neu' });
  assert.equal(port.anfragen.length, 5);
});

test('ein fünfter Block mit neuer: false ist noch nachgeholt', async () => {
  const bloecke = Array.from({ length: 5 }, (_, block) => ({ nachrichten: reihe(block * 200, 200), aelter: true, neuer: block < 4 }));
  const port = falscherPort(bloecke);

  const ergebnis = await nachholen('z0', port.abrufen);

  assert.equal(ergebnis.art, 'nachgeholt');
  assert.equal(ergebnis.nachrichten.length, 1000);
});

test('nichts verpasst → nachgeholt mit 0 Nachrichten nach einem Abruf', async () => {
  const port = falscherPort([{ nachrichten: [], aelter: true, neuer: false }]);

  const ergebnis = await nachholen('z9', port.abrufen);

  assert.deepEqual(ergebnis, { art: 'nachgeholt', nachrichten: [] });
  assert.equal(port.anfragen.length, 1);
});

test('ohne jüngste Zeit (leerer Channel) wird ab dem Anfang aller Zeiten geholt', async () => {
  const port = falscherPort([{ nachrichten: reihe(1, 3), aelter: false, neuer: false }]);

  const ergebnis = await nachholen(null, port.abrufen);

  assert.equal(ergebnis.nachrichten.length, 3);
  assert.deepEqual(port.anfragen[0].cursor, { nach: '1970-01-01T00:00:00.000+00:00' });
});

test('ein Fehler des Ports ist ein Fehler, kein Teilergebnis', async () => {
  const abrufen = async () => ({ art: 'fehler', status: 0 });

  assert.deepEqual(await nachholen('z1', abrufen), { art: 'fehler' });
});
