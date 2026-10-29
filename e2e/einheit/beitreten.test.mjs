// Unit-Tests der Texte des Beitreten-Bildschirms (R00003 F0010, node --test): Meldung und Hinweis als Teile,
// Text oder { kennung } für Namen, Kennungen und Vorschläge.
import test from 'node:test';
import assert from 'node:assert/strict';
import { meldungsteile, hinweisFuer, wartendeText, zeichnen } from '../../web/beitreten.js';
import { LEER, ungesendete } from '../../web/zustellung.js';

process.env.TZ = 'Europe/Berlin';

const REGEL = '1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende';

function alsText(teile) {
  return teile.map((teil) => {
    if (typeof teil === 'string') {
      return teil;
    }
    return teil.kennung;
  }).join('');
}

function kennungen(teile) {
  return teile.filter((teil) => typeof teil !== 'string').map((teil) => teil.kennung);
}

test('Name vergeben: ⚠, Name, Träger, Uhrzeit HH:MM in Ortszeit und den Vorschlag, alle drei als Kennung', () => {
  const teile = meldungsteile({
    art: 'vergeben', name: 'shoff', belegtVon: 'shoff@127.0.0.1', seit: '2026-10-27T08:04:00.000+00:00', vorschlag: 'shoff-2',
  });

  assert.equal(alsText(teile), '⚠ Der Name shoff ist vergeben — shoff@127.0.0.1 ist seit 09:04 dabei. Wie wäre shoff-2?');
  assert.deepEqual(kennungen(teile), ['shoff', 'shoff@127.0.0.1', 'shoff-2']);
});

test('ungültig: ⚠, Name, Regel des Servers und Vorschlag — dieselbe Form wie „vergeben“', () => {
  const teile = meldungsteile({ art: 'ungueltig', name: 'Jörg', fehler: 'f', regel: REGEL, vorschlag: 'joerg' });

  assert.equal(alsText(teile), `⚠ Der Name Jörg ist nicht erlaubt — ${REGEL}. Wie wäre joerg?`);
  assert.deepEqual(kennungen(teile), ['Jörg', 'joerg']);
});

test('ungültig ohne Vorschlag endet nach der Regel', () => {
  const teile = meldungsteile({ art: 'ungueltig', name: '___', fehler: 'f', regel: REGEL, vorschlag: '' });

  assert.equal(alsText(teile), `⚠ Der Name ___ ist nicht erlaubt — ${REGEL}.`);
});

test('leer, Server weg und sonstige Fehler mit ⚠ und Wort', () => {
  assert.equal(alsText(meldungsteile({ art: 'leer' })), '⚠ Bitte einen Namen eingeben.');
  assert.equal(alsText(meldungsteile({ art: 'serverWeg', name: 'shoff' })), '⚠ Server nicht erreichbar.');
  assert.equal(alsText(meldungsteile({ art: 'fehler', name: 'n', fehler: 'Zu viele Sitzungen' })), '⚠ Zu viele Sitzungen');
});

test('ohne Meldung: leerer Bildschirm', () => {
  assert.deepEqual(meldungsteile({ art: 'beitreten', name: '' }), []);
});

test('Hinweis beim Tippen: leer bei gültigem, leerem oder nur aus Leerzeichen bestehendem Text, sonst Regel und Vorschlag', () => {
  const ohne = { regel: '', vorschlagsteile: [] };
  assert.deepEqual(hinweisFuer('claude-2'), ohne);
  assert.deepEqual(hinweisFuer(''), ohne);
  assert.deepEqual(hinweisFuer('   '), ohne);
  assert.equal(hinweisFuer('Jörg').regel, `Erlaubt sind ${REGEL}.`);
  assert.equal(alsText(hinweisFuer('Jörg').vorschlagsteile), ' Wie wäre joerg?');
  assert.deepEqual(kennungen(hinweisFuer('Jörg').vorschlagsteile), ['joerg']);
  assert.deepEqual(hinweisFuer('___'), { regel: `Erlaubt sind ${REGEL}.`, vorschlagsteile: [] });
});

// R00025 F0088 „Wartende Nachrichten genannt“ (B0492; UX-Vorprüfung #36 V12): keine Zeile ohne ungesendete Nachricht,
// sonst Einzahl und Mehrzahl; gezählt werden fehlgeschlagene, nicht vorläufige und nicht verworfene.
test('wartende Nachrichten auf dem Beitreten-Bildschirm: keine, eine, zwei', () => {
  assert.equal(wartendeText(0), '');
  assert.equal(wartendeText(1), '1 ungesendete Nachricht wartet auf den Beitritt — Neuladen verwirft sie.');
  assert.equal(wartendeText(2), '2 ungesendete Nachrichten warten auf den Beitritt — Neuladen verwirft sie.');
  const zustand = {
    ...LEER,
    vorlaeufige: [
      { schluessel: 'a', channel: 'allgemein', text: 'a', zustand: 'fehlgeschlagen', grund: 'x' },
      { schluessel: 'b', channel: 'technik', text: 'b', zustand: 'fehlgeschlagen', grund: 'x' },
      { schluessel: 'c', channel: 'allgemein', text: 'c', zustand: 'wird gesendet' },
      { schluessel: 'd', channel: 'allgemein', text: 'd', zustand: 'verworfen', grund: 'x' },
    ],
  };
  assert.equal(ungesendete(zustand), 2);
});

// R00033 F0124 B0551 (ux-reviewer #36 S3): ein DOM-Ersatz genügt, um die Verdrahtung von Wartezeile und Feld zu lesen.
class Knoten {
  constructor(tag) {
    this.tag = tag;
    this.kinder = [];
    this.attribute = {};
    this.classList = { toggle() {} };
  }
  setAttribute(name, wert) { this.attribute[name] = wert; }
  removeAttribute(name) { delete this.attribute[name]; }
  append(...kinder) { this.kinder.push(...kinder); }
  replaceChildren(...kinder) { this.kinder = kinder; }
  addEventListener() {}
  focus() {}
  finden(pruefung) {
    if (pruefung(this)) {
      return this;
    }
    return this.kinder.filter((k) => k instanceof Knoten).map((k) => k.finden(pruefung)).find(Boolean);
  }
}

function beitretenZeichnen(wartende) {
  const vorher = globalThis.document;
  globalThis.document = { createElement: (tag) => new Knoten(tag) };
  try {
    const wurzel = new Knoten('div');
    zeichnen(wurzel, { art: 'beitreten', name: '' }, { wartende, beitreten: async () => {} });
    return wurzel;
  } finally {
    globalThis.document = vorher;
  }
}

test('die Wartezeile beschreibt das Namensfeld für Bildschirmleser', () => {
  const wurzel = beitretenZeichnen(2);
  const feld = wurzel.finden((k) => k.id === 'lc-name');
  const zeile = wurzel.finden((k) => k.className === 'lc-wartende');

  assert.equal(zeile.textContent, '2 ungesendete Nachrichten warten auf den Beitritt — Neuladen verwirft sie.');
  assert.ok(zeile.id, 'die Wartezeile trägt eine id');
  assert.equal(feld.attribute['aria-describedby'], `lc-name-zweck lc-name-hinweis lc-meldung ${zeile.id}`);
  assert.equal(zeile.attribute.role, undefined);
});

test('ohne wartende Nachrichten gibt es weder Zeile noch Verweis', () => {
  const wurzel = beitretenZeichnen(0);
  const feld = wurzel.finden((k) => k.id === 'lc-name');

  assert.equal(wurzel.finden((k) => k.className === 'lc-wartende'), undefined);
  assert.equal(feld.attribute['aria-describedby'], 'lc-name-zweck lc-name-hinweis lc-meldung');
});
