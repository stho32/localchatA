// Unit-Tests der Eingabe (R00011 F0043 „Tasten“, B0217; ADR-0038 Regel 2; R00027 „Bereit-Regel“, B0413; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { taste, sendbar } from '../../web/eingabe.js';

const ohne = { key: 'Enter', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, isComposing: false, keyCode: 13 };
const mit = (werte) => ({ ...ohne, ...werte });

test('Enter, Strg Enter und Meta Enter senden; Shift Enter macht eine neue Zeile', () => {
  assert.equal(taste(ohne, 'Hallo'), 'senden');
  assert.equal(taste(mit({ ctrlKey: true }), 'Hallo'), 'senden');
  assert.equal(taste(mit({ metaKey: true }), 'Hallo'), 'senden');
  assert.equal(taste(mit({ shiftKey: true }), 'Hallo'), 'zeile');
});

test('Alt Enter tut nichts', () => {
  assert.equal(taste(mit({ altKey: true }), 'Hallo'), null);
});

test('während einer IME-Komposition wird nie gesendet', () => {
  assert.equal(taste(mit({ isComposing: true }), 'Hallo'), null);
  assert.equal(taste(mit({ keyCode: 229 }), 'Hallo'), null);
});

test('Text nur aus Leerraum sendet nicht', () => {
  assert.equal(taste(ohne, ' \n '), null);
  assert.equal(taste(ohne, ''), null);
  assert.equal(taste(ohne, ' \t'), null);
});

// R00020 F0071 „Leerraum wie beim Server“ (B0351): Enter urteilt mit der Leer-Regel der Textregel, nicht mit trim().
test('Enter mit U+0085 sendet nicht, mit U+FEFF sendet es — wie der Server urteilt', () => {
  assert.equal(taste(ohne, '\u0085'), null);
  assert.equal(taste(ohne, '\uFEFF'), 'senden');
});

test('eine andere Taste tut nichts', () => {
  assert.equal(taste(mit({ key: 'a', keyCode: 65 }), 'Hallo'), null);
});

// R00027 F0091 „Bereit-Regel“ (B0413, ADR-0038 Ergänzung Karte #41 K2): eine Bedingung für Taste und Knopf. Mutanten
// „Leer-Prüfung mit trim()“ (rot bei U+0085 und U+FEFF) und „Textregel nicht geprüft“ (rot bei 16.385 Byte).
test('sendbar ist wahr bei Text, der senden würde', () => {
  assert.equal(sendbar('a'), true);
  assert.equal(sendbar('Hallo\nWelt'), true);
  assert.equal(sendbar('\uFEFF'), true);
  assert.equal(sendbar(`${'€'.repeat(5461)}a`), true);
});

test('sendbar ist falsch bei Leerraum nach White_Space und bei verletzter Textregel', () => {
  for (const text of ['', '   ', ' \n\t', '\u0085', '\u3000', 'a'.repeat(16385), 'rot \u001b[31m']) {
    assert.equal(sendbar(text), false, JSON.stringify(text.slice(0, 12)));
  }
});

// R00027 F0092 „Taste mit Zeigerart“ (B0483, ADR-0038 Ergänzung Karte #41 K5): mit grobem Zeiger macht Enter eine Zeile,
// Strg/Meta Enter senden; die Mutante „Zeigerart ignoriert“ wird rot.
test('grober Zeiger: Enter und Shift Enter machen eine Zeile, Strg Enter und Meta Enter senden', () => {
  assert.equal(taste(ohne, 'Hallo', 'grob'), 'zeile');
  assert.equal(taste(mit({ shiftKey: true }), 'Hallo', 'grob'), 'zeile');
  assert.equal(taste(mit({ ctrlKey: true }), 'a', 'grob'), 'senden');
  assert.equal(taste(mit({ metaKey: true }), 'a', 'grob'), 'senden');
});

// Strg sticht Shift: Strg Shift Enter sendet wie Strg Enter, mit feinem und mit grobem Zeiger (Code-Review #41 A2).
test('Strg Shift Enter sendet, mit feinem und mit grobem Zeiger', () => {
  for (const zeiger of ['fein', 'grob']) {
    assert.equal(taste(mit({ ctrlKey: true, shiftKey: true }), 'Hallo', zeiger), 'senden', zeiger);
  }
});

test('grober Zeiger: Strg Enter während einer Komposition oder bei Leerraum tut nichts', () => {
  assert.equal(taste(mit({ ctrlKey: true, isComposing: true }), 'a', 'grob'), null);
  assert.equal(taste(mit({ ctrlKey: true, keyCode: 229 }), 'a', 'grob'), null);
  assert.equal(taste(mit({ ctrlKey: true }), ' \n ', 'grob'), null);
});

test('grober Zeiger: Enter mit keyCode 229 tut nichts — die Zeile schreibt die Bildschirmtastatur', () => {
  assert.equal(taste(mit({ keyCode: 229 }), 'Hallo', 'grob'), null);
});

test('feiner Zeiger: alle Fälle aus R00011 „Tasten“ unverändert', () => {
  const faelle = [
    [ohne, 'Hallo', 'senden'], [mit({ ctrlKey: true }), 'Hallo', 'senden'], [mit({ metaKey: true }), 'Hallo', 'senden'],
    [mit({ shiftKey: true }), 'Hallo', 'zeile'], [mit({ altKey: true }), 'Hallo', null], [mit({ isComposing: true }), 'Hallo', null],
    [mit({ keyCode: 229 }), 'Hallo', null], [ohne, ' \n ', null], [ohne, '', null], [ohne, '\u0085', null], [ohne, '﻿', 'senden'],
    [mit({ key: 'a', keyCode: 65 }), 'Hallo', null],
  ];
  for (const [ereignis, text, erwartet] of faelle) {
    assert.equal(taste(ereignis, text, 'fein'), erwartet, JSON.stringify({ ...ereignis, text }));
  }
});

// R00030 F0101 „Bereit-Regel“ (B0449, wörtlich): sendbar("", 1 Datei) → wahr; sendbar("   ", 1 Datei) → wahr (Text wird
// ""); sendbar("", 0) → falsch; sendbar(16.385 × "a", 1 Datei) → falsch (Textregel); sendbar("a", 0) → wahr wie R00027.
// Die Mutante „Dateien ignoriert“ wird rot.
test('sendbar mit Dateien: leerer Text oder Leerraum reicht, die Textregel gilt weiter', () => {
  assert.equal(sendbar('', 1), true);
  assert.equal(sendbar('   ', 1), true);
  assert.equal(sendbar('', 0), false);
  assert.equal(sendbar('a'.repeat(16385), 1), false);
  assert.equal(sendbar('a', 0), true);
  assert.equal(sendbar('rot \u001b[31m', 3), false);
  assert.equal(sendbar(' \n\t', 10), true);
});

test('sendbar ohne Angabe der Dateien bleibt die Regel aus R00027', () => {
  assert.equal(sendbar(''), false);
  assert.equal(sendbar('a'), true);
});

// R00030 F0101 „Taste ohne Text“ (B0453): `taste` erhält die Zahl der Dateien — Enter (Desktop) bzw. Strg Enter (Tablet)
// sendet auch nur mit Dateien. Die Mutante „Leer-Prüfung ohne Dateien“ wird rot.
test('Taste mit Dateien: leeres Feld sendet mit Enter (fein) und Strg Enter (grob)', () => {
  assert.equal(taste(ohne, '', 'fein', 1), 'senden');
  assert.equal(taste(ohne, '  ', 'fein', 2), 'senden');
  assert.equal(taste(mit({ ctrlKey: true }), '', 'grob', 1), 'senden');
  assert.equal(taste(ohne, '', 'grob', 1), 'zeile');
  assert.equal(taste(ohne, '', 'fein', 0), null);
  assert.equal(taste(ohne, ''), null);
});
