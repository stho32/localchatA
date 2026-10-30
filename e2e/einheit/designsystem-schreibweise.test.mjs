// Schreibweise des Designsystems (Karte #82, Rechtschreib-Review #68/#78): Tastenname „Umschalt“, keine inneren
// Anführungszeichen „…„…“…“, kein „—, darunter“, Antwortzeile der Eingabe ohne ↩ und ✕ (node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const wurzel = fileURLToPath(new URL('../../Dokumentation/Wireframes/designsystem/', import.meta.url));
const lesen = (pfad) => readFileSync(wurzel + pfad, 'utf8');
const zaehlen = (text, muster) => (text.match(muster) ?? []).length;

test('1-tastatur.md: kein „Shift“, mindestens neunmal „Umschalt“', () => {
  const text = lesen('1-tastatur.md');
  assert.equal(zaehlen(text, /Shift/g), 0);
  assert.ok(zaehlen(text, /Umschalt/g) >= 9, 'Umschalt zu selten');
});

test('Eingabe/README.md: Tastenname „Umschalt Enter“, Antwortzeile wie R00047 ohne ↩ und ✕', () => {
  const text = lesen('components/Eingabe/README.md');
  assert.equal(zaehlen(text, /Shift/g), 0);
  assert.equal(zaehlen(text, /✕/g), 0);
  assert.equal(zaehlen(text, /↩/g), 0);
  assert.match(text, /„Antwort an <Kennung>:“/);
  assert.match(text, /Knopf „Abbrechen“/);
});

test('Vorschauen zeigen „Umschalt“ statt „Shift“', () => {
  for (const pfad of ['vorschau/Hauptansicht.html', 'vorschau/Eingabe.html', 'vorschau/ScreenFlow.html',
    'components/Hauptansicht/preview.html', 'components/Eingabe/preview.html', 'components/ScreenFlow/preview.html']) {
    assert.equal(zaehlen(lesen(pfad), /Shift/g), 0, pfad);
  }
});

test('Nachricht/README.md: Anführungszeichen innen ‚…‘, „—, darunter“ ohne Komma', () => {
  const text = lesen('components/Nachricht/README.md');
  assert.equal(zaehlen(text, /„[^“\n]*„/g), 0, 'innere „…“ statt ‚…‘');
  assert.equal(zaehlen(text, /—, darunter/g), 0);
});
