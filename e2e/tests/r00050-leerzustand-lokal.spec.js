'use strict';
// R00050 F0189 B0808 (Karte #85, ux-reviewer-Vorprüfung): eine lokale Nachricht — auch eine fehlgeschlagene ⚠ — beendet den
// Leerzustand des Gesprächs („Noch keine Nachrichten mit …“ samt „Agenten schreiben hier mit POST …“), nach „Verwerfen“
// kehrt er zurück, nach „Rückgängig“ ist er wieder weg; im Direktgespräch (404 beim Senden an jemanden, den es nicht gibt;
// 1,5 s verzögerter POST an beta@127.0.0.2) wie im leeren Channel. Mutante, die rot werden muss: Leerzustand bleibt stehen,
// solange nur Vorläufiges da ist. Gegen das echte Programm; Desktop und Touch 375 × 667.
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, beitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { direktOeffnen, gespraechOffen, verlaufMit, eingabeAn, tippenOhneGespraech } = require('./r00050-direkt-hilfen');
const { ROUTE } = require('./r00012-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const ZED = 'zed@127.0.0.7';
const BETA = 'beta@127.0.0.2';
const GERAETE = {
  desktop: { viewport: { width: 1280, height: 800 } },
  telefon: { viewport: { width: 375, height: 667 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 },
};
const LEER_DM = (kennung) => `Noch keine Nachrichten mit ${kennung}.`;
const AGENTEN = 'Agenten schreiben hier mit';

const direktPfad = (kennung) => `/api/direkt/${encodeURIComponent(kennung)}/nachrichten`;

/// Sendet im offenen Gespräch über die Eingabe (Enter am Desktop, Knopf „Senden“ auf Touch).
async function senden(page, eingabeFeld, text, touch) {
  await eingabeFeld.fill(text);
  if (touch) {
    await page.locator('.lc-comp').getByRole('button', { name: 'Senden', exact: true }).tap();
    return;
  }
  await eingabeFeld.press('Enter');
}

const vorlaeufigeDa = (log) => log.locator('.lc-msg.lc-vorlaeufig');

/// Sofort, ohne Warten auf einen späteren Zustand: der Verlauf zeigt jetzt keinen Leerzustand.
async function keinLeerzustand(log, leerText) {
  const text = await log.evaluate((element) => element.textContent);
  expect(text).not.toContain(leerText);
  expect(text).not.toContain(AGENTEN);
  expect(await log.locator('.lc-leer-api').count()).toBe(0);
}

for (const [geraet, einstellung] of Object.entries(GERAETE)) {
  const touch = geraet === 'telefon';

  test(`${geraet}, Direktgespräch nach 404 beim Senden (B0808): die ⚠-Nachricht steht, der Leerzustand ist weg; Verwerfen bringt ihn zurück, Rückgängig nimmt ihn wieder weg`, async ({ browser }) => {
    instanz = await sendenStarten();
    const anna = await browserOeffnen(browser, {
      geraet: einstellung, zusaetzlich: { ...tippenOhneGespraech(ZED), [direktPfad(ZED)]: [404] },
    });
    await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
    await direktOeffnen(anna.page, ZED);
    await gespraechOffen(anna.page, ZED);
    const log = verlaufMit(anna.page, ZED);
    await expect(log).toContainText(LEER_DM(ZED));
    await expect(log).toContainText(AGENTEN);

    await senden(anna.page, eingabeAn(anna.page, ZED), 'Hallo zed', touch);
    await expect(vorlaeufigeDa(log).locator('.lc-send')).toContainText('⚠');
    await keinLeerzustand(log, LEER_DM(ZED));
    await expect(log).toContainText('Hallo zed');

    await vorlaeufigeDa(log).getByRole('button', { name: 'Verwerfen' }).click();
    await expect(log.locator('.lc-verworfen')).toBeVisible();
    await expect(log).toContainText(LEER_DM(ZED));
    await expect(log).toContainText(AGENTEN);

    await log.locator('.lc-verworfen').getByRole('button', { name: 'Rückgängig' }).click();
    await expect(vorlaeufigeDa(log).locator('.lc-send')).toContainText('⚠');
    await keinLeerzustand(log, LEER_DM(ZED));
    await expect(log).toContainText('Hallo zed');
  });

  test(`${geraet}, Direktgespräch mit 1,5 s verzögertem POST (B0808): 500 ms nach dem Senden steht die Nachricht „wird gesendet“ ohne Leerzustand, danach die bestätigte`, async ({ browser }) => {
    instanz = await sendenStarten();
    const anna = await browserOeffnen(browser, { geraet: einstellung, zusaetzlich: tippenOhneGespraech(BETA) });
    await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
    await beitreten(instanz.adresse, 'beta', { art: 'mensch', herkunft: '127.0.0.2' });
    await anna.page.route('**/api/direkt/*/nachrichten', async (route) => {
      if (route.request().method() === 'POST') {
        await new Promise((weiter) => setTimeout(weiter, 1500));
      }
      await route.continue();
    });
    await direktOeffnen(anna.page, BETA);
    await gespraechOffen(anna.page, BETA);
    const log = verlaufMit(anna.page, BETA);
    await expect(log).toContainText(LEER_DM(BETA));

    await senden(anna.page, eingabeAn(anna.page, BETA), 'Hallo beta', touch);
    await anna.page.waitForTimeout(500);
    await expect(vorlaeufigeDa(log)).toContainText('wird gesendet');
    await keinLeerzustand(log, LEER_DM(BETA));
    await expect(log.locator('.lc-msg:not(.lc-vorlaeufig)').filter({ hasText: 'Hallo beta' })).toHaveCount(1, { timeout: 5000 });
    await keinLeerzustand(log, LEER_DM(BETA));
  });

  test(`${geraet}, leerer Channel (B0808): fehlgeschlagene Nachricht beendet „Noch keine Nachrichten in # allgemein.“, Verwerfen bringt ihn zurück, Rückgängig nimmt ihn weg`, async ({ browser }) => {
    instanz = await sendenStarten();
    const anna = await browserOeffnen(browser, { geraet: einstellung, zusaetzlich: {} });
    await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
    let erste = true;
    await anna.page.route(ROUTE, async (route) => {
      if (route.request().method() === 'POST' && erste) {
        erste = false;
        await route.abort();
        return;
      }
      await route.continue();
    });
    const log = anna.page.getByRole('log');
    const leer = 'Noch keine Nachrichten in # allgemein.';
    await expect(log).toContainText(leer);

    await senden(anna.page, eingabe(anna.page), 'drei', touch);
    await expect(vorlaeufigeDa(log).locator('.lc-send')).toContainText('⚠');
    await keinLeerzustand(log, leer);

    await vorlaeufigeDa(log).getByRole('button', { name: 'Verwerfen' }).click();
    await expect(log.locator('.lc-verworfen')).toBeVisible();
    await expect(log).toContainText(leer);
    await expect(log).toContainText(AGENTEN);

    await log.locator('.lc-verworfen').getByRole('button', { name: 'Rückgängig' }).click();
    await expect(vorlaeufigeDa(log).locator('.lc-send')).toContainText('⚠');
    await keinLeerzustand(log, leer);
  });
}
