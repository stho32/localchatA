// Unit-Tests der Senden-Verdrahtung (R00012 B0242; ADR-0037 Regel 4; node --test) mit Ersatz für Verlauf und Abruf.
import test from 'node:test';
import assert from 'node:assert/strict';
import { sendenEinrichten } from '../../web/senden.js';
import { LEER, zustellen } from '../../web/zustellung.js';
import { wiederbeitrittAnlegen } from '../../web/wiederbeitritt.js';
import { SITZUNG, NAME } from '../../web/sitzung.js';

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
    wiederBeitreten: async () => ({ art: 'kopfzeile' }),
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
    wiederBeitreten: async () => ({ art: 'kopfzeile' }),
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
    wiederBeitreten: async () => ({ art: 'kopfzeile' }),
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

// R00025 F0088 „Scheitern im Tab“ (B0412; ADR-0012 Ergänzung Karte #36, erster Test, wörtlich): „stiller Beitritt
// scheitert (409 `vergeben`) nach einer 401 beim Senden, zwei vorläufige in der Warteschlange → kein weiterer
// `POST …/nachrichten`, beide fehlgeschlagen mit dem Grund aus Regel 4, der Zustand `vergeben` geht an `zeigen`;
// dasselbe, wenn der Strom die Ablehnung zuerst meldet. Mutante „Senden scheitert still wie heute“ → rot.“ Die
// Zustellung gehört dem Tab; was start.js beim Scheitern tut, steht hier im Rückruf `gescheitert`.
const VERGEBEN = { status: 409, daten: { belegt_von: '127.0.0.9', seit: '2026-10-28T15:00:00+01:00', vorschlag: 'shoff-2' } };

function tabMitScheiterndemBeitritt(abschicken, { beitrittsAntworten = [], zustellung = { zustand: LEER } } = {}) {
  const gezeigt = [];
  const werte = new Map([[SITZUNG, 'alt'], [NAME, 'shoff']]);
  const speicher = {
    getItem: (schluessel) => (werte.has(schluessel) ? werte.get(schluessel) : null),
    setItem: (schluessel, wert) => werte.set(schluessel, wert),
    removeItem: (schluessel) => werte.delete(schluessel),
  };
  const api = {
    beitreten: async () => beitrittsAntworten.shift() || VERGEBEN,
    ich: async () => ({ status: 401 }),
  };
  const wiederBeitreten = wiederbeitrittAnlegen({
    speicher,
    api,
    sperre: (arbeit) => arbeit({ neuesToken: async () => null }),
    gelungen: () => {},
    gescheitert: (zustand) => {
      zustellung.zustand = zustellen(zustellung.zustand, { art: 'beitrittGescheitert' }).zustand;
      gezeigt.push(zustand.art);
    },
  });
  const senden = sendenEinrichten({
    verlauf: verlaufErsatz(),
    kennung: 'shoff@127.0.0.1',
    abschicken,
    wiederBeitreten,
    netzAus: () => false,
    ansagen: () => {},
    zustellung,
  });
  senden.channelGeoeffnet('allgemein');
  return { senden, zustellung, gezeigt, wiederBeitreten };
}

const ABGELEHNT = { art: 'antwort', status: 401, daten: { fehler: 'Sitzung unbekannt.' }, token: 'alt' };

test('Scheitern im Tab (Senden zuerst): kein weiterer POST, beide fehlgeschlagen, vergeben an zeigen', async () => {
  const gesendet = [];
  const tab = tabMitScheiterndemBeitritt(async (channel, text) => {
    gesendet.push(text);
    return ABGELEHNT;
  });

  tab.senden.senden('eins');
  tab.senden.senden('zwei');
  for (let runde = 0; runde < 10; runde += 1) {
    await naechsteRunde();
  }

  assert.deepEqual(gesendet, ['eins']);
  assert.deepEqual(tab.zustellung.zustand.vorlaeufige.map((v) => [v.text, v.zustand, v.grund]), [
    ['eins', 'fehlgeschlagen', 'Nicht gesendet — der Beitritt war abgelaufen.'],
    ['zwei', 'fehlgeschlagen', 'Nicht gesendet — der Beitritt war abgelaufen.'],
  ]);
  assert.deepEqual(tab.gezeigt, ['vergeben']);
});

test('Scheitern im Tab (Strom zuerst): die spätere 401 des laufenden Versuchs sendet nicht noch einmal', async () => {
  const gesendet = [];
  let antworten = null;
  const tab = tabMitScheiterndemBeitritt(async (channel, text) => {
    gesendet.push(text);
    return new Promise((liefern) => { antworten = liefern; });
  });
  tab.senden.senden('eins');
  tab.senden.senden('zwei');
  await naechsteRunde();

  await tab.wiederBeitreten('alt');
  antworten(ABGELEHNT);
  for (let runde = 0; runde < 10; runde += 1) {
    await naechsteRunde();
  }

  assert.deepEqual(gesendet, ['eins']);
  assert.deepEqual(tab.zustellung.zustand.vorlaeufige.map((v) => v.zustand), ['fehlgeschlagen', 'fehlgeschlagen']);
  assert.deepEqual(tab.gezeigt, ['vergeben']);
});

// Nacharbeit #36, ux-reviewer M1 (Abnahme-Probe, Node-Teil): die Zustellung des Tabs ist getrennt (Strom meldete die
// Trennung, dann die 401), der stille Beitritt scheitert; nach dem Beitritt unter neuem Namen richtet die neue
// Hauptansicht ihr Senden über dieselbe Zustellung ein → „Erneut senden“ liefert genau einen Auftrag mit demselben
// Schlüssel, eine neue Nachricht danach ebenso.
test('Scheitern bei getrenntem Strom: nach dem Beitritt sendet „Erneut senden“ genau einmal mit demselben Schlüssel', async () => {
  const zustellung = { zustand: { ...LEER, getrennt: true } };
  const alt = tabMitScheiterndemBeitritt(async () => ABGELEHNT, { zustellung });
  alt.senden.senden('fünf');
  await alt.wiederBeitreten('alt');
  const [wartend] = zustellung.zustand.vorlaeufige;
  assert.deepEqual([wartend.text, wartend.zustand], ['fünf', 'fehlgeschlagen']);

  const gesendet = [];
  const neu = sendenEinrichten({
    verlauf: verlaufErsatz(),
    kennung: 'shoff-2@127.0.0.1',
    abschicken: async (channel, text, schluessel) => {
      gesendet.push([text, schluessel]);
      return { art: 'antwort', status: 201, daten: { channel, nachricht: { zeit: `z${gesendet.length}`, von: 'shoff-2@127.0.0.1', text } }, token: 'neu' };
    },
    wiederBeitreten: async () => ({ art: 'kopfzeile' }),
    netzAus: () => false,
    ansagen: () => {},
    zustellung,
  });
  neu.channelGeoeffnet('allgemein');
  neu.schalter('erneut', wartend.schluessel);
  await naechsteRunde();
  neu.senden('sechs');
  await naechsteRunde();

  assert.deepEqual(gesendet.map(([text]) => text), ['fünf', 'sechs']);
  assert.equal(gesendet[0][1], wartend.schluessel);
  assert.deepEqual(zustellung.zustand.vorlaeufige, []);
});
