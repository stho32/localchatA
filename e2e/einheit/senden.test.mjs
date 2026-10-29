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

// B0391 (Nacharbeit #22, Code-Review B1; ADR-0037 Ergänzung Karte #22/#36, 7f): getrennt fallen Ereignisse weg; ab dem
// Wiederverbinden des Stroms kommen sie in den Verlauf, während die Warteschlange bis nach dem Nachholen ruht.
test('ab dem Wiederverbinden des Stroms werden Ereignisse angenommen, die Warteschlange ruht weiter', async () => {
  const angenommen = [];
  const verlauf = { ...verlaufErsatz(), nachrichtAnnehmen: (nachricht) => angenommen.push(nachricht.text) };
  const gesendet = [];
  const senden = sendenEinrichten({
    verlauf,
    kennung: 'anna@127.0.0.1',
    abschicken: async (channel, text) => {
      gesendet.push(text);
      return { art: 'antwort', status: 201, daten: { channel, nachricht: { zeit: 'z9', von: 'anna@127.0.0.1', text } }, token: 't' };
    },
    wiederBeitreten: async () => {},
    netzAus: () => false,
    ansagen: () => {},
  });
  senden.channelGeoeffnet('allgemein');
  const fremd = (text, zeit) => ({ channel: 'allgemein', nachricht: { zeit, von: 'beta@127.0.0.1', art: 'agent', text } });

  senden.getrennt();
  senden.senden('eigen');
  senden.empfangen(fremd('getrennt', 'z1'));
  senden.stromWieder();
  senden.empfangen(fremd('waehrend', 'z2'));
  await naechsteRunde();
  const vorDemNachholen = { angenommen: [...angenommen], gesendet: [...gesendet] };
  senden.verbunden();
  await naechsteRunde();

  assert.deepEqual(vorDemNachholen, { angenommen: ['waehrend'], gesendet: [] });
  assert.deepEqual(gesendet, ['eigen']);
});
