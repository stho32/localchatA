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

// Abnahme-Probe ux-reviewer #36 S1, wörtlich: „Probe: Textbreite ≤ clientWidth − Polster bei 320/375/414.“ Gemessen mit
// `Range.getBoundingClientRect()` über den Text des Teils; „Polster“ ist padding-left + padding-right der Leiste. Der
// Stand (breitester: „neuer Versuch in 10 s“, danach „wird verbunden …“) ist immer ungekürzt; ab 375 px auch der Anlass.
function leisteMessen() {
  const leiste = document.querySelector('.lc-center > .lc-leiste');
  const stil = getComputedStyle(leiste);
  const polster = parseFloat(stil.paddingLeft) + parseFloat(stil.paddingRight);
  const textbreite = (knoten) => {
    const bereich = document.createRange();
    bereich.selectNodeContents(knoten);
    return bereich.getBoundingClientRect().width;
  };
  const anlass = leiste.querySelector('.lc-leiste-anlass');
  const stand = leiste.querySelector('.lc-leiste-stand');
  return {
    text: leiste.textContent,
    verfuegbar: leiste.getBoundingClientRect().width - polster,
    gesamt: textbreite(leiste),
    anlassSichtbar: anlass.getBoundingClientRect().width,
    anlassGekuerzt: anlass.scrollWidth > anlass.clientWidth,
    standBreite: stand ? textbreite(stand) : 0,
    standGekuerzt: stand ? stand.scrollWidth > stand.clientWidth : false,
    hoehe: leiste.getBoundingClientRect().height,
  };
}

async function imRaster(page, text) {
  await page.waitForFunction((muster) => new RegExp(muster).test(document.querySelector('.lc-center > .lc-leiste')?.textContent ?? ''),
    text, { polling: 'raf', timeout: 60000 });
  return page.evaluate(leisteMessen);
}

for (const breite of [320, 375, 414]) {
  test(`Countdown bei Telefon ${breite} px: der Stand steht ungekürzt da`, async ({ browser }) => {
    test.setTimeout(120000);
    await instanzMitChannel();
    const kontext = await browser.newContext({ viewport: { width: breite, height: 740 }, hasTouch: true, isMobile: true });
    kontexte.push(kontext);
    const page = await kontext.newPage();
    const fehler = fehlerSammeln(page, { serverWegErwartet: true });
    await imBrowserBeitreten(page, instanz.adresse, 'shoff');
    await instanz.beenden();

    const countdown = await imRaster(page, 'neuer Versuch in 10 s$');
    // „wird verbunden …“ steht nur einen Augenblick (der nächste Versuch scheitert sofort): den Text setzt der echte
    // Code der Leiste (`leisteFuellen`) in die echte Leiste, gemessen wird mit dem echten CSS.
    await page.evaluate(async (text) => {
      const { leisteFuellen } = await import('/web/verbindungsleiste.js');
      const leiste = document.querySelector('.lc-center > .lc-leiste');
      leisteFuellen(leiste, text);
    }, '⚠ Verbindung verloren · wird verbunden …');
    const verbindet = await page.evaluate(leisteMessen);

    for (const messung of [countdown, verbindet]) {
      expect(messung.text).toMatch(/^⚠ Verbindung verloren · (neuer Versuch in 10 s|wird verbunden …)$/);
      expect(messung.standGekuerzt).toBe(false);
      expect(messung.standBreite).toBeLessThanOrEqual(messung.verfuegbar - messung.anlassSichtbar);
      expect(messung.hoehe).toBe(countdown.hoehe);
      if (breite === 320) {
        expect(messung.anlassGekuerzt, 'bei 320 px kürzt der Anlass mit „…“').toBe(true);
      }
      if (breite >= 375) {
        expect(messung.anlassGekuerzt).toBe(false);
        expect(messung.gesamt).toBeLessThanOrEqual(messung.verfuegbar);
      }
    }
    expect(fehler).toEqual([]);
  });
}
