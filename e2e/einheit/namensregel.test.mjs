// Unit-Tests der Namensregel in der Oberfläche (R00003 F0010, node --test) — Spiegel von ADR-0013.
import test from 'node:test';
import assert from 'node:assert/strict';
import { pruefen } from '../../web/namensregel.js';

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
