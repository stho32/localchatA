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
    this.behandler = {};
    this.classList = { toggle() {} };
  }
  setAttribute(name, wert) { this.attribute[name] = wert; }
  removeAttribute(name) { delete this.attribute[name]; }
  append(...kinder) { this.kinder.push(...kinder); }
  replaceChildren(...kinder) { this.kinder = kinder; }
  addEventListener(art, behandler) { (this.behandler[art] ||= []).push(behandler); }
  ausloesen(art, ereignis) { (this.behandler[art] || []).forEach((behandler) => behandler(ereignis)); }
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

// Karte #62, F0127 B0569 (ADR-0012 Regel 4): Folgen sperrt das Formular und gibt es wieder frei, die Eingabe bleibt.
function bildschirmZeichnen(zustand = { art: 'beitreten', name: '' }) {
  const vorher = globalThis.document;
  globalThis.document = { createElement: (tag) => new Knoten(tag) };
  try {
    const wurzel = new Knoten('div');
    const griffe = zeichnen(wurzel, zustand, { wartende: 0, beitreten: async () => {} });
    return { wurzel, griffe, feld: wurzel.finden((k) => k.id === 'lc-name'), schalter: wurzel.finden((k) => k.tag === 'button' && k.textContent === 'Beitreten'), formular: wurzel.finden((k) => k.tag === 'form') };
  } finally {
    globalThis.document = vorher;
  }
}

test('Sperren: Feld und Schalter disabled, das Formular aria-busy; die Eingabe bleibt', () => {
  const { griffe, feld, schalter, formular } = bildschirmZeichnen({ art: 'vergeben', name: 'shoff', belegtVon: 'x', seit: '2026-10-27T08:04:00.000+00:00', vorschlag: 'shoff-2' });
  feld.value = 'abc';

  griffe.sperren();

  assert.equal(feld.disabled, true);
  assert.equal(schalter.disabled, true);
  assert.equal(formular.attribute['aria-busy'], 'true');
  assert.equal(feld.value, 'abc');
});

test('Freigeben: beides wieder frei, aria-busy weg, die Eingabe bleibt', () => {
  const { griffe, feld, schalter, formular } = bildschirmZeichnen();
  feld.value = 'abc';
  griffe.sperren();

  griffe.freigeben();

  assert.equal(feld.disabled, false);
  assert.equal(schalter.disabled, false);
  assert.equal(formular.attribute['aria-busy'], undefined);
  assert.equal(feld.value, 'abc');
});

test('Freigeben nach einem Klick, der noch läuft: der Klick ist wieder möglich', async () => {
  const vorher = globalThis.document;
  globalThis.document = { createElement: (tag) => new Knoten(tag) };
  const beitritte = [];
  try {
    const wurzel = new Knoten('div');
    const griffe = zeichnen(wurzel, { art: 'beitreten', name: 'shoff' }, { wartende: 0, beitreten: async (name) => { beitritte.push(name); } });
    const formular = wurzel.finden((k) => k.tag === 'form');
    griffe.sperren();
    griffe.freigeben();
    formular.ausloesen('submit', { preventDefault() {} });
    await Promise.resolve();
    assert.deepEqual(beitritte, ['shoff']);
  } finally {
    globalThis.document = vorher;
  }
});

// ux-reviewer #62: scheitert das Folgen, soll der Fokus wieder im Feld liegen, wenn er vor dem Sperren dort lag.
function fokusVerfolgt(liegtImFeld) {
  const vorher = globalThis.document;
  const fokussiert = [];
  globalThis.document = { createElement: (tag) => new Knoten(tag), activeElement: null };
  try {
    const wurzel = new Knoten('div');
    const griffe = zeichnen(wurzel, { art: 'beitreten', name: '' }, { wartende: 0, beitreten: async () => {} });
    const feld = wurzel.finden((k) => k.id === 'lc-name');
    feld.focus = () => { fokussiert.push('feld'); };
    if (liegtImFeld) {
      globalThis.document.activeElement = feld;
    }
    griffe.sperren();
    globalThis.document.activeElement = null;
    griffe.freigeben();
    return fokussiert;
  } finally {
    globalThis.document = vorher;
  }
}

test('Freigeben bringt den Fokus ins Feld zurück, wenn er vor dem Sperren dort lag', () => {
  assert.deepEqual(fokusVerfolgt(true), ['feld']);
});

test('Freigeben holt den Fokus nicht, wenn er vor dem Sperren woanders lag', () => {
  assert.deepEqual(fokusVerfolgt(false), []);
});
