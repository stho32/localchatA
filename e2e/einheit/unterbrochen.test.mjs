// Unit-Tests „Hochladen unterbrochen“ (R00030 F0119 B0519, node --test): der Hinweis nach dem Neuladen (UX V8), der
// gesicherte Text vor dem Entwurf (product-owner Zeile 163) und der Eintrag im Tab-Speicher (Gate #9 R1: nur Text und
// Namen, kaputt oder übergroß übergangen, nach dem Lesen weg).
import test from 'node:test';
import assert from 'node:assert/strict';
import { unterbrochenHinweis, feldNachNeuladen, unterbrochenAnlegen } from '../../web/unterbrochen.js';

function speicherAnlegen(anfang = {}) {
  const werte = new Map(Object.entries(anfang));
  return {
    werte,
    getItem: (schluessel) => (werte.has(schluessel) ? werte.get(schluessel) : null),
    setItem: (schluessel, wert) => werte.set(schluessel, String(wert)),
    removeItem: (schluessel) => werte.delete(schluessel),
  };
}

test('Hinweis mit einer Datei: „Hochladen unterbrochen — bericht.pdf bitte erneut wählen.“, die Ansage gleich', () => {
  assert.deepEqual(unterbrochenHinweis(['bericht.pdf'], false), {
    hinweis: 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen.',
    ansage: 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen.',
  });
});

test('Hinweis mit Text oben im Feld endet mit „Text steht oben im Feld.“ — Hinweis und Ansage', () => {
  const erwartet = 'Hochladen unterbrochen — bericht.pdf bitte erneut wählen. Text steht oben im Feld.';
  assert.deepEqual(unterbrochenHinweis(['bericht.pdf'], true), { hinweis: erwartet, ansage: erwartet });
});

test('bis zu 3 Dateien: alle Namen, durch Komma getrennt', () => {
  assert.equal(
    unterbrochenHinweis(['a.pdf', 'b.png', 'c.txt'], false).hinweis,
    'Hochladen unterbrochen — a.pdf, b.png, c.txt bitte erneut wählen.',
  );
});

test('5 Dateien: die ersten drei und „und 2 weitere“', () => {
  assert.equal(
    unterbrochenHinweis(['a.pdf', 'b.png', 'c.txt', 'd.txt', 'e.txt'], false).hinweis,
    'Hochladen unterbrochen — a.pdf, b.png, c.txt und 2 weitere bitte erneut wählen.',
  );
});

test('ein Name von 40 Zeichen steht im Hinweis auf 30 Zeichen gekürzt (mit Ellipse), die Ansage nennt ihn ganz', () => {
  const lang = 'quartalsbericht-drittes-quartal-2026.pdf';
  assert.equal(Array.from(lang).length, 40);
  const { hinweis, ansage } = unterbrochenHinweis([lang, 'b.png'], false);
  assert.equal(hinweis, 'Hochladen unterbrochen — quartalsbericht-drittes-quart…, b.png bitte erneut wählen.');
  assert.equal(Array.from('quartalsbericht-drittes-quart…').length, 30);
  assert.equal(ansage, `Hochladen unterbrochen — ${lang}, b.png bitte erneut wählen.`);
});

test('ein Name von genau 30 Zeichen bleibt ungekürzt, Zeichen zählen als Codepunkte', () => {
  const dreissig = `${'ä'.repeat(26)}.pdf`;
  assert.equal(unterbrochenHinweis([dreissig], false).hinweis, `Hochladen unterbrochen — ${dreissig} bitte erneut wählen.`);
});

test('Feld leer: der gesicherte Text steht im Feld, nicht „oben“', () => {
  assert.deepEqual(feldNachNeuladen('Bericht anbei', ''), { text: 'Bericht anbei', textOben: false });
});

test('Feld nicht leer: gesicherter Text, Leerzeile, Entwurf unverändert dahinter', () => {
  assert.deepEqual(feldNachNeuladen('Bericht anbei', 'Neuer Entwurf'), { text: 'Bericht anbei\n\nNeuer Entwurf', textOben: true });
  assert.deepEqual(feldNachNeuladen('Bericht anbei', '  Entwurf\n'), { text: 'Bericht anbei\n\n  Entwurf\n', textOben: true });
});

test('ohne gesicherten Text bleibt der Entwurf, wie er ist', () => {
  assert.deepEqual(feldNachNeuladen('', 'Neuer Entwurf'), { text: 'Neuer Entwurf', textOben: false });
  assert.deepEqual(feldNachNeuladen('', ''), { text: '', textOben: false });
});

test('sichern merkt je Channel nur Text und Namen — keine Anhang-ID, kein Token, kein Inhalt', () => {
  const speicher = speicherAnlegen();
  unterbrochenAnlegen({ speicher }).sichern([
    { channel: 'allgemein', text: 'Bericht anbei', namen: ['bericht.pdf'], id: 'abcdefghijklmnopqrstuvwxyz', token: 'geheim' },
  ]);
  const roh = speicher.getItem('localchatA.unterbrochen.allgemein');
  assert.deepEqual(JSON.parse(roh), { text: 'Bericht anbei', namen: ['bericht.pdf'] });
  assert.ok(roh.length < 1024);
  assert.equal(speicher.werte.size, 1);
});

test('nehmen liest den Eintrag des Channels genau einmal; danach ist er weg, andere Channels bleiben', () => {
  const speicher = speicherAnlegen();
  const unterbrochen = unterbrochenAnlegen({ speicher });
  unterbrochen.sichern([
    { channel: 'allgemein', text: 'eins', namen: ['a.pdf'] },
    { channel: 'technik', text: '', namen: ['b.png'] },
  ]);
  assert.deepEqual(unterbrochen.nehmen('allgemein'), { text: 'eins', namen: ['a.pdf'] });
  assert.equal(unterbrochen.nehmen('allgemein'), null);
  assert.deepEqual(unterbrochen.nehmen('technik'), { text: '', namen: ['b.png'] });
});

test('zwei offene Einträge eines Channels und ein früher ungelesener werden einer: Texte mit Leerzeile, alle Namen', () => {
  const speicher = speicherAnlegen();
  const unterbrochen = unterbrochenAnlegen({ speicher });
  unterbrochen.sichern([{ channel: 'allgemein', text: 'alt', namen: ['a.pdf'] }]);
  unterbrochen.sichern([
    { channel: 'allgemein', text: 'eins', namen: ['b.png'] },
    { channel: 'allgemein', text: '', namen: ['c.txt'] },
    { channel: 'allgemein', text: 'zwei', namen: ['d.txt'] },
  ]);
  assert.deepEqual(unterbrochen.nehmen('allgemein'), { text: 'alt\n\neins\n\nzwei', namen: ['a.pdf', 'b.png', 'c.txt', 'd.txt'] });
});

test('ohne offene Einträge schreibt sichern nichts und löscht nichts', () => {
  const speicher = speicherAnlegen({ 'localchatA.unterbrochen.allgemein': '{"text":"x","namen":["a.pdf"]}' });
  unterbrochenAnlegen({ speicher }).sichern([]);
  assert.equal(speicher.werte.size, 1);
});

test('ein kaputter, übergroßer oder falsch geformter Eintrag wird ohne Fehler übergangen und entfernt', () => {
  const kaputt = [
    '{kaputt',
    'null',
    '[]',
    '"text"',
    '{"text":1,"namen":["a.pdf"]}',
    '{"text":"x","namen":"a.pdf"}',
    '{"text":"x","namen":[]}',
    '{"text":"x","namen":[1]}',
    '{"text":"x","namen":[""]}',
    JSON.stringify({ text: 'x'.repeat(70 * 1024), namen: ['a.pdf'] }),
  ];
  for (const roh of kaputt) {
    const speicher = speicherAnlegen({ 'localchatA.unterbrochen.allgemein': roh });
    assert.equal(unterbrochenAnlegen({ speicher }).nehmen('allgemein'), null, roh.slice(0, 40));
    assert.equal(speicher.getItem('localchatA.unterbrochen.allgemein'), null, roh.slice(0, 40));
  }
});

test('ein Speicher, der beim Schreiben wirft (voll, gesperrt), lässt sichern nicht scheitern', () => {
  const speicher = { ...speicherAnlegen(), setItem: () => { throw new Error('QuotaExceededError'); } };
  const warnungen = [];
  const warnen = console.warn;
  console.warn = (text) => warnungen.push(text);
  try {
    unterbrochenAnlegen({ speicher }).sichern([{ channel: 'allgemein', text: 'x', namen: ['a.pdf'] }]);
  } finally {
    console.warn = warnen;
  }
  assert.equal(warnungen.length, 1);
});
