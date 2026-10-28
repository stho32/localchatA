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
