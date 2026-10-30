// Unit-Tests des Hochladens im Browser (R00030 F0101, node --test): wie eine Anmeldung ausgeht (B0450) und wie eine
// Abweisung angesagt wird.
import test from 'node:test';
import assert from 'node:assert/strict';
import { anmeldungBewerten, abweisungAnsage } from '../../web/anhaenge.js';

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
