// Unit-Tests der Textregel im Browser (R00012 F0048 „Textregel im Browser“, B0246; dieselben Fälle wie die
// Textregel des Servers in R00011 F0041, src/gespraeche/textregel.rs; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { textPruefen } from '../../web/textregel.js';

test('die Grenze liegt bei 16 384 Bytes UTF-8 (Mutanten 16 383/16 385 und „Zeichen statt Bytes“ werden rot)', () => {
  assert.deepEqual(textPruefen('a'.repeat(16384)), { art: 'ok', bytes: 16384 });
  assert.deepEqual(textPruefen('a'.repeat(16385)), { art: 'laenge', bytes: 16385 });
  assert.deepEqual(textPruefen(`${'€'.repeat(5461)}a`), { art: 'ok', bytes: 16384 });
  assert.deepEqual(textPruefen('€'.repeat(5462)), { art: 'laenge', bytes: 16386 });
});

test('verbotene Zeichen mit Stelle ab 0 und Schreibweise; \\r nur vor \\n', () => {
  assert.deepEqual(textPruefen('ab\x1b'), { art: 'zeichen', bytes: 3, stelle: 2, zeichen: 'U+001B' });
  assert.deepEqual(textPruefen('a\rb'), { art: 'zeichen', bytes: 3, stelle: 1, zeichen: 'U+000D' });
  assert.deepEqual(textPruefen('ab\r'), { art: 'zeichen', bytes: 3, stelle: 2, zeichen: 'U+000D' });
  assert.equal(textPruefen('a\r\nb').art, 'ok');
  assert.equal(textPruefen('👩‍💻').art, 'ok');
});

test('jede verbotene Klasse und ihre Grenzen wie beim Server', () => {
  const faelle = [
    ['\u0000', 'U+0000'], ['\u0008', 'U+0008'], ['\u000b', 'U+000B'], ['\u000c', 'U+000C'], ['\u001f', 'U+001F'],
    ['\u007f', 'U+007F'], ['\u0085', 'U+0085'], ['\u009f', 'U+009F'],
  ];
  for (const [einzeln, schreibweise] of faelle) {
    const ergebnis = textPruefen(`ab${einzeln}`);
    assert.equal(ergebnis.art, 'zeichen', schreibweise);
    assert.equal(ergebnis.zeichen, schreibweise);
    assert.equal(ergebnis.stelle, 2);
  }
  for (const einzeln of ['\t', '\n', ' ', '~', ' ', '‮']) {
    assert.equal(textPruefen(`ab${einzeln}`).art, 'ok', JSON.stringify(einzeln));
  }
});

test('die Stelle zählt Zeichen, nicht Bytes und nicht UTF-16-Einheiten', () => {
  assert.equal(textPruefen('€€\x1b').stelle, 2);
  assert.equal(textPruefen('😀\x1b').stelle, 1);
});

test('nur Leerraum ist leer und geht vor Länge', () => {
  assert.equal(textPruefen('').art, 'leer');
  assert.equal(textPruefen(' \n\t ').art, 'leer');
  assert.equal(textPruefen(' '.repeat(16385)).art, 'leer');
});
