// Unit-Tests der Tippmeldung (R00045 F0161 B0688, US-4; ADR-0051 Regel 8; node --test, gestellte Zeit).
import test from 'node:test';
import assert from 'node:assert/strict';
import { TIPPEN_LEER, tippen, tippenAnlegen } from '../../web/tippen.js';

/// Spielt Ereignisse zu Zeitpunkten (ms) durch und sammelt die Aufrufe als 'setzen allgemein' | 'loeschen allgemein'.
function abspielen(schritte, zustand = TIPPEN_LEER) {
  const aufrufe = [];
  let aktuell = zustand;
  for (const [jetztMs, ereignis] of schritte) {
    const ergebnis = tippen(aktuell, ereignis, jetztMs);
    aktuell = ergebnis.zustand;
    if (ergebnis.aufruf !== null) {
      aufrufe.push(`${ergebnis.aufruf.art} ${ergebnis.aufruf.channel}@${jetztMs}`);
    }
  }
  return { aufrufe, zustand: aktuell };
}

const eingabe = (text, channel = 'allgemein') => ({ art: 'eingabe', channel, text });

test('erste Eingabe in ein danach nichtleeres Feld meldet sofort', () => {
  assert.deepEqual(abspielen([[1000, eingabe('h')]]).aufrufe, ['setzen allgemein@1000']);
});

test('weiteres Tippen meldet höchstens alle 3 s', () => {
  const { aufrufe } = abspielen([
    [0, eingabe('h')], [1000, eingabe('ha')], [2999, eingabe('hal')], [3000, eingabe('hall')], [5999, eingabe('hallo')],
    [6000, eingabe('hallo ')],
  ]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'setzen allgemein@3000', 'setzen allgemein@6000']);
});

test('10 s ununterbrochen tippen (alle 100 ms eine Taste) → höchstens ⌈10 / 3⌉ + 1 = 5 Meldungen', () => {
  const schritte = [];
  let text = '';
  for (let ms = 0; ms <= 10000; ms += 100) {
    text += 'x';
    schritte.push([ms, eingabe(text)]);
  }
  const { aufrufe } = abspielen(schritte);
  assert.ok(aufrufe.length <= 5, aufrufe.join(', '));
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'setzen allgemein@3000', 'setzen allgemein@6000', 'setzen allgemein@9000']);
});

test('Eingabe, nach der das Feld nur Leerraum enthält, meldet nicht', () => {
  for (const leer of ['', ' ', '\n\t', ' ', ' ']) {
    assert.deepEqual(abspielen([[0, eingabe(leer)]]).aufrufe, [], JSON.stringify(leer));
  }
});

test('U+200B ist kein Leerraum (wie die Textregel) und meldet', () => {
  assert.deepEqual(abspielen([[0, eingabe('​')]]).aufrufe, ['setzen allgemein@0']);
});

test('Feld wird leer, während eine Meldung gilt → löschen; das nächste Setzen ist wieder sofort erlaubt', () => {
  const { aufrufe } = abspielen([[0, eingabe('ha')], [500, eingabe('')], [700, eingabe('n')]]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'loeschen allgemein@500', 'setzen allgemein@700']);
});

test('Feld wird leer ohne geltende Meldung → kein Aufruf (nie gemeldet, oder nach 6 s abgelaufen)', () => {
  assert.deepEqual(abspielen([[0, eingabe('')]]).aufrufe, []);
  const abgelaufen = abspielen([[0, eingabe('h')], [6000, eingabe('')]]);
  assert.deepEqual(abgelaufen.aufrufe, ['setzen allgemein@0']);
  const geradeNoch = abspielen([[0, eingabe('h')], [5999, eingabe('')]]);
  assert.deepEqual(geradeNoch.aufrufe, ['setzen allgemein@0', 'loeschen allgemein@5999']);
});

test('ein zweites Leeren löscht nicht noch einmal', () => {
  const { aufrufe } = abspielen([[0, eingabe('h')], [100, eingabe('')], [200, eingabe(' ')]]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'loeschen allgemein@100']);
});

test('nach dem Senden kein Löschen, das nächste Setzen ist sofort erlaubt', () => {
  const { aufrufe } = abspielen([
    [0, eingabe('hallo')], [1000, { art: 'gesendet', channel: 'allgemein' }], [1100, eingabe('')], [1200, eingabe('n')],
  ]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'setzen allgemein@1200']);
});

test('Channel-Wechsel löscht im alten Channel, wenn dort eine Meldung gilt; im neuen meldet die erste Eingabe sofort', () => {
  const { aufrufe } = abspielen([
    [0, eingabe('h')], [1000, { art: 'gewechselt', channel: 'allgemein' }], [1500, eingabe('t', 'technik')],
  ]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'loeschen allgemein@1000', 'setzen technik@1500']);
});

test('Channel-Wechsel ohne geltende Meldung → kein Aufruf', () => {
  assert.deepEqual(abspielen([[0, { art: 'gewechselt', channel: 'allgemein' }]]).aufrufe, []);
  const abgelaufen = abspielen([[0, eingabe('h')], [7000, { art: 'gewechselt', channel: 'allgemein' }]]);
  assert.deepEqual(abgelaufen.aufrufe, ['setzen allgemein@0']);
});

test('nach 429 wird bis Retry-After nicht gemeldet, danach wieder', () => {
  const { aufrufe } = abspielen([
    [0, eingabe('h')], [10, { art: 'gebremst', wartenS: 2 }], [2009, eingabe('ha')], [3000, eingabe('hal')],
  ]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'setzen allgemein@3000']);
});

test('gebremst: auch das erste Setzen nach einem Ende wartet bis Retry-After', () => {
  const { aufrufe } = abspielen([
    [0, eingabe('h')], [10, { art: 'gebremst', wartenS: 1 }], [20, { art: 'gesendet', channel: 'allgemein' }],
    [500, eingabe('n')], [1010, eingabe('ne')],
  ]);
  assert.deepEqual(aufrufe, ['setzen allgemein@0', 'setzen allgemein@1010']);
});

test('ein unbekanntes Ereignis ist ein Fehler, kein stilles Nichts', () => {
  assert.throws(() => tippen(TIPPEN_LEER, { art: 'fokus' }, 0), /unbekanntes Ereignis fokus/);
});

// Die Verdrahtung: tippenAnlegen ruft `melden` je Aufruf und hält den Zustand; 429 sperrt bis Retry-After.
function verdrahtet(antworten = []) {
  let jetztMs = 0;
  const gemeldet = [];
  const griffe = tippenAnlegen({
    melden: async (aufruf) => {
      gemeldet.push(`${aufruf.art} ${aufruf.channel}`);
      return antworten.shift() || { art: 'antwort', status: 200, wartenS: null };
    },
    jetzt: () => jetztMs,
  });
  return { griffe, gemeldet, stellen: (ms) => { jetztMs = ms; } };
}

test('tippenAnlegen meldet über melden und führt den Zustand', async () => {
  const welt = verdrahtet();
  welt.griffe.eingabe('allgemein', 'h');
  welt.stellen(1000);
  welt.griffe.eingabe('allgemein', 'ha');
  welt.griffe.gewechselt('allgemein');
  welt.griffe.eingabe('technik', 'x');
  welt.griffe.gesendet('technik');
  welt.griffe.eingabe('technik', '');
  await Promise.resolve();
  assert.deepEqual(welt.gemeldet, ['setzen allgemein', 'loeschen allgemein', 'setzen technik']);
});

test('tippenAnlegen: 429 mit Retry-After sperrt das Melden; ein Netzfehler bleibt ohne Folgen', async () => {
  const welt = verdrahtet([{ art: 'antwort', status: 429, wartenS: 2 }, { art: 'netzfehler' }]);
  welt.griffe.eingabe('allgemein', 'h');
  await new Promise((weiter) => setImmediate(weiter));
  welt.stellen(3500);
  welt.griffe.eingabe('allgemein', 'ha');
  await new Promise((weiter) => setImmediate(weiter));
  welt.stellen(7000);
  welt.griffe.eingabe('allgemein', 'hal');
  assert.deepEqual(welt.gemeldet, ['setzen allgemein', 'setzen allgemein', 'setzen allgemein']);
  const zweite = verdrahtet([{ art: 'antwort', status: 429, wartenS: 2 }]);
  zweite.griffe.eingabe('allgemein', 'h');
  await new Promise((weiter) => setImmediate(weiter));
  zweite.griffe.gesendet('allgemein');
  zweite.stellen(1999);
  zweite.griffe.eingabe('allgemein', 'n');
  assert.deepEqual(zweite.gemeldet, ['setzen allgemein']);
});
