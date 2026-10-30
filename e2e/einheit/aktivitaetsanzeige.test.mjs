// Unit-Tests der Tippanzeige (R00045 F0161 B0689; ADR-0051 Regeln 5, 7 c, 8; node --test, gestellte Zeit): Tabelle je
// Channel und Kennung mit lokalem Ablauf = Empfang + (bis − jetzt), Ende durch Ereignis, Nachricht, Lücke und Ablauf.
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  AKTIVITAETEN_LEER, aktivitaetEinordnen, nachrichtEingetroffen, aktiveIn, anzeigeTeile, anzeigeText, ANSAGE_STILL, tippAnsage,
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

// Wortlaut (R00045 F0162 B0690, ux-reviewer A): je Zustand eine Gruppe, `tippt` zuerst, getrennt durch ` · `, am Ende
// ` …` (U+2026) genau einmal; Name = Teil der Kennung vor dem @; die Eingabe ist schon alphabetisch nach Kennung.
const aktiv = (...paare) => paare.map(([kennung, zustand]) => ({ kennung, zustand, ablaufMs: 1 }));

test('Wortlaut: niemand → leer; ein, zwei, ab drei gezählt — je Zustand', () => {
  assert.equal(anzeigeText([]), '');
  assert.equal(anzeigeText(aktiv(['anna@10.0.0.7', 'tippt'])), 'anna tippt …');
  assert.equal(anzeigeText(aktiv(['anna@x', 'tippt'], ['ben@y', 'tippt'])), 'anna und ben tippen …');
  assert.equal(anzeigeText(aktiv(['anna@x', 'tippt'], ['ben@x', 'tippt'], ['cora@x', 'tippt'])), '3 Teilnehmer tippen …');
  assert.equal(anzeigeText(aktiv(['claude@x', 'denkt_nach'])), 'claude denkt nach …');
  assert.equal(anzeigeText(aktiv(['claude@x', 'denkt_nach'], ['codex@x', 'denkt_nach'])), 'claude und codex denken nach …');
  const vier = aktiv(['a@x', 'denkt_nach'], ['b@x', 'denkt_nach'], ['c@x', 'denkt_nach'], ['d@x', 'denkt_nach']);
  assert.equal(anzeigeText(vier), '4 Teilnehmer denken nach …');
});

test('Wortlaut: beide Zustände — tippt zuerst, auch wenn die Kennung des Nachdenkenden vorn steht', () => {
  assert.equal(anzeigeText(aktiv(['anna@x', 'tippt'], ['claude@x', 'denkt_nach'])), 'anna tippt · claude denkt nach …');
  assert.equal(anzeigeText(aktiv(['aaron@x', 'denkt_nach'], ['zora@x', 'tippt'])), 'zora tippt · aaron denkt nach …');
  const vier = aktiv(['anna@x', 'tippt'], ['ben@x', 'tippt'], ['claude@x', 'denkt_nach'], ['codex@x', 'denkt_nach']);
  assert.equal(anzeigeText(vier), 'anna und ben tippen · claude und codex denken nach …');
  const fuenf = aktiv(['anna@x', 'tippt'], ['ben@x', 'tippt'], ['cora@x', 'tippt'], ['claude@x', 'denkt_nach']);
  assert.equal(anzeigeText(fuenf), '3 Teilnehmer tippen · claude denkt nach …');
});

test('Wortlaut: „…“ ist U+2026 und steht genau einmal, am Ende nach einem Leerzeichen', () => {
  const text = anzeigeText(aktiv(['anna@x', 'tippt'], ['claude@x', 'denkt_nach']));
  assert.equal([...text].filter((zeichen) => zeichen === '\u2026').length, 1);
  assert.ok(text.endsWith(' \u2026'));
  assert.ok(!text.includes('...'));
});

test('Teile: Namen sind als Name markiert (halbfett), Zählung und Verben nicht', () => {
  assert.deepEqual(anzeigeTeile(aktiv(['anna@x', 'tippt'], ['claude@x', 'denkt_nach'])), [
    { text: 'anna', name: true }, { text: ' tippt', name: false }, { text: ' · ', name: false },
    { text: 'claude', name: true }, { text: ' denkt nach', name: false }, { text: ' …', name: false },
  ]);
  assert.deepEqual(anzeigeTeile(aktiv(['anna@x', 'tippt'], ['ben@x', 'tippt'])), [
    { text: 'anna', name: true }, { text: ' und ', name: false }, { text: 'ben', name: true },
    { text: ' tippen', name: false }, { text: ' …', name: false },
  ]);
  assert.deepEqual(anzeigeTeile(aktiv(['a@x', 'tippt'], ['b@x', 'tippt'], ['c@x', 'tippt'])), [
    { text: '3 Teilnehmer tippen', name: false }, { text: ' …', name: false },
  ]);
  assert.deepEqual(anzeigeTeile([]), []);
});

test('Wortlaut: ein unbekannter Zustand ist ein Fehler', () => {
  assert.throws(() => anzeigeText(aktiv(['anna@x', 'schreibt'])), /unbekannter Zustand schreibt/);
});

// Ansage (R00045 F0162 B0691, ux-reviewer C; gestellte Uhr): Text = Zeile ohne „ …“; gesetzt nur beim Wechsel leer →
// nicht leer und frühestens 30 s nach der vorigen Setzung, geleert beim Wechsel nicht leer → leer; dieselbe Zeichenkette
// nie neu. Die Schritte sind [ms, Text der Zeile ohne „ …“]; gesammelt wird, was in die Region geschrieben wird.
function ansagen(schritte) {
  let zustand = ANSAGE_STILL;
  const geschrieben = [];
  for (const [jetztMs, text] of schritte) {
    const ergebnis = tippAnsage(zustand, text, jetztMs);
    zustand = ergebnis.zustand;
    if (ergebnis.schreiben !== null) {
      geschrieben.push(`${JSON.stringify(ergebnis.schreiben)}@${jetztMs}`);
    }
  }
  return geschrieben;
}

test('Ansage: anna tippt 12 s mit Erneuerungen, dann Ablauf → genau zwei Änderungen', () => {
  const schritte = [[0, '']];
  for (let ms = 100; ms <= 12000; ms += 3000) {
    schritte.push([ms, 'anna tippt']);
  }
  schritte.push([18000, '']);
  assert.deepEqual(ansagen(schritte), ['"anna tippt"@100', '""@18000']);
});

test('Ansage: Namens- und Zustandswechsel bei nicht leerer Zeile ändern nichts (M13)', () => {
  assert.deepEqual(ansagen([
    [0, 'anna tippt'], [1000, 'anna und ben tippen'], [2000, 'anna und ben tippen · claude denkt nach'], [3000, 'claude denkt nach'],
  ]), ['"anna tippt"@0']);
});

test('Ansage: frühestens 30 s nach der vorigen Setzung — dazwischen bleibt die Region leer, auch wenn die Zeile wieder steht', () => {
  assert.deepEqual(ansagen([
    [0, 'anna tippt'], [7000, ''], [10000, 'ben tippt'], [16000, ''], [29999, 'cora tippt'], [29999.5, ''], [30000, 'dora tippt'],
  ]), ['"anna tippt"@0', '""@7000', '"dora tippt"@30000']);
});

test('Ansage: leer → leer und eine leere Region werden nie neu geschrieben; dieselbe Zeichenkette nie zweimal hintereinander', () => {
  assert.deepEqual(ansagen([[0, ''], [5, '']]), []);
  assert.deepEqual(ansagen([[0, 'anna tippt'], [100, ''], [40000, ''], [50000, 'anna tippt']]), [
    '"anna tippt"@0', '""@100', '"anna tippt"@50000',
  ]);
});

test('Ansage: übersprungene Setzung → beim Leeren wird nichts geschrieben (die Region ist schon leer)', () => {
  assert.deepEqual(ansagen([[0, 'anna tippt'], [100, ''], [200, 'ben tippt'], [300, '']]), ['"anna tippt"@0', '""@100']);
});
