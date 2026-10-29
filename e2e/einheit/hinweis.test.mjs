// Unit-Tests der Hinweiszeile unter dem Feld (R00012 F0048 „Hinweis“, B0247; ADR-0038 Regel 4; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { hinweis, hinweisAnsagen } from '../../web/hinweis.js';
import { textPruefen } from '../../web/textregel.js';

const fuer = (text) => hinweis(textPruefen(text));

test('unter 15.000 Bytes leer, ab 15.000 „noch n Zeichen“ (Mutanten Schwelle 14.999 und 15.001 werden rot)', () => {
  assert.deepEqual(fuer('a'.repeat(14999)), { stufe: 'leer', text: '' });
  assert.deepEqual(fuer('a'.repeat(15000)), { stufe: 'zaehlen', text: 'noch 1.384 Zeichen' });
  assert.deepEqual(fuer('a'.repeat(15001)), { stufe: 'zaehlen', text: 'noch 1.383 Zeichen' });
  assert.deepEqual(fuer('a'.repeat(16384)), { stufe: 'zaehlen', text: 'noch 0 Zeichen' });
});

test('über 16.384 Bytes und bei einem Steuerzeichen ein Fehler mit ⚠, Stelle ab 1', () => {
  assert.deepEqual(fuer('a'.repeat(16385)), { stufe: 'fehler', text: '⚠ Zu lang: 16.385 von höchstens 16.384 Byte — bitte teilen.' });
  assert.deepEqual(fuer('rot \x1b[31m'), { stufe: 'fehler', text: '⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.' });
});

test('ein leeres Feld hat keinen Hinweis', () => {
  assert.deepEqual(fuer(''), { stufe: 'leer', text: '' });
});

test('angesagt wird nur beim Überschreiten der Schwelle und beim Wechsel in einen Fehler, nicht je Taste', () => {
  const leer = fuer('a');
  const zaehlen = fuer('a'.repeat(15000));
  const weiterZaehlen = fuer('a'.repeat(15010));
  const zuLang = fuer('a'.repeat(16390));
  const nochLaenger = fuer('a'.repeat(16400));

  assert.equal(hinweisAnsagen(leer, zaehlen), true);
  assert.equal(hinweisAnsagen(zaehlen, weiterZaehlen), false);
  assert.equal(hinweisAnsagen(weiterZaehlen, zuLang), true);
  assert.equal(hinweisAnsagen(zuLang, nochLaenger), false);
  assert.equal(hinweisAnsagen(zuLang, zaehlen), false);
  assert.equal(hinweisAnsagen(zaehlen, leer), false);
  assert.equal(hinweisAnsagen(leer, fuer('rot \x1b')), true);
});

// R00025 F0086 „Leerraum“ (B0405): ein Text nur aus Leerraum zeigt keinen Hinweis, gleich wie viele Bytes — nie eine
// negative Zahl; mit einem Zeichen gilt die Regel wie sonst (Mutante „leer wie ok“ wird rot).
test('Leerraum zeigt keinen Hinweis und nie eine negative Zahl', () => {
  assert.deepEqual(fuer(' '.repeat(17000)), { stufe: 'leer', text: '' });
  assert.deepEqual(fuer(' '.repeat(14999)), { stufe: 'leer', text: '' });
  assert.deepEqual(fuer(`${' '.repeat(17000)}a`), { stufe: 'fehler', text: '⚠ Zu lang: 17.001 von höchstens 16.384 Byte — bitte teilen.' });
  assert.deepEqual(fuer(`${' '.repeat(15000)}a`), { stufe: 'zaehlen', text: 'noch 1.383 Zeichen' });
  assert.deepEqual(fuer('\n　'.repeat(8000)), { stufe: 'leer', text: '' });
});
