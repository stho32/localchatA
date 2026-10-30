// Unit-Tests des Hochladens im Browser (R00030 F0101, node --test): wie eine Anmeldung ausgeht (B0450) und wie eine
// Abweisung angesagt wird.
import test from 'node:test';
import assert from 'node:assert/strict';
import { anmeldungBewerten, abweisungAnsage, prozentAus, hochladenBewerten } from '../../web/anhaenge.js';

const antwort = (status, daten, wartenS = null) => ({ art: 'antwort', status, daten, wartenS });

test('201 mit Anhang → angemeldet mit ID', () => {
  assert.deepEqual(anmeldungBewerten(antwort(201, { anhang: { id: 'abc' } })), { art: 'angemeldet', id: 'abc' });
});

test('429 und 507 `hochladungen` → warten nach Retry-After', () => {
  assert.deepEqual(anmeldungBewerten(antwort(429, { fehler: 'x', grenze: 'offen_kennung' }, 5)), { art: 'warten', sekunden: 5 });
  assert.deepEqual(anmeldungBewerten(antwort(507, { fehler: 'x', regel: 'quote', grund: 'hochladungen' }, 30)), { art: 'warten', sekunden: 30 });
});

test('507 `voll`, 507 `platte`, 422 `zu_gross` und `name` → abgewiesen mit dem Text der Instanz', () => {
  for (const daten of [
    { fehler: 'voll', regel: 'quote', grund: 'voll' }, { fehler: 'platte', regel: 'platte' },
    { fehler: 'zu groß', regel: 'zu_gross' }, { fehler: 'name', regel: 'name' },
  ]) {
    const status = daten.regel === 'quote' || daten.regel === 'platte' ? 507 : 422;
    assert.deepEqual(anmeldungBewerten(antwort(status, daten)), { art: 'abgewiesen', grund: daten.fehler });
  }
});

test('ohne Text der Instanz der Status, ohne Antwort „ohneAntwort“', () => {
  assert.deepEqual(anmeldungBewerten(antwort(500, null)), { art: 'abgewiesen', grund: 'Nicht hochgeladen (Status 500).' });
  assert.deepEqual(anmeldungBewerten({ art: 'netzfehler' }), { art: 'ohneAntwort' });
});

test('Ansage der Abweisung: „Nicht hochgeladen: <name> — <Grund>“, das Präfix der Instanz nur einmal', () => {
  assert.equal(
    abweisungAnsage('foto.png', 'Nicht hochgeladen: der Platz für Anhänge ist voll.'),
    'Nicht hochgeladen: foto.png — der Platz für Anhänge ist voll.',
  );
  assert.equal(abweisungAnsage('a.txt', 'Der Name ist ungültig.'), 'Nicht hochgeladen: a.txt — Der Name ist ungültig.');
});

// R00030 „Fortschritt“ (B0451): die Fortschrittsrechnung, falls die Drosselung im E2E nicht greift.
test('Fortschritt in ganzen Prozent: abgerundet, ohne Länge 0, höchstens 100', () => {
  assert.equal(prozentAus(0, 2457600), 0);
  assert.equal(prozentAus(1228800, 2457600), 50);
  assert.equal(prozentAus(2457599, 2457600), 99);
  assert.equal(prozentAus(2457600, 2457600), 100);
  assert.equal(prozentAus(10, 0), 0);
  assert.equal(prozentAus(20, 10), 100);
});

test('Hochladen: 200 fertig, 422 `metadaten` abgewiesen, sonst fehlgeschlagen mit Grund; Abbruch bleibt Abbruch', () => {
  assert.deepEqual(hochladenBewerten(antwort(200, null)), { art: 'fertig' });
  assert.deepEqual(hochladenBewerten(antwort(422, { fehler: 'kaputt', regel: 'metadaten' })), { art: 'abgewiesen', grund: 'kaputt' });
  assert.deepEqual(hochladenBewerten(antwort(408, { fehler: 'Frist' })), { art: 'fehlgeschlagen', grund: 'Frist' });
  assert.deepEqual(hochladenBewerten(antwort(500, null)), { art: 'fehlgeschlagen', grund: 'Nicht hochgeladen (Status 500).' });
  assert.deepEqual(hochladenBewerten({ art: 'netzfehler' }), { art: 'fehlgeschlagen', grund: 'Keine Antwort vom Server.' });
  assert.deepEqual(hochladenBewerten({ art: 'abgebrochen' }), { art: 'abgebrochen' });
});
