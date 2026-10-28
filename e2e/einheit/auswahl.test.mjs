// Unit-Tests der Channel-Auswahl (R00007 F0028 B0148, B0149; F0032 B0168, B0169; ADR-0027; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { auswahl } from '../../web/auswahl.js';

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
