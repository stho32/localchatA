'use strict';
// R00050 F0178 B0788 (Karte #13, ADR-0055 D12 Teil Gespräch, Matrix K2, S1): drei Browserkontexte `anna`, `ben`, `cora`
// gegen das echte Programm. Teilnehmerliste → „Direktnachricht“ öffnet `#/d/<kennung>` im Privat-Modus (lc-privat-bg,
// im Kopf des Gesprächs die volle Kennung und der Hinweis); `ben` sieht die Nachricht live; `cora` sieht keine
// Nachricht, auch nach Neuladen, und unter `#/d/<fremdes Paar>` nur ein leeres Gespräch; Tippen im Direktgespräch nur
// beim Gegenüber — nie im zweiten Tab von `anna`, nie bei `cora`. Mutanten, die rot werden müssen: Hash eines fremden
// Paars öffnet Inhalt; Tippzeile zeigt das eigene Tippen.
const { test, expect } = require('./rahmen');
const { sichtbarNach, fehlerSammeln } = require('./hilfen');
const { sendenStarten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { zeigtTipp } = require('./r00045-hilfen');
const {
  HINWEIS, tippenOhneGespraech, paarordner, direktOeffnen, verlaufMit, eingabeAn, gespraechOffen,
} = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ANNA_B = 'anna@127.0.0.1';
const BEN_B = 'ben@127.0.0.1';
const CORA_B = 'cora@127.0.0.1';
const ZUSTELLUNG_MS = 1000;
const GEHEIM = 'Nur für ben';
const warten = (ms) => new Promise((weiter) => setTimeout(weiter, ms));

/// Öffnet über Teilnehmerliste und Menü das Gespräch mit `kennung`.
async function ueberDasMenue(page, kennung) {
  const zeile = page.getByRole('complementary', { name: 'Teilnehmer' }).locator('.lc-person')
    .filter({ has: page.locator('.lc-id', { hasText: kennung }) });
  await zeile.getByRole('button').click();
  await page.getByRole('menuitem', { name: 'Direktnachricht' }).click();
  await gespraechOffen(page, kennung);
}

/// Merkt im Tab jeden nicht leeren Text der Tippzeile — auch einen, der nur kurz steht.
function tippzeileMitschreiben(page) {
  return page.evaluate(() => {
    window.tippzeileGesehen = [];
    const zeile = document.querySelector('.lc-comp .lc-comp-tipp');
    new MutationObserver(() => {
      if (zeile.textContent !== '') {
        window.tippzeileGesehen.push(zeile.textContent);
      }
    }).observe(zeile, { childList: true, characterData: true, subtree: true });
  });
}

function zeigtText({ text }) {
  return [...document.querySelectorAll('.lc-log .lc-msg:not(.lc-vorlaeufig) .lc-body')].some((body) => body.textContent === text);
}

test('Drei Kontexte (B0788, D12, K2, S1): anna beginnt über das Menü, ben sieht live, cora sieht nichts — auch nach Neuladen und unter fremder Adresse', async ({ browser }) => {
  test.setTimeout(45000);
  instanz = await sendenStarten();
  const anna = await browserOeffnen(browser, { zusaetzlich: tippenOhneGespraech(BEN_B) });
  const ben = await browserOeffnen(browser, { zusaetzlich: tippenOhneGespraech(ANNA_B) });
  const cora = await browserOeffnen(browser, { zusaetzlich: tippenOhneGespraech(ANNA_B, BEN_B) });
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await imBrowserBeitreten(cora.page, instanz.adresse, 'cora');

  await ueberDasMenue(anna.page, BEN_B);
  await expect(anna.page).toHaveURL(new RegExp(`#/d/${encodeURIComponent(BEN_B)}$`));
  await expect(anna.page.locator('main.lc-center')).toHaveCSS('background-color', 'rgb(233, 237, 242)');
  await expect(anna.page.getByRole('banner').locator('.lc-head-gegenueber')).toHaveText(BEN_B);
  await expect(anna.page.locator('p.lc-privat-hinweis')).toHaveText(HINWEIS);
  await ueberDasMenue(ben.page, ANNA_B);
  await direktOeffnen(cora.page, ANNA_B);
  await gespraechOffen(cora.page, ANNA_B);

  const annaZweiterTab = await anna.kontext.newPage();
  const annaZwei = { page: annaZweiterTab, fehler: fehlerSammeln(annaZweiterTab, { zusaetzlich: tippenOhneGespraech(BEN_B) }) };
  await annaZwei.page.goto(`${instanz.adresse}#/d/${encodeURIComponent(BEN_B)}`);
  await gespraechOffen(annaZwei.page, BEN_B);

  const gesendet = anna.page.waitForResponse((antwort) => antwort.request().method() === 'POST' && antwort.url().endsWith('/nachrichten'));
  await eingabeAn(anna.page, BEN_B).fill(GEHEIM);
  await eingabeAn(anna.page, BEN_B).press('Enter');
  expect((await gesendet).status()).toBe(201);
  const ab = Date.now();
  expect(await sichtbarNach(ben.page, zeigtText, { argument: { text: GEHEIM }, ab }), 'ben live ab der 201').toBeLessThanOrEqual(ZUSTELLUNG_MS);
  await sichtbarNach(annaZwei.page, zeigtText, { argument: { text: GEHEIM }, ab });

  for (const seite of [anna.page, annaZwei.page, cora.page]) {
    await tippzeileMitschreiben(seite);
  }
  const tippenAb = Date.now();
  await eingabeAn(anna.page, BEN_B).pressSequentially('Noch');
  expect(await sichtbarNach(ben.page, zeigtTipp, { argument: 'anna tippt …', ab: tippenAb }), 'Tippen bei ben').toBeLessThanOrEqual(ZUSTELLUNG_MS);
  await warten(1500);
  for (const seite of [anna.page, annaZwei.page, cora.page]) {
    expect(await seite.evaluate(() => window.tippzeileGesehen), 'Tippzeile ohne das eigene Tippen').toEqual([]);
  }

  await expect(verlaufMit(cora.page, ANNA_B).locator('.lc-msg')).toHaveCount(0);
  await expect(cora.page.getByText(GEHEIM)).toHaveCount(0);
  await cora.page.reload();
  await gespraechOffen(cora.page, ANNA_B);
  await expect(cora.page.locator('.lc-mitte-hinweis').first()).toHaveText(`Noch keine Nachrichten mit ${ANNA_B}.`);
  await direktOeffnen(cora.page, BEN_B);
  await gespraechOffen(cora.page, BEN_B);
  await expect(verlaufMit(cora.page, BEN_B).locator('.lc-msg')).toHaveCount(0);
  await cora.page.evaluate(() => { window.location.hash = '#/c/allgemein'; });
  await expect(cora.page.getByRole('log', { name: 'Verlauf # allgemein' })).toBeVisible();
  await expect(cora.page.getByText(GEHEIM)).toHaveCount(0);

  expect(paarordner(instanz.ordner)).toHaveLength(1);
  expect([...anna.fehler, ...annaZwei.fehler, ...ben.fehler, ...cora.fehler]).toEqual([]);
});
