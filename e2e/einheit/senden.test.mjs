// Unit-Tests der Senden-Verdrahtung (R00012 B0242; ADR-0037 Regel 4; node --test) mit Ersatz für Verlauf und Abruf.
import test from 'node:test';
import assert from 'node:assert/strict';
import { sendenEinrichten } from '../../web/senden.js';

function verlaufErsatz() {
  const gezeigt = [];
  return {
    gezeigt,
    nachrichtAnnehmen: () => {},
    vorlaeufigeZeigen: (liste) => gezeigt.push(liste),
    zumEndeFuerSenden: () => {},
  };
}

const naechsteRunde = () => new Promise((weiter) => setImmediate(weiter));

// Meldet das Netz des Tabs „aus“, bevor das Ereignis `offline` die Verbindung als getrennt gemeldet hat, ist ein
// Netzfehler trotzdem kein Fehlschlag: die Nachricht bleibt „wird gesendet“ und die Ansage schweigt.
test('ein Netzfehler bei ausgeschaltetem Netz lässt die Nachricht vorläufig, auch vor dem Ereignis offline', async () => {
  const verlauf = verlaufErsatz();
  const ansagen = [];
  const senden = sendenEinrichten({
    verlauf,
    kennung: 'shoff@127.0.0.1',
    abschicken: async () => ({ art: 'netzfehler', token: 't' }),
    wiederBeitreten: async () => {},
    netzAus: () => true,
    ansagen: (ereignis) => ansagen.push(ereignis),
  });
  senden.channelGeoeffnet('allgemein');

  senden.senden('x');
  await naechsteRunde();

  const zuletzt = verlauf.gezeigt[verlauf.gezeigt.length - 1];
  assert.deepEqual(zuletzt.map((vorlaeufig) => vorlaeufig.zustand), ['wird gesendet']);
  assert.deepEqual(ansagen, []);
});

test('ein Netzfehler bei eingeschaltetem Netz ist ein Fehlschlag und wird sofort angesagt', async () => {
  const verlauf = verlaufErsatz();
  const ansagen = [];
  const senden = sendenEinrichten({
    verlauf,
    kennung: 'shoff@127.0.0.1',
    abschicken: async () => ({ art: 'netzfehler', token: 't' }),
    wiederBeitreten: async () => {},
    netzAus: () => false,
    ansagen: (ereignis) => ansagen.push(ereignis),
  });
  senden.channelGeoeffnet('allgemein');

  senden.senden('x');
  await naechsteRunde();

  const zuletzt = verlauf.gezeigt[verlauf.gezeigt.length - 1];
  assert.deepEqual(zuletzt.map((vorlaeufig) => vorlaeufig.zustand), ['fehlgeschlagen']);
  assert.deepEqual(ansagen, [{ art: 'fehlschlag', grund: 'Keine Antwort vom Server.' }]);
});
