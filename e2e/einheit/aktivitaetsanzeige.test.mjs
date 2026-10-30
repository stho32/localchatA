// Unit-Tests der Tippanzeige (R00045 F0161 B0689; ADR-0051 Regeln 5, 7 c, 8; node --test, gestellte Zeit): Tabelle je
// Channel und Kennung mit lokalem Ablauf = Empfang + (bis − jetzt), Ende durch Ereignis, Nachricht, Lücke und Ablauf.
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  AKTIVITAETEN_LEER, aktivitaetEinordnen, nachrichtEingetroffen, aktiveIn,
} from '../../web/aktivitaetsanzeige.js';

const JETZT = '2026-10-30T10:00:00.000+01:00';
const gesetzt = (kennung, { channel = 'allgemein', zustand = 'tippt', bisMs = 6000 } = {}) => ({
  art: 'aktivitaet', channel, kennung, zustand, jetzt: JETZT, bis: new Date(Date.parse(JETZT) + bisMs).toISOString(),
});
const beendet = (kennung, channel = 'allgemein') => ({ art: 'aktivitaet', channel, kennung, zustand: null });

test('gesetzt: Eintrag mit lokalem Ablauf Empfang + (bis − jetzt), unabhängig von der Uhr des Servers', () => {
  const tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('anna@10.0.0.7'), 500000);

  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ben@10.0.0.8', 500000), [
    { kennung: 'anna@10.0.0.7', zustand: 'tippt', ablaufMs: 506000 },
  ]);
  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ben@10.0.0.8', 505999).length, 1);
  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ben@10.0.0.8', 506000), []);
});

test('Erneuerung ersetzt den Ablauf (nicht addiert), ein anderer Zustand ersetzt den alten', () => {
  let tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('claude@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('claude@x', { zustand: 'denkt_nach', bisMs: 30000 }), 2000);

  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ich@x', 2000), [{ kennung: 'claude@x', zustand: 'denkt_nach', ablaufMs: 32000 }]);
});

test('beendet entfernt nur diese Kennung in diesem Channel', () => {
  let tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('anna@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('ben@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('anna@x', { channel: 'technik' }), 0);
  tabelle = aktivitaetEinordnen(tabelle, beendet('anna@x'), 10);

  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ich@x', 10).map((eintrag) => eintrag.kennung), ['ben@x']);
  assert.deepEqual(aktiveIn(tabelle, 'technik', 'ich@x', 10).map((eintrag) => eintrag.kennung), ['anna@x']);
});

test('eine Nachricht von K in C blendet K in C sofort aus — nicht in anderen Channels, nicht andere Kennungen', () => {
  let tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('anna@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('ben@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('anna@x', { channel: 'technik' }), 0);

  tabelle = nachrichtEingetroffen(tabelle, { art: 'nachricht', channel: 'allgemein', nachricht: { von: 'anna@x', text: 'Hallo' } });

  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ich@x', 1).map((eintrag) => eintrag.kennung), ['ben@x']);
  assert.deepEqual(aktiveIn(tabelle, 'technik', 'ich@x', 1).map((eintrag) => eintrag.kennung), ['anna@x']);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('anna@x'), 5);
  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ich@x', 5).map((eintrag) => eintrag.kennung), ['anna@x', 'ben@x']);
});

test('aktivitaet_luecke leert alle Anzeigen aller Channels', () => {
  let tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('anna@x'), 0);
  tabelle = aktivitaetEinordnen(tabelle, gesetzt('ben@x', { channel: 'technik' }), 0);

  tabelle = aktivitaetEinordnen(tabelle, { art: 'aktivitaet_luecke' }, 1);

  assert.deepEqual(tabelle, AKTIVITAETEN_LEER);
});

test('aktiveIn: alphabetisch nach Kennung, die eigene Kennung nie, ein unbekannter Channel leer', () => {
  let tabelle = AKTIVITAETEN_LEER;
  for (const kennung of ['cora@x', 'anna@y', 'ich@x', 'ben@x']) {
    tabelle = aktivitaetEinordnen(tabelle, gesetzt(kennung), 0);
  }

  assert.deepEqual(aktiveIn(tabelle, 'allgemein', 'ich@x', 0).map((eintrag) => eintrag.kennung), ['anna@y', 'ben@x', 'cora@x']);
  assert.deepEqual(aktiveIn(tabelle, 'gibt-es-nicht', 'ich@x', 0), []);
});

test('der Ablauf rechnet ab dem Empfang beim Leser: maßgeblich ist bis − jetzt aus demselben Ereignis', () => {
  const tabelle = aktivitaetEinordnen(AKTIVITAETEN_LEER, gesetzt('anna@x', { bisMs: 6000 }), 1000);

  assert.equal(aktiveIn(tabelle, 'allgemein', 'ich@x', 1000)[0].ablaufMs, 7000);
});

test('ein unbekanntes Ereignis ist ein Fehler', () => {
  assert.throws(() => aktivitaetEinordnen(AKTIVITAETEN_LEER, { art: 'teilnehmer' }, 0), /unbekanntes Ereignis teilnehmer/);
});
