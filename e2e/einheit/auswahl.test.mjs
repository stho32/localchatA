// Unit-Tests der Channel-Auswahl (R00007 F0028 B0148, B0149; F0032 B0168, B0169; ADR-0027; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { auswahl, startadresse, auswahlMerken, MERKER, nachbarChannel, kuerzelRichtung } from '../../web/auswahl.js';

const LISTE = ['allgemein', 'test'];

test('#/c/<name> mit bekanntem Namen wählt ihn als bekannt', () => {
  assert.deepEqual(auswahl('#/c/allgemein', LISTE), { name: 'allgemein', bekannt: true });
  assert.deepEqual(auswahl('#/c/test', LISTE), { name: 'test', bekannt: true });
});

test('ein gültiger, aber unbekannter Name ist gewählt und unbekannt', () => {
  assert.deepEqual(auswahl('#/c/neu', LISTE), { name: 'neu', bekannt: false });
  assert.deepEqual(auswahl('#/c/neu', []), { name: 'neu', bekannt: false });
});

test('ungültige Namen und andere Adressen wählen nichts', () => {
  for (const hash of ['#/c/Neu', '#/x', '', '#', '#/c/', '#/c/allgemein/', '#/c/a b', '#c/allgemein', '#/c/-a', '#/c/con']) {
    assert.equal(auswahl(hash, LISTE), null, hash);
  }
});

test('Start ohne Adresse mit Merker setzt die Adresse des Merkers, sonst nichts', () => {
  assert.equal(startadresse('', 'test'), '#/c/test');
  assert.equal(startadresse('#', 'test'), '#/c/test');
  assert.equal(startadresse('', null), null);
  assert.equal(startadresse('', 'Kaputt'), null);
  assert.equal(startadresse('#/c/allgemein', 'test'), null);
});

test('jede Auswahl wird je Tab gemerkt, keine Auswahl ändert den Merker nicht', () => {
  const gespeichert = new Map();
  const speicher = { setItem: (schluessel, wert) => gespeichert.set(schluessel, wert) };

  auswahlMerken(speicher, { name: 'test', bekannt: true });
  assert.equal(gespeichert.get(MERKER), 'test');
  auswahlMerken(speicher, { name: 'spaeter', bekannt: false });
  assert.equal(gespeichert.get(MERKER), 'spaeter');
  auswahlMerken(speicher, null);
  assert.equal(gespeichert.get(MERKER), 'spaeter');
  assert.equal(MERKER, 'localchatA.channel');
});

test('Nachbar-Channel: nächster und voriger in der Reihenfolge der Liste, am Rand nichts', () => {
  const liste = ['a', 'b', 'c'];
  assert.equal(nachbarChannel(liste, 'b', 'ab'), 'c');
  assert.equal(nachbarChannel(liste, 'b', 'auf'), 'a');
  assert.equal(nachbarChannel(liste, 'c', 'ab'), null);
  assert.equal(nachbarChannel(liste, 'a', 'auf'), null);
});

test('Nachbar-Channel ohne Auswahl: ↓ der erste, ↑ der letzte; eine leere Liste hat keinen', () => {
  assert.equal(nachbarChannel(['a', 'b', 'c'], null, 'ab'), 'a');
  assert.equal(nachbarChannel(['a', 'b', 'c'], null, 'auf'), 'c');
  assert.equal(nachbarChannel(['a', 'b', 'c'], 'unbekannt', 'ab'), 'a');
  assert.equal(nachbarChannel([], null, 'ab'), null);
  assert.equal(nachbarChannel([], null, 'auf'), null);
});

const taste = (key, zusatz = {}, altGraph = false) => ({
  key, altKey: true, ctrlKey: false, shiftKey: false, metaKey: false, getModifierState: (name) => name === 'AltGraph' && altGraph, ...zusatz,
});

test('Kürzel: Alt ↓ und Alt ↑ wechseln, mit Strg, Umschalt, Meta oder AltGraph nicht', () => {
  assert.equal(kuerzelRichtung(taste('ArrowDown')), 'ab');
  assert.equal(kuerzelRichtung(taste('ArrowUp')), 'auf');
  assert.equal(kuerzelRichtung(taste('ArrowDown', { ctrlKey: true })), null);
  assert.equal(kuerzelRichtung(taste('ArrowDown', { shiftKey: true })), null);
  assert.equal(kuerzelRichtung(taste('ArrowDown', { metaKey: true })), null);
  assert.equal(kuerzelRichtung(taste('ArrowDown', {}, true)), null);
  assert.equal(kuerzelRichtung(taste('ArrowDown', { altKey: false })), null);
  assert.equal(kuerzelRichtung(taste('ArrowLeft')), null);
});
