'use strict';
// R00041 F0186 B0802 (Karte #85, ux-reviewer-Vorprüfung): der Hinweis am Bearbeiten-Feld (#lc-bearbeiten-hinweis) ist selbst die
// Live-Region — aria-live (nicht off) oder role="alert", über aria-describedby mit dem Feld verbunden und schon vor seinem
// Text im DOM; die Ansage-Region der Seite bleibt dafür leer. Jede Abweisung wird genau einmal angesagt, auch eine zweite
// mit demselben Text (erst leeren, dann setzen); nach einer Texteingabe verschwindet ein Hinweis, der nicht mehr zutrifft.
// Mutanten, die rot werden müssen: aria-live fehlt; zweites Setzen desselben Texts ohne Leeren; Hinweis bleibt nach Eingabe.
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen } = require('./hilfen');
const { JETZT, sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen, imBrowserSenden, nachrichtMitZeit, ansagenMitschreiben } = require('./r00041-hilfen');
const { feld, feldPerTaste } = require('./r00041-feld-hilfen');

const QUOTE_ORDNER = path.resolve(__dirname, '../../target/debug/examples/quote-ordner');
const BEARBEITEN = '/api/channels/allgemein/nachrichten/bearbeiten';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const hinweisAm = (page) => page.locator('#lc-bearbeiten-hinweis');
const speichern = (page, zeit) => nachrichtMitZeit(page, zeit).getByRole('button', { name: 'Speichern' });

/// Schreibt jeden nicht leeren Text des Hinweises mit, sobald er sich ändert; → Abfrage der bisherigen Texte.
async function hinweisMitschreiben(page) {
  await page.evaluate(() => {
    window.hinweisTexte = [];
    const knoten = document.querySelector('#lc-bearbeiten-hinweis');
    new MutationObserver(() => {
      if (knoten.textContent !== '') {
        window.hinweisTexte.push(knoten.textContent);
      }
    }).observe(knoten, { childList: true, characterData: true, subtree: true });
  });
  return () => page.evaluate(() => window.hinweisTexte);
}

function eigenschaftenDesHinweises() {
  const knoten = document.querySelector('#lc-bearbeiten-hinweis');
  const feldKnoten = document.querySelector('textarea[aria-label="Nachricht bearbeiten"]');
  return {
    live: knoten.getAttribute('aria-live'),
    rolle: knoten.getAttribute('role'),
    beschreibtFeld: feldKnoten.getAttribute('aria-describedby').split(/\s+/).includes(knoten.id),
    leerVorDemText: knoten.textContent === '',
  };
}

/// Der Hinweis ist eine Live-Region (aria-live nicht off oder role=alert), beschreibt das Feld und war vor seinem Text da.
async function istLiveRegion(page) {
  const eigenschaften = await page.evaluate(eigenschaftenDesHinweises);
  expect(eigenschaften.live === null ? eigenschaften.rolle : eigenschaften.live).toMatch(/^(alert|polite|assertive)$/);
  expect(eigenschaften.beschreibtFeld).toBe(true);
  expect(eigenschaften.leerVorDemText).toBe(true);
}

async function mitQuote(browser) {
  instanz = await instanzStarten({
    argumente: ['--plattenquote', '10MiB'],
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      channelAnlegen(ordner, 'allgemein');
      execFileSync(QUOTE_ORDNER, [ordner, 'allgemein', String(10 * 1024 * 1024 - 300)], {
        stdio: 'ignore', env: { ...process.env, TZ: 'Europe/Berlin', LOCALCHATA_JETZT: JETZT },
      });
    },
  });
  const a = await browserOeffnen(browser, { zusaetzlich: { [BEARBEITEN]: [507] } });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  return { a, zeit };
}

test('507 (B0802): der Hinweis ist Live-Region und mit dem Feld verbunden; über 2 s genau eine Ansage, ein zweites Speichern mit demselben 507 wieder genau eine; die Ansage-Region der Seite bleibt leer; nach einer Eingabe ist er leer', async ({ browser }) => {
  const { a, zeit } = await mitQuote(browser);
  await feldPerTaste(a.page, zeit);
  await istLiveRegion(a.page);

  const angesagt = await ansagenMitschreiben(a.page);
  const hinweise = await hinweisMitschreiben(a.page);
  await feld(a.page).fill('x'.repeat(400));
  await speichern(a.page, zeit).click();
  await expect(hinweisAm(a.page)).toContainText('Nicht bearbeitet');
  await a.page.waitForTimeout(2000);
  expect(await hinweise()).toHaveLength(1);
  expect((await hinweise())[0]).toContain('Nicht bearbeitet');
  expect(await angesagt()).toEqual([]);

  await speichern(a.page, zeit).click();
  await expect.poll(async () => (await hinweise()).length).toBe(2);
  await a.page.waitForTimeout(2000);
  expect(await hinweise()).toHaveLength(2);
  expect(await angesagt()).toEqual([]);

  await feld(a.page).pressSequentially('y');
  await expect(hinweisAm(a.page)).toHaveText('');
});

test('„Nicht gespeichert: keine Antwort vom Server.“ (B0802): genau eine Ansage am Feld, die Region der Seite bleibt leer', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { serverWegErwartet: false });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldPerTaste(a.page, zeit);
  await istLiveRegion(a.page);
  await feld(a.page).fill('Hallo neu');
  const angesagt = await ansagenMitschreiben(a.page);
  const hinweise = await hinweisMitschreiben(a.page);
  await a.page.route(`**${BEARBEITEN}`, (route) => route.abort(), { times: 1 });
  await speichern(a.page, zeit).click();
  await expect(hinweisAm(a.page)).toHaveText('⚠ Nicht gespeichert: keine Antwort vom Server.');
  await a.page.waitForTimeout(2000);
  expect(await hinweise()).toEqual(['⚠ Nicht gespeichert: keine Antwort vom Server.']);
  expect(await angesagt()).toEqual([]);
});

test('„Leer geht nicht …“ (B0802): leeren des Felds sagt es genau einmal am Feld an, weiteres Tippen im selben Zustand nicht noch einmal', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldPerTaste(a.page, zeit);
  await istLiveRegion(a.page);
  const angesagt = await ansagenMitschreiben(a.page);
  const hinweise = await hinweisMitschreiben(a.page);
  await feld(a.page).fill('');
  await expect(hinweisAm(a.page)).toContainText('Leer geht nicht');
  await feld(a.page).press('Shift+Enter');
  await feld(a.page).press('Backspace');
  await a.page.waitForTimeout(2000);
  const gesammelt = await hinweise();
  expect(gesammelt.filter((text) => text.startsWith('Leer geht nicht'))).toHaveLength(1);
  expect(await angesagt()).toEqual([]);

  await feld(a.page).pressSequentially('Neu');
  await expect(hinweisAm(a.page)).toHaveText('');
  expect(a.fehler).toEqual([]);
});
