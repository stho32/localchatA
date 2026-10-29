// Unit-Tests der Zustellung im Tab (R00011 F0043 „Zustellung“, Z4; B0218; ADR-0037 Regeln 1–4; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { LEER, zustellen } from '../../web/zustellung.js';

const ICH = 'shoff@127.0.0.1';
const nachricht = (zeit, text, von = ICH) => ({ zeit, von, art: 'mensch', text });
const senden = (text, schluessel = `k-${text}`, channel = 'allgemein') => ({ art: 'senden', channel, text, schluessel });
const antwort = (schluessel, status, daten) => ({ art: 'antwort', schluessel, status, daten });
const ereignis = (n, channel = 'allgemein') => ({ art: 'ereignis', kennung: ICH, channel, nachricht: n });

// Wendet die Ereignisse nacheinander an; liefert den Endzustand und alle Ausgaben.
function ablauf(...ereignisse) {
  let zustand = LEER;
  const ausgaben = [];
  for (const einzeln of ereignisse) {
    const ergebnis = zustellen(zustand, einzeln);
    zustand = ergebnis.zustand;
    ausgaben.push(ergebnis);
  }
  return { zustand, ausgaben };
}

const bestaetigte = (ausgaben) => ausgaben.filter((a) => a.bestaetigt !== null).map((a) => a.bestaetigt.nachricht.zeit);

test('(a) senden legt eine vorläufige an und vergibt den Auftrag; 201 bestätigt mit der Zeit', () => {
  const { zustand, ausgaben } = ablauf(senden('x'), antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }));

  assert.deepEqual(ausgaben[0].auftrag, { schluessel: 'k-x', channel: 'allgemein', text: 'x' });
  assert.equal(ausgaben[0].zustand.vorlaeufige.length, 1);
  assert.equal(ausgaben[0].zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.deepEqual(zustand.vorlaeufige, []);
  assert.deepEqual(bestaetigte(ausgaben), ['5']);
});

test('(b) Ereignis vor der Antwort ersetzt die vorläufige; die Antwort danach ändert nichts — Zeit 5 genau einmal', () => {
  const { zustand, ausgaben } = ablauf(
    senden('x'),
    ereignis(nachricht('5', 'x')),
    antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }),
  );

  assert.deepEqual(zustand.vorlaeufige, []);
  assert.deepEqual(bestaetigte(ausgaben), ['5']);
  assert.equal(ausgaben[1].fremd, null);
  assert.equal(ausgaben[2].fremd, null);
});

test('(c) ein Sendeauftrag zugleich: y wartet auf die Antwort für x', () => {
  const { ausgaben } = ablauf(
    senden('x'),
    senden('y'),
    antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }),
  );

  assert.deepEqual(ausgaben[0].auftrag, { schluessel: 'k-x', channel: 'allgemein', text: 'x' });
  assert.equal(ausgaben[1].auftrag, null);
  assert.deepEqual(ausgaben[2].auftrag, { schluessel: 'k-y', channel: 'allgemein', text: 'y' });
});

test('(d) eigene Kennung mit anderem Text bestätigt nicht und geht als fremde weiter', () => {
  const { zustand, ausgaben } = ablauf(senden('x'), ereignis(nachricht('4', 'z')));

  assert.equal(ausgaben[1].bestaetigt, null);
  assert.deepEqual(ausgaben[1].fremd, { channel: 'allgemein', nachricht: nachricht('4', 'z') });
  assert.equal(zustand.vorlaeufige[0].zustand, 'wird gesendet');
});

// R00020 F0071 „Bestätigen nur im eigenen Channel“ (B0338, Code-Review #8 A1).
test('eigene Kennung und gleicher Text aus einem anderen Channel bestätigt nicht; aus dem eigenen Channel schon', () => {
  const ausTechnik = nachricht('4', 'x');
  const ausAllgemein = nachricht('5', 'x');
  const { zustand, ausgaben } = ablauf(senden('x'), ereignis(ausTechnik, 'technik'), ereignis(ausAllgemein, 'allgemein'));

  assert.equal(ausgaben[1].bestaetigt, null);
  assert.deepEqual(ausgaben[1].fremd, { channel: 'technik', nachricht: ausTechnik });
  assert.deepEqual(ausgaben[1].zustand.vorlaeufige.map((v) => [v.text, v.zustand]), [['x', 'wird gesendet']]);
  assert.deepEqual(ausgaben[2].bestaetigt, { channel: 'allgemein', nachricht: ausAllgemein });
  assert.equal(ausgaben[2].fremd, null);
  assert.deepEqual(zustand.vorlaeufige, []);
});

test('(e) Fehlerantworten machen fehlgeschlagen mit Grund, und der nächste Auftrag läuft', () => {
  const { zustand, ausgaben } = ablauf(
    senden('a'),
    senden('b'),
    senden('c'),
    senden('d'),
    antwort('k-a', 422, { fehler: 'Die Nachricht ist leer.', regel: 'leer' }),
    antwort('k-b', 500, { fehler: 'Interner Fehler' }),
    { art: 'netzfehler', schluessel: 'k-c' },
  );

  assert.deepEqual(
    zustand.vorlaeufige.map((v) => [v.text, v.zustand, v.grund]),
    [
      ['a', 'fehlgeschlagen', 'Die Nachricht ist leer.'],
      ['b', 'fehlgeschlagen', 'Nicht gespeichert — der Server konnte nicht schreiben.'],
      ['c', 'fehlgeschlagen', 'Keine Antwort vom Server.'],
      ['d', 'wird gesendet', undefined],
    ],
  );
  assert.deepEqual(ausgaben.slice(4).map((a) => a.auftrag && a.auftrag.text), ['b', 'c', 'd']);
});

test('der Schlüssel eines Auftrags bleibt derselbe; vorläufige gehören zu ihrem Channel', () => {
  const { ausgaben } = ablauf(senden('x', 'u-1', 'technik'), senden('y', 'u-2', 'allgemein'), { art: 'netzfehler', schluessel: 'u-1' });

  assert.equal(ausgaben[0].auftrag.schluessel, 'u-1');
  assert.equal(ausgaben[0].auftrag.channel, 'technik');
  assert.deepEqual(ausgaben[2].auftrag, { schluessel: 'u-2', channel: 'allgemein', text: 'y' });
});

test('ein Ereignis eines anderen Absenders ist fremd; eine schon bestätigte Zeit kommt nicht noch einmal', () => {
  const { ausgaben } = ablauf(
    ereignis(nachricht('3', 'hallo', 'beta@127.0.0.1')),
    senden('x'),
    antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }),
    ereignis(nachricht('5', 'x')),
  );

  assert.deepEqual(ausgaben[0].fremd, { channel: 'allgemein', nachricht: nachricht('3', 'hallo', 'beta@127.0.0.1') });
  assert.equal(ausgaben[3].fremd, null);
  assert.equal(ausgaben[3].bestaetigt, null);
});

test('der Text wird wie vom Server verglichen: \\r\\n als \\n, ohne abschließende Leerzeilen', () => {
  const { zustand, ausgaben } = ablauf(senden('Hallo\r\nWelt\n\n'), ereignis(nachricht('6', 'Hallo\nWelt')));

  assert.deepEqual(zustand.vorlaeufige, []);
  assert.deepEqual(bestaetigte(ausgaben), ['6']);
});

// R00012 F0046 „Zustellung erweitert“ (B0239; ADR-0037 Regel 4): Strom getrennt, Zeitgrenze, 401, „Erneut senden“,
// „Verwerfen“. Die Zeitgrenze selbst (9 999 / 10 000 ms) prüft api.test.mjs; hier, was danach geschieht.
const GETRENNT = { art: 'getrennt' };
const VERBUNDEN = { art: 'verbunden' };
const auftraege = (ausgaben) => ausgaben.map((a) => a.auftrag && a.auftrag.text);

test('(a) bei getrenntem Strom kein Auftrag, die Nachricht bleibt vorläufig; wieder verbunden → Auftrag mit ihrem Schlüssel', () => {
  const { zustand, ausgaben } = ablauf(GETRENNT, senden('x'), VERBUNDEN);

  assert.equal(ausgaben[1].auftrag, null);
  assert.equal(ausgaben[1].zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.deepEqual(ausgaben[2].auftrag, { schluessel: 'k-x', channel: 'allgemein', text: 'x' });
  assert.equal(zustand.laufend, 'k-x');
});

test('(b) Auftrag läuft, Strom trennt, Netzfehler → bleibt vorläufig; wieder verbunden → derselbe Auftrag mit demselben Schlüssel', () => {
  const { ausgaben } = ablauf(senden('x'), GETRENNT, { art: 'netzfehler', schluessel: 'k-x' }, VERBUNDEN);

  assert.equal(ausgaben[2].zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.equal(ausgaben[2].auftrag, null);
  assert.deepEqual(ausgaben[3].auftrag, ausgaben[0].auftrag);
});

test('(b) eine Zeitüberschreitung bei getrenntem Strom lässt die Nachricht vorläufig', () => {
  const { zustand } = ablauf(senden('x'), GETRENNT, { art: 'zeitueberschreitung', schluessel: 'k-x' });

  assert.equal(zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.equal(zustand.laufend, null);
});

test('(b) bei getrenntem Strom startet auch nach einer Antwort kein neuer Auftrag', () => {
  const { ausgaben } = ablauf(
    senden('x'), senden('y'), GETRENNT, antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }), VERBUNDEN,
  );

  assert.equal(ausgaben[3].auftrag, null);
  assert.equal(ausgaben[4].auftrag.text, 'y');
});

test('(c) Zeitüberschreitung bei verbundenem Strom → fehlgeschlagen „Keine Antwort vom Server.“, der nächste Auftrag läuft', () => {
  const { zustand, ausgaben } = ablauf(senden('x'), senden('y'), { art: 'zeitueberschreitung', schluessel: 'k-x' });

  assert.deepEqual([zustand.vorlaeufige[0].zustand, zustand.vorlaeufige[0].grund], ['fehlgeschlagen', 'Keine Antwort vom Server.']);
  assert.equal(ausgaben[2].auftrag.text, 'y');
});

test('(d) 401 → Wiederbeitritt und ein Versuch mit demselben Schlüssel; die zweite 401 → fehlgeschlagen mit dem fehler der Antwort', () => {
  const abgelehnt = { fehler: 'Sitzung unbekannt — bitte neu beitreten.' };
  const { zustand, ausgaben } = ablauf(senden('x'), antwort('k-x', 401, abgelehnt), antwort('k-x', 401, abgelehnt));

  assert.equal(ausgaben[1].wiederbeitreten, true);
  assert.deepEqual(ausgaben[1].auftrag, { schluessel: 'k-x', channel: 'allgemein', text: 'x' });
  assert.equal(ausgaben[1].zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.equal(ausgaben[2].wiederbeitreten, false);
  assert.equal(ausgaben[2].auftrag, null);
  assert.deepEqual([zustand.vorlaeufige[0].zustand, zustand.vorlaeufige[0].grund], ['fehlgeschlagen', abgelehnt.fehler]);
});

test('(d) 401 und danach 201 → bestätigt, kein Fehlschlag', () => {
  const { zustand, ausgaben } = ablauf(senden('x'), antwort('k-x', 401, {}), antwort('k-x', 201, { channel: 'allgemein', nachricht: nachricht('5', 'x') }));

  assert.deepEqual(zustand.vorlaeufige, []);
  assert.deepEqual(bestaetigte(ausgaben), ['5']);
});

test('(e) „Erneut senden“ stellt ans Ende der Warteschlange mit demselben Schlüssel; „Verwerfen“ entfernt', () => {
  const { zustand, ausgaben } = ablauf(
    senden('a'), senden('b'), senden('c'),
    { art: 'netzfehler', schluessel: 'k-a' },
    { art: 'netzfehler', schluessel: 'k-b' },
    { art: 'erneut', schluessel: 'k-a' },
    { art: 'verwerfen', schluessel: 'k-b' },
    antwort('k-c', 201, { channel: 'allgemein', nachricht: nachricht('5', 'c') }),
  );

  assert.equal(ausgaben[5].auftrag, null, 'c läuft noch');
  assert.deepEqual(ausgaben[5].zustand.vorlaeufige.map((v) => [v.text, v.zustand]), [['b', 'fehlgeschlagen'], ['c', 'wird gesendet'], ['a', 'wird gesendet']]);
  assert.deepEqual(ausgaben[6].zustand.vorlaeufige.map((v) => v.text), ['c', 'a']);
  assert.deepEqual(ausgaben[7].auftrag, { schluessel: 'k-a', channel: 'allgemein', text: 'a' });
  assert.deepEqual(zustand.vorlaeufige.map((v) => [v.text, v.zustand, v.grund]), [['a', 'wird gesendet', undefined]]);
});

test('(e) „Erneut senden“ ohne laufenden Auftrag sendet sofort; nach einer 401 gilt wieder ein Versuch', () => {
  const { ausgaben } = ablauf(
    senden('x'), antwort('k-x', 401, {}), antwort('k-x', 401, { fehler: 'nein' }), { art: 'erneut', schluessel: 'k-x' }, antwort('k-x', 401, {}),
  );

  assert.deepEqual(ausgaben[3].auftrag, { schluessel: 'k-x', channel: 'allgemein', text: 'x' });
  assert.equal(ausgaben[4].wiederbeitreten, true);
});

test('(e) „Erneut senden“ bei getrenntem Strom wartet', () => {
  const { ausgaben } = ablauf(senden('x'), { art: 'netzfehler', schluessel: 'k-x' }, GETRENNT, { art: 'erneut', schluessel: 'k-x' }, VERBUNDEN);

  assert.equal(ausgaben[3].auftrag, null);
  assert.equal(ausgaben[3].zustand.vorlaeufige[0].zustand, 'wird gesendet');
  assert.equal(ausgaben[4].auftrag.text, 'x');
});

test('(f) das eigene Ereignis kommt vor dem Netzfehler → bestätigt; der Netzfehler danach ändert nichts', () => {
  const { zustand, ausgaben } = ablauf(senden('x'), ereignis(nachricht('5', 'x')), { art: 'netzfehler', schluessel: 'k-x' });

  assert.deepEqual(bestaetigte(ausgaben), ['5']);
  assert.deepEqual(zustand.vorlaeufige, []);
  assert.equal(zustand.laufend, null);
  assert.deepEqual(auftraege(ausgaben), ['x', null, null]);
});
