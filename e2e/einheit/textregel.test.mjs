// Unit-Tests der Textregel im Browser (R00012 F0048 „Textregel im Browser“, B0246; dieselben Fälle wie die
// Textregel des Servers in R00011 F0041, src/gespraeche/textregel.rs; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { textPruefen, nurLeerraum, steuerzeichenEntfernen } from '../../web/textregel.js';

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

// R00020 F0071 „Leerraum wie beim Server“ (B0351; Abnahme-Probe aus ADR-0038 Ergänzung Karte #31): leer ist, was nur
// aus Zeichen der Unicode-Eigenschaft White_Space besteht — dieselbe Klasse wie char::is_whitespace des Servers.
test('nur Leerraum: White_Space ist leer, U+FEFF und U+200B sind es nicht', () => {
  for (const leer of ['', ' \n\t', '\u00A0', '\u0085', '\u3000']) {
    assert.equal(nurLeerraum(leer), true, JSON.stringify(leer));
  }
  for (const nichtLeer of ['\uFEFF', '\u200B', 'a']) {
    assert.equal(nurLeerraum(nichtLeer), false, JSON.stringify(nichtLeer));
  }
});

// R00025 F0086 „Steuerzeichen entfernen“ (B0403): nur die verbotenen Zeichen der Textregel, nicht ganze ANSI-Folgen;
// ein Wagenrücklauf vor einem Zeilenumbruch bleibt (Mutante „Wagenrücklauf vor Umbruch entfernt“ wird rot).
test('steuerzeichenEntfernen nimmt nur die verbotenen Zeichen und zählt sie', () => {
  const faelle = [
    ['rot \u001b[31mFehler\u001b[0m', 'rot [31mFehler[0m', 2],
    ['a\rb', 'ab', 1],
    ['a\r\nb', 'a\r\nb', 0],
    ['a\tb', 'a\tb', 0],
    ['👩‍💻', '👩‍💻', 0],
    ['\u0000\u007f\u0085x\r', 'x', 4],
  ];
  for (const [vorher, nachher, entfernt] of faelle) {
    assert.deepEqual(steuerzeichenEntfernen(vorher), { text: nachher, entfernt }, JSON.stringify(vorher));
    assert.notEqual(textPruefen(nachher).art, 'zeichen', JSON.stringify(nachher));
  }
});
