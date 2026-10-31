'use strict';
// R00050 F0189 B0807 (Karte #85, ux-reviewer-Vorprüfung): „Direktnachrichten“ und sein Inhalt erscheinen im selben Frame —
// ein Frame-Sampler (requestAnimationFrame ab Dokumentstart) sieht nie die Überschrift sichtbar bei leerem Inhalt
// (`.lc-side-eintraege` ohne Kind), ob `GET /api/direkt` sofort oder 500 ms später antwortet, auf dem Desktop (1280) und
// auf Touch (768). Mit einem Gespräch erscheint „Noch keine Direktnachrichten.“ in keinem Frame; schlägt das Laden fehl,
// steht „Direktnachrichten nicht geladen.“ (Wächter). Mutante, die rot werden muss: Abschnitt von Anfang an sichtbar.
const { test, expect } = require('./rahmen');
const { sendenStarten, beitreten, imBrowserBeitreten } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');
const { direktSenden } = require('./r00050-direkt-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GERAETE = {
  desktop: { viewport: { width: 1280, height: 800 } },
  touch768: { viewport: { width: 768, height: 1024 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 },
};

/// Läuft ab Dokumentstart in jedem Frame: merkt sich, wie viele Frames die Überschrift „Direktnachrichten“ sichtbar bei
/// leerem Inhalt zeigten, wie viele den Leerzustand und wie viele den Fehlerhinweis.
function samplerInstallieren() {
  window.__frames = { ueberschriftOhneInhalt: 0, leerzustand: 0, nichtGeladen: 0, gesamt: 0 };
  const sichtbar = (element) => element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden';
  function tick() {
    const nav = document.querySelector('nav[aria-label="Direktnachrichten"]');
    if (nav !== null) {
      const ueberschrift = nav.querySelector('h2');
      const inhalt = nav.querySelector('.lc-side-eintraege');
      const frames = window.__frames;
      frames.gesamt += 1;
      if (sichtbar(ueberschrift) && inhalt.children.length === 0) {
        frames.ueberschriftOhneInhalt += 1;
      }
      const texte = [...inhalt.querySelectorAll('.lc-side-leer')].filter(sichtbar).map((absatz) => absatz.textContent);
      if (texte.includes('Noch keine Direktnachrichten.')) {
        frames.leerzustand += 1;
      }
      if (texte.includes('Direktnachrichten nicht geladen.')) {
        frames.nichtGeladen += 1;
      }
    }
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

async function beitretenMitSampler(browser, geraet, { verzoegerungMs = 0, abbrechen = false } = {}) {
  const a = await browserOeffnen(browser, { geraet: GERAETE[geraet], zusaetzlich: abbrechen ? { '/api/direkt': [500] } : {} });
  await a.kontext.addInitScript(samplerInstallieren);
  await a.page.route('**/api/direkt', async (route) => {
    if (abbrechen) {
      await route.fulfill({ status: 500, body: '{}', contentType: 'application/json' });
      return;
    }
    await new Promise((weiter) => setTimeout(weiter, verzoegerungMs));
    await route.continue();
  });
  await imBrowserBeitreten(a.page, instanz.adresse, 'anna');
  return a;
}

const frames = (page) => page.evaluate(() => window.__frames);
const direktNav = (page) => page.getByRole('navigation', { name: 'Direktnachrichten' });

for (const geraet of Object.keys(GERAETE)) {
  for (const verzoegerungMs of [0, 500]) {
    test(`${geraet}, GET /api/direkt nach ${verzoegerungMs} ms (B0807): nie die Überschrift ohne Inhalt, am Ende Überschrift und Leerzustand`, async ({ browser }) => {
      instanz = await sendenStarten();
      const a = await beitretenMitSampler(browser, geraet, { verzoegerungMs });
      await expect(direktNav(a.page).locator('.lc-side-leer')).toHaveText(['Noch keine Direktnachrichten.', 'Ein Gespräch beginnt in der Teilnehmerliste.']);
      await expect(direktNav(a.page).getByRole('heading', { level: 2, name: 'Direktnachrichten' })).toBeAttached();
      await a.page.waitForTimeout(300);
      const gemessen = await frames(a.page);
      expect(gemessen.gesamt).toBeGreaterThan(5);
      expect(gemessen.ueberschriftOhneInhalt).toBe(0);
      expect(gemessen.nichtGeladen).toBe(0);
    });
  }

  test(`${geraet} mit einem Gespräch (B0807): „Noch keine Direktnachrichten.“ erscheint in keinem Frame`, async ({ browser }) => {
    instanz = await sendenStarten();
    const a = await beitretenMitSampler(browser, geraet, { verzoegerungMs: 500 });
    const ben = await beitreten(instanz.adresse, 'ben', { art: 'mensch', herkunft: '127.0.0.3' });
    expect((await direktSenden(instanz.adresse, ben, 'anna@127.0.0.1', 'Hallo anna', '127.0.0.3')).status).toBe(201);
    await a.page.reload();
    await expect(direktNav(a.page).getByRole('link')).toHaveCount(1);
    await a.page.waitForTimeout(300);
    const gemessen = await frames(a.page);
    expect(gemessen.leerzustand).toBe(0);
    expect(gemessen.ueberschriftOhneInhalt).toBe(0);
  });
}

test('Laden schlägt fehl (Wächter B0807): „Direktnachrichten nicht geladen.“ steht unter der Überschrift, nie die Überschrift allein', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await beitretenMitSampler(browser, 'desktop', { abbrechen: true });
  await expect(direktNav(a.page).locator('.lc-side-leer')).toHaveText(['Direktnachrichten nicht geladen.']);
  const gemessen = await frames(a.page);
  expect(gemessen.ueberschriftOhneInhalt).toBe(0);
  expect(gemessen.leerzustand).toBe(0);
});
