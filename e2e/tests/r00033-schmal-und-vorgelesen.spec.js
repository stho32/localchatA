'use strict';
// R00033 F0124 „Senden bei Störungen nach den Reviews #36“, gegen das echte Programm: die Wartezeile des
// Beitreten-Bildschirms ist die Beschreibung des Namensfelds (B0551, ux-reviewer #36 S3) und der Countdown der
// Verbindungsleiste bleibt bei 320 px ganz (B0550, ux-reviewer #36 S1). A ist `shoff`.
const { test, expect } = require('./rahmen');
const { JETZT, kontextOeffnen, imBrowserBeitreten, beitreten, eingabe } = require('./senden-hilfen');
const { leiste } = require('./r00012-hilfen');
const { instanzStarten, freierPort, channelAnlegen, fehlerSammeln } = require('./hilfen');

let instanz;
let kontexte = [];
test.afterEach(async () => {
  await Promise.all(kontexte.map((kontext) => kontext.close()));
  kontexte = [];
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const BEITRETEN = '/api/beitreten';
const STROM = '/api/ereignisse';

async function instanzMitChannel() {
  instanz = await instanzStarten({
    port: await freierPort(),
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein'),
  });
}

// Weg „Scheitern nach Neustart (a) Strom zuerst“ aus R00025 F0088: Instanz weg, Name auf dem neuen Lauf von anderer
// Herkunft belegt, eine Nachricht kann nicht gesendet werden → der Strom meldet 401, der Beitreten-Bildschirm steht.
test('Scheitern nach Neustart (a): die Wartezeile gehört zur Beschreibung des Namensfelds', async ({ browser }) => {
  test.setTimeout(60000);
  await instanzMitChannel();
  const a = await kontextOeffnen(browser, { serverWegErwartet: true, zusaetzlich: { [BEITRETEN]: [409], [STROM]: [401] } });
  kontexte.push(a.kontext);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await instanz.beenden();
  await expect(leiste(a.page)).toHaveText(/^⚠ Verbindung verloren/);
  await instanz.starten();
  await beitreten(instanz.adresse, 'shoff', { art: 'mensch', herkunft: '127.0.0.2' });
  await eingabe(a.page).fill('fünf');
  await eingabe(a.page).press('Enter');

  const name = a.page.getByLabel('Name');
  await expect(name).toBeVisible({ timeout: 20000 });

  await expect(name).toHaveAccessibleDescription(/1 ungesendete Nachricht wartet auf den Beitritt — Neuladen verwirft sie\./);
  expect(a.fehler).toEqual([]);
});
