// Unit-Tests des Platzhalters (R00030 F0184 B0798; node --test): die Fassung ist rein, die Breite kommt als Prüfung herein.
import test from 'node:test';
import assert from 'node:assert/strict';
import { platzhalterWaehlen } from '../../web/platzhalter.js';

const bis = (zeichen) => (text) => Array.from(text).length <= zeichen;

test('Platz genug → der volle Text', () => {
  assert.equal(platzhalterWaehlen('# allgemein', bis(24)), 'Nachricht an # allgemein');
  assert.equal(platzhalterWaehlen('beta@127.0.0.2', bis(40)), 'Nachricht an beta@127.0.0.2');
});

test('Channel: Nachricht an # allgemein → An # allgemein → # allgemein → Name mit …', () => {
  assert.equal(platzhalterWaehlen('# allgemein', bis(23)), 'An # allgemein');
  assert.equal(platzhalterWaehlen('# allgemein', bis(13)), '# allgemein');
  assert.equal(platzhalterWaehlen('# allgemein', bis(8)), '# allge…');
  assert.equal(platzhalterWaehlen('# allgemein', bis(3)), '# …');
});

test('Direktgespräch: die kürzeren Fassungen ohne @-Teil', () => {
  assert.equal(platzhalterWaehlen('beta@127.0.0.2', bis(26)), 'Nachricht an beta');
  assert.equal(platzhalterWaehlen('beta@127.0.0.2', bis(10)), 'An beta');
  assert.equal(platzhalterWaehlen('beta@127.0.0.2', bis(5)), 'beta');
  assert.equal(platzhalterWaehlen('beta@127.0.0.2', bis(3)), 'be…');
});

test('nichts passt → der erste Buchstabe mit …, nie leer', () => {
  assert.equal(platzhalterWaehlen('# allgemein', bis(0)), '#…');
});

test('lange Namen: die Ellipse nimmt das Ende, der Anfang bleibt', () => {
  assert.equal(platzhalterWaehlen('# ein-sehr-langer-channelname', bis(10)), '# ein-seh…');
});
