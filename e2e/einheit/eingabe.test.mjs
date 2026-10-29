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
