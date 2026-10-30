'use strict';
// R00050 F0178 Nacharbeit S3 (Code-Review #13 F0178, Abnahme-Probe wörtlich): Nachladen im Direktgespräch wie im Channel.
// (a) anna hat das Gespräch mit ben offen, ihr Netz ist aus, ben sendet g1–g3 per POST /api/direkt/anna@127.0.0.1/nachrichten,
// Netz an → g1–g3 je genau einmal im Verlauf, Leiste „Wieder verbunden · 3 nachgeladen“, keine Konsolenfehler.
// (b) 130 Nachrichten ben → anna, anna öffnet #/d/ben… → 100 sichtbar, nach scrollTop = 0 alle 130, m000 vorhanden.
// Gegen das echte Programm.
const fs = require('fs');
const http = require('http');
const path = require('path');
const { test, expect } = require('./rahmen');
const { ANNA, BEN, direktStarten, direktSenden, direktOeffnen, verlaufMit, gespraechOffen } = require('./r00050-direkt-hilfen');
const { leiste, netzAus, netzAn } = require('./r00012-hilfen');

let instanz;
let zeuge = null;
test.afterEach(async () => {
  if (zeuge !== null) {
    zeuge.schliessen();
    zeuge = null;
  }
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

/// Der Strom von ben als Zeuge: er bekommt jede Direktnachricht des Paars, auch seine eigene — `bisGesehen(text)` löst
/// auf, sobald ein Ereignis mit diesem Text verteilt ist.
function stromVon(adresse, token, herkunft) {
  const ziel = new URL('api/ereignisse', adresse);
  let gelesen = '';
  const wartende = [];
  return new Promise((offen, gescheitert) => {
    const anfrage = http.get({
      host: ziel.hostname, port: ziel.port, path: ziel.pathname, localAddress: herkunft,
      headers: { 'X-LocalchatA-Sitzung': token },
    }, (antwort) => {
      antwort.setEncoding('utf8');
      antwort.on('data', (stueck) => {
        gelesen += stueck;
        wartende.filter(({ text }) => gelesen.includes(text)).forEach(({ weiter }) => weiter());
      });
      offen({
        schliessen: () => anfrage.destroy(),
        bisGesehen: (text) => new Promise((weiter) => {
          if (gelesen.includes(text)) {
            weiter();
            return;
          }
          wartende.push({ text, weiter });
        }),
      });
    });
    anfrage.once('error', gescheitert);
  });
}

/// Legt vor dem Start `direkt/anna@127.0.0.1+ben@127.0.0.3/2026-10-28.log` mit `anzahl` Nachrichten von ben an (m000 …,
/// je eine Minute, vor der Testuhr 15:00) — über die API bremste das Schreibkonto nach 30 auf eine je Sekunde.
function gespraechMitBen(anzahl) {
  return (ordner) => {
    const saetze = Array.from({ length: anzahl }, (_, nummer) => {
      const minute = 9 * 60 + nummer;
      const zeit = `2026-10-28T${String(Math.floor(minute / 60)).padStart(2, '0')}:${String(minute % 60).padStart(2, '0')}:00.000+01:00`;
      return `@ ${zeit} ${BEN}\n  m${String(nummer).padStart(3, '0')}\n\n`;
    });
    const paar = path.join(ordner, 'direkt', `${ANNA}+${BEN}`);
    fs.mkdirSync(paar, { recursive: true });
    fs.writeFileSync(path.join(paar, '2026-10-28.log'), saetze.join(''));
  };
}

const texte = (page) => verlaufMit(page, BEN).locator('.lc-msg:not(.lc-vorlaeufig) .lc-body');

test('Nachladen nach Trennung (S3 a): g1–g3 je genau einmal, „Wieder verbunden · 3 nachgeladen“, keine Konsolenfehler', async ({ browser }) => {
  const gestartet = await direktStarten(browser);
  instanz = gestartet.instanz;
  const { anna, token } = gestartet;
  const anna2 = { kontext: anna.kontext, page: anna.page };
  expect((await direktSenden(instanz.adresse, token.ben, ANNA, 'davor', '127.0.0.3')).status).toBe(201);
  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);
  await expect(texte(anna.page)).toHaveText(['davor']);
  zeuge = await stromVon(instanz.adresse, token.ben, '127.0.0.3');

  await netzAus(anna2);
  for (const text of ['g1', 'g2', 'g3']) {
    expect((await direktSenden(instanz.adresse, token.ben, ANNA, text, '127.0.0.3')).status).toBe(201);
  }
  await zeuge.bisGesehen('"g3"');
  await netzAn(anna2);

  await expect(leiste(anna.page)).toHaveText(/Wieder verbunden · 3 nachgeladen/);
  await expect(texte(anna.page)).toHaveText(['davor', 'g1', 'g2', 'g3']);
  expect(anna.fehler).toEqual([]);
});

test('Nachladen älterer (S3 b): 130 Nachrichten — 100 beim Öffnen, nach scrollTop = 0 alle 130 mit m000', async ({ browser }) => {
  const gestartet = await direktStarten(browser, { vorbereiten: gespraechMitBen(130) });
  instanz = gestartet.instanz;
  const { anna } = gestartet;

  await direktOeffnen(anna.page, BEN);
  await gespraechOffen(anna.page, BEN);
  await expect(texte(anna.page)).toHaveCount(100);
  await expect(texte(anna.page).last()).toHaveText('m129');
  await verlaufMit(anna.page, BEN).evaluate((log) => { log.scrollTop = 0; });

  await expect(texte(anna.page)).toHaveCount(130);
  await expect(texte(anna.page).filter({ hasText: /^m000$/ })).toHaveCount(1);
  expect(anna.fehler).toEqual([]);
});
