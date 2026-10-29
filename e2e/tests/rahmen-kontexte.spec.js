'use strict';
// Rahmen der E2E-Suite (Karte #49): ein Browser-Kontext aus browser.newContext() überlebt sein Testende nicht. Bliebe
// er offen, verbände sich sein Tab mit seinem alten Port neu; bekäme eine spätere Instanz diesen Port, träte der alte
// Tab dort still als `shoff` bei, und der Beitritt des neuen Tests scheiterte mit 409.
const { test, expect } = require('./rahmen');
const { instanzStarten, freierPort, channelAnlegen, beitretenPerApi } = require('./hilfen');
const { kontextOeffnen, imBrowserBeitreten } = require('./senden-hilfen');

/// Der Strom versucht es spätestens 10 s nach dem letzten Fehlschlag erneut (web/verbindung.js, RUECKZUG_MS); ein
/// offener alter Tab hätte binnen dieser Frist plus Beitritt die neue Instanz erreicht.
const ALTER_TAB_HAETTE_SICH_GEMELDET_MS = 12000;

test.describe.serial('Kontexte enden mit ihrem Test', () => {
  let port;

  test('Vorlauf: A tritt als shoff bei und lässt seinen Kontext offen', async ({ browser }) => {
    port = await freierPort();
    const instanz = await instanzStarten({ port, vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein') });
    const a = await kontextOeffnen(browser);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    await instanz.stoppen();
  });

  test('Port-Probe: neue Instanz auf demselben Port — kein Kontext offen, der Beitritt shoff gelingt', async ({ browser }) => {
    test.setTimeout(30000);
    expect(browser.contexts()).toHaveLength(0);
    const instanz = await instanzStarten({ port, vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein') });
    try {
      await new Promise((weiter) => setTimeout(weiter, ALTER_TAB_HAETTE_SICH_GEMELDET_MS));
      const beitritt = await beitretenPerApi(instanz.adresse, 'shoff');
      expect(beitritt.status, JSON.stringify(beitritt.daten)).toBe(201);
    } finally {
      await instanz.stoppen();
    }
  });
});
