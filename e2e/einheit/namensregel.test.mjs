// Unit-Tests der Namensregel in der Oberfläche (R00003 F0010, node --test) — Spiegel von ADR-0013.
import test from 'node:test';
import assert from 'node:assert/strict';
import { AKZENTE, pruefen } from '../../web/namensregel.js';

test('gültige Namen haben keinen Hinweis und keinen Vorschlag', () => {
  for (const name of ['a', 'claude-2', 'a--b', 'x'.repeat(32)]) {
    assert.deepEqual(pruefen(name), { gueltig: true, hinweis: '', vorschlag: '' }, name);
  }
});

test('Jörg ist ungültig mit Hinweis auf die erlaubten Zeichen und Vorschlag joerg', () => {
  const ergebnis = pruefen('Jörg');
  assert.equal(ergebnis.gueltig, false);
  assert.match(ergebnis.hinweis, /a–z, 0–9 und -/);
  assert.equal(ergebnis.vorschlag, 'joerg');
});

test('die Umformung folgt der Regel des Servers', () => {
  const faelle = [
    ['Claude', 'claude'], ['ÄÖÜß', 'aeoeuess'], ['a_b', 'a-b'], ['a.b', 'a-b'], ['a b', 'a-b'],
    ['a@b', 'a-b'], ['-a', 'a'], ['a-', 'a'], ['a__..b', 'a-b'], ['chef​', 'chef'],
    ['‮abc', 'abc'], ['c'.repeat(33), 'c'.repeat(32)], ['a'.repeat(31) + '-b', 'a'.repeat(31)],
  ];
  for (const [eingabe, vorschlag] of faelle) {
    assert.equal(pruefen(eingabe).vorschlag, vorschlag, JSON.stringify(eingabe));
  }
});

// Wie am Server (GELESEN_HOECHSTENS in src/name.rs): umgeformt werden höchstens die ersten 4 × 32 Zeichen.
test('lange Eingaben: der Vorschlag kommt aus den ersten 128 Zeichen, wie am Server', () => {
  assert.equal(pruefen('a'.repeat(40) + 'ÿ'.repeat(100000)).vorschlag, 'a'.repeat(32));
  assert.equal(pruefen('_'.repeat(125) + 'abc').vorschlag, 'abc');
  assert.equal(pruefen('_'.repeat(128) + 'abc').vorschlag, '');
});

test('ohne verwertbares Zeichen gibt es keinen Vorschlag', () => {
  for (const eingabe of ['', '___', '-', '​']) {
    const ergebnis = pruefen(eingabe);
    assert.equal(ergebnis.gueltig, false, JSON.stringify(eingabe));
    assert.equal(ergebnis.vorschlag, '', JSON.stringify(eingabe));
  }
});

test('der Hinweis nennt die Regel mit dem Wortlaut des Servers (NAMENSREGEL in src/name.rs)', async () => {
  const { readFile } = await import('node:fs/promises');
  const quelle = await readFile(new URL('../../src/name.rs', import.meta.url), 'utf8');
  const regelDesServers = /pub const NAMENSREGEL: &str =\s*"([^"]+)";/.exec(quelle)[1];

  assert.equal(pruefen('Jörg').hinweis, `Erlaubt sind ${regelDesServers}.`);
});

// R00006 B0138: Akzente im Vorschlag mit derselben Tabelle wie am Server, für Teilnehmer- und Channelnamen.
test('Akzente werden im Vorschlag zum Grundbuchstaben (Teilnehmer und Channel)', () => {
  const faelle = [
    ['Allgemein', 'allgemein'], ['Prüfung', 'pruefung'], ['José', 'jose'], ['Café Crème', 'cafe-creme'],
    ['Ñandú', 'nandu'], ['Łódź', 'lodz'], ['Ærø', 'aero'], ['Straße', 'strasse'], ['Œuvre', 'oeuvre'],
    ['İstanbul', 'istanbul'], ['日本', ''],
  ];
  for (const art of ['teilnehmer', 'channel']) {
    for (const [eingabe, vorschlag] of faelle) {
      assert.equal(pruefen(eingabe, art).vorschlag, vorschlag, `${art}: ${eingabe}`);
    }
  }
});

test('die Akzenttabelle ist dieselbe wie am Server (AKZENTE in src/name.rs)', async () => {
  const { readFile } = await import('node:fs/promises');
  const quelle = await readFile(new URL('../../src/name.rs', import.meta.url), 'utf8');
  const block = /pub const AKZENTE: &\[\(&str, &str\)\] = &\[([\s\S]*?)\];/.exec(quelle)[1];
  const tabelleDesServers = [...block.matchAll(/\("([^"]+)", "([a-z]+)"\)/g)].map((treffer) => [treffer[1], treffer[2]]);

  assert.ok(tabelleDesServers.length > 20, `${tabelleDesServers.length} Zeilen gelesen`);
  assert.deepEqual(AKZENTE, tabelleDesServers);
});

test('Channelnamen: reservierte Gerätenamen ohne Vorschlag, ist die Umformung reserviert, keiner', () => {
  for (const name of ['con', 'prn', 'aux', 'nul', 'com0', 'com9', 'lpt0', 'lpt9']) {
    const ergebnis = pruefen(name, 'channel');
    assert.equal(ergebnis.gueltig, false, name);
    assert.match(ergebnis.hinweis, /Gerätename/, name);
    assert.equal(ergebnis.vorschlag, '', name);
  }
  assert.equal(pruefen('CON', 'channel').vorschlag, '');
  assert.equal(pruefen('Com10', 'channel').vorschlag, 'com10');
  for (const name of ['com10', 'console', 'lpt', 'a-1']) {
    assert.equal(pruefen(name, 'channel').gueltig, true, name);
  }
});

test('für Teilnehmer gilt die Regel ohne Gerätenamen — wie am Server', () => {
  assert.equal(pruefen('con').gueltig, true);
  assert.equal(pruefen('con', 'teilnehmer').gueltig, true);
});
