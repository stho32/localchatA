// Unit-Tests des Gesprächsschlüssels der Oberfläche und der Adresse `#/d/<kennung>` (R00050 F0178 B0784, ADR-0055
// Regel 3, A00005 Ergänzung Karte #13; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  direktgespraech, gegenueberIn, mitGespraechsschluessel, apiPfad, ortText, anredeText,
} from '../../web/gespraech.js';
import { direktAuswahl } from '../../web/auswahl.js';
import { verlaufAbrufen, nachrichtSenden, nachrichtBearbeiten, nachrichtLoeschen, aktivitaetMelden } from '../../web/api.js';

const ICH = 'anna@127.0.0.1';

test('ein Direktgespräch heißt direkt:<Kennung des Gegenübers>, ein Channel bleibt sein Name', () => {
  assert.equal(direktgespraech('ben@127.0.0.3'), 'direkt:ben@127.0.0.3');
  assert.equal(gegenueberIn('direkt:ben@127.0.0.3'), 'ben@127.0.0.3');
  assert.equal(gegenueberIn('allgemein'), null);
});

test('Antworten und Ereignisse mit `direkt` tragen danach den Schlüssel als `channel`; Channels bleiben, wie sie sind', () => {
  const direkt = { art: 'nachricht', channel: null, direkt: 'ben@127.0.0.3', nachricht: { text: 'x' } };
  assert.deepEqual(mitGespraechsschluessel(direkt), { ...direkt, channel: 'direkt:ben@127.0.0.3' });
  const imChannel = { art: 'nachricht', channel: 'allgemein', direkt: null, nachricht: { text: 'x' } };
  assert.equal(mitGespraechsschluessel(imChannel), imChannel);
  assert.equal(mitGespraechsschluessel(null), null);
  assert.deepEqual(mitGespraechsschluessel({ fehler: 'x' }), { fehler: 'x' });
});

test('der API-Pfad: Channel unter /api/channels, Direktgespräch unter /api/direkt — beide prozentkodiert', () => {
  assert.equal(apiPfad('a-b'), '/api/channels/a-b');
  assert.equal(apiPfad('direkt:ben@127.0.0.3'), '/api/direkt/ben%40127.0.0.3');
  assert.equal(apiPfad('direkt:ben@::1'), '/api/direkt/ben%40%3A%3A1');
});

test('Ort und Anrede: „# name“ im Channel, „Direktnachricht mit <Kennung>“ und die Kennung im Direktgespräch', () => {
  assert.equal(ortText('allgemein'), '# allgemein');
  assert.equal(ortText('direkt:ben@127.0.0.3'), 'Direktnachricht mit ben@127.0.0.3');
  assert.equal(anredeText('allgemein'), '# allgemein');
  assert.equal(anredeText('direkt:ben@127.0.0.3'), 'ben@127.0.0.3');
});

test('#/d/<kennung> wählt das Gegenüber — prozentkodiert oder nicht, IPv4 und IPv6 in kanonischer Form', () => {
  assert.deepEqual(direktAuswahl('#/d/ben%40127.0.0.3', ICH), { gegenueber: 'ben@127.0.0.3' });
  assert.deepEqual(direktAuswahl('#/d/ben@127.0.0.3', ICH), { gegenueber: 'ben@127.0.0.3' });
  assert.deepEqual(direktAuswahl('#/d/ben%40%3A%3A1', ICH), { gegenueber: 'ben@::1' });
  assert.deepEqual(direktAuswahl('#/d/claude-2%40fd00%3A%3Aa', ICH), { gegenueber: 'claude-2@fd00::a' });
});

test('andere Adressen sind keine Direktnachricht', () => {
  for (const hash of ['', '#', '#/c/allgemein', '#/x', '#d/ben@127.0.0.3']) {
    assert.equal(direktAuswahl(hash, ICH), null, hash);
  }
});

test('die eigene Kennung und jede ungültige Adresse führen zu keinem Gespräch', () => {
  const ungueltig = [
    '#/d/anna%40127.0.0.1', '#/d/', '#/d/xyz', '#/d/%40', '#/d/ben%40', '#/d/%40127.0.0.1', '#/d/BEN%40127.0.0.1',
    '#/d/ben%40999.1.1.1', '#/d/ben%40127.1', '#/d/ben%40localhost', '#/d/ben%40FD00%3A%3AA', '#/d/ben%40fd00%3A0%3A%3Aa',
    '#/d/ben%40127.0.0.3%40x', '#/d/ben%40127.0.0.3/x', '#/d/%E0%A4%A', '#/d/ben%40fe80%3A%3A1%25eth0', '#/d/ben%20%40127.0.0.3',
  ];
  for (const hash of ungueltig) {
    assert.deepEqual(direktAuswahl(hash, ICH), { gegenueber: null }, hash);
  }
});

// Code-Review #13 F0178 S1: IPv4-gemappte Formen sind am Server Aliase der IPv4-Kennung (to_canonical) — auch in der
// Schreibweise, die der Browser selbst bildet (Hex). Die Gegenprobe ohne `ffff` bleibt eine echte IPv6-Kennung.
test('IPv4-gemappte Kennungen führen zu keinem Gespräch, ::7f00:1 bleibt gültig (S1)', () => {
  for (const hash of ['#/d/ben%40%3A%3Affff%3A7f00%3A1', '#/d/ben%40%3A%3Affff%3A127.0.0.1', '#/d/ben%40%3A%3A7f00%3A1%3A']) {
    assert.deepEqual(direktAuswahl(hash, ICH), { gegenueber: null }, hash);
  }
  assert.deepEqual(direktAuswahl('#/d/ben%40%3A%3A7f00%3A1', ICH), { gegenueber: 'ben@::7f00:1' });
});

// Code-Review #13 F0178 S2: IPv4 nur in kürzester Schreibweise — führende Nullen und der Schlusspunkt sind Aliase.
test('IPv4-Aliase mit führenden Nullen oder Schlusspunkt führen zu keinem Gespräch (S2)', () => {
  for (const hash of ['#/d/ben%40127.0.0.03', '#/d/ben%40127.0.0.003', '#/d/ben%400127.0.0.3', '#/d/ben%40127.0.0.3.']) {
    assert.deepEqual(direktAuswahl(hash, ICH), { gegenueber: null }, hash);
  }
});

function holenMit(status, text = '{}') {
  const aufrufe = [];
  const holen = async (pfad, optionen) => {
    aufrufe.push({ pfad, optionen });
    return { status, ok: status < 400, text: async () => text, json: async () => JSON.parse(text), headers: { get: () => null } };
  };
  return { holen, aufrufe };
}

test('Verlauf, Senden, Bearbeiten, Löschen und Tippen gehen im Direktgespräch an /api/direkt/<kennung>', async () => {
  const { holen, aufrufe } = holenMit(200);
  const schluessel = 'direkt:ben@127.0.0.3';
  await verlaufAbrufen(schluessel, {}, { token: 't' }, holen);
  await nachrichtSenden(schluessel, 'Hallo', 'k', { token: 't' }, holen);
  await nachrichtBearbeiten(schluessel, 'z', 'Neu', 'k', { token: 't' }, holen);
  await nachrichtLoeschen(schluessel, 'z', { token: 't' }, holen);
  await aktivitaetMelden({ art: 'setzen', channel: schluessel }, { token: 't' }, holen);

  assert.deepEqual(aufrufe.map((aufruf) => aufruf.pfad), [
    '/api/direkt/ben%40127.0.0.3/nachrichten?n=100',
    '/api/direkt/ben%40127.0.0.3/nachrichten',
    '/api/direkt/ben%40127.0.0.3/nachrichten/bearbeiten',
    '/api/direkt/ben%40127.0.0.3/nachrichten/loeschen',
    '/api/direkt/ben%40127.0.0.3/aktivitaet',
  ]);
});
