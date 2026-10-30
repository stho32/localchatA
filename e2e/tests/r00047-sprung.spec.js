'use strict';
// R00047 I0044 F0159: der Sprung vom Zitat zum Original (ADR-0053 Regel 8, P10) — im Fenster (B0717): zentriert,
// 2000 ms hervorgehoben, Fokus.
const { test, expect } = require('./rahmen');
const { channelAnlegen, sichtbarNach } = require('./hilfen');
const { sendenStarten, beitreten, imBrowserBeitreten } = require('./senden-hilfen');
const { geraetOeffnen } = require('./r00027-hilfen');
const { nachrichtMitZeit, loeschenPerApi, ansagenMitschreiben } = require('./r00041-hilfen');
const {
  ANNA, CLAUDE, claudeSchreibt, mitBezugSenden, benOeffnen, zitatVon,
} = require('./r00047-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const HERVORHEBUNG_MS = 2000;

/// Erste und letzte Zeit des Fensters.
const fensterRand = (page) => page.getByRole('log').locator('.lc-msg[data-zeit]').evaluateAll((alle) => [alle[0].dataset.zeit, alle[alle.length - 1].dataset.zeit]);

/// Steht die Nachricht ganz im Verlauf, hervorgehoben (Grund kd-surface-2, 4-px-Balken kd-accent links, Fokusring) und
/// fokussiert? Läuft im Browser.
function hervorgehobenImBild({ zeit }) {
  const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${zeit}"]`);
  if (knoten === null) {
    return false;
  }
  const log = knoten.closest('[role="log"]').getBoundingClientRect();
  const rand = knoten.getBoundingClientRect();
  const farbe = (wert) => {
    const probe = document.createElement('div');
    probe.style.color = wert;
    document.body.append(probe);
    const berechnet = getComputedStyle(probe).color;
    probe.remove();
    return berechnet;
  };
  const stil = getComputedStyle(knoten);
  const ganz = rand.top >= log.top - 0.5 && rand.bottom <= log.bottom + 0.5;
  return ganz && document.activeElement === knoten && stil.backgroundColor === farbe('var(--kd-surface-2)')
    && stil.boxShadow.includes(farbe('var(--kd-accent)')) && stil.boxShadow.includes('4px') && stil.outlineStyle === 'solid';
}

/// Schreibt mit, wann die Nachricht `zeit` die Hervorhebung bekommt und verliert (performance.now()).
function hervorhebungMitschreiben(page, zeit) {
  return page.evaluate((gesucht) => {
    window.hervorhebung = [];
    new MutationObserver(() => {
      const knoten = document.querySelector(`.lc-log .lc-msg[data-zeit="${gesucht}"]`);
      const an = knoten !== null && knoten.classList.contains('lc-hervorgehoben');
      const letzter = window.hervorhebung[window.hervorhebung.length - 1];
      if (letzter === undefined || letzter[0] !== an) {
        window.hervorhebung.push([an, performance.now()]);
      }
    }).observe(document.querySelector('.lc-log'), { attributes: true, attributeFilter: ['class'], subtree: true, childList: true });
  }, zeit);
}

/// `anna` hat 60 Nachrichten im Log (eine je Sekunde ab 14:00), `claude` antwortet auf die 25.: das Original liegt im
/// Fenster, aber außerhalb des Bildes, mit genug Inhalt darüber, um zentriert zu stehen.
async function antwortImFenster() {
  const zeiten = Array.from({ length: 60 }, (_, nummer) => `2026-10-28T14:${String(Math.floor(nummer / 60)).padStart(2, '0')}:${String(nummer % 60).padStart(2, '0')}.000+01:00`);
  instanz = await sendenStarten({
    vorbereiten: (ordner) => channelAnlegen(ordner, 'allgemein', {
      '2026-10-28': zeiten.map((zeit, nummer) => [zeit, 'anna@127.0.0.2', `Nachricht ${nummer + 1}`]),
    }),
  });
  const anna = { token: await beitreten(instanz.adresse, 'anna', { herkunft: ANNA, art: 'mensch' }), zeiten };
  const claude = await claudeSchreibt(instanz.adresse, []);
  const antwort = await mitBezugSenden(instanz.adresse, claude.token, 'Darauf', zeiten[24], CLAUDE);
  return { anna, claude, original: zeiten[24], antwort };
}

for (const weg of ['Klick', 'Enter']) {
  test(`Sprung im Fenster per ${weg} (P10, B0717): das Original steht im Bild, fokussiert, 2000 ms hervorgehoben (Grund, Balken, Fokusring), danach normal; keine Ansage`, async ({ browser }) => {
    const { original, antwort } = await antwortImFenster();
    const b = await benOeffnen(browser, instanz.adresse);
    await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
    await expect(nachrichtMitZeit(b.page, original)).not.toBeInViewport();
    const angesagt = await ansagenMitschreiben(b.page);
    await hervorhebungMitschreiben(b.page, original);
    const randVorher = await fensterRand(b.page);

    if (weg === 'Klick') {
      await zitatVon(b.page, antwort).click();
    } else {
      await zitatVon(b.page, antwort).focus();
      await b.page.keyboard.press('Enter');
    }

    await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
    const mitte = await nachrichtMitZeit(b.page, original).evaluate((knoten) => {
      const log = knoten.closest('[role="log"]').getBoundingClientRect();
      const rand = knoten.getBoundingClientRect();
      return Math.abs((rand.top + rand.bottom) / 2 - (log.top + log.bottom) / 2);
    });
    expect(mitte, 'zentriert').toBeLessThanOrEqual(30);
    await b.page.waitForFunction(() => window.hervorhebung.length >= 2, null, { timeout: 5000 });
    const wechsel = await b.page.evaluate(() => window.hervorhebung);
    expect(wechsel[0][0]).toBe(true);
    expect(wechsel[1][0]).toBe(false);
    expect(wechsel[1][1] - wechsel[0][1]).toBeGreaterThanOrEqual(HERVORHEBUNG_MS - 300);
    expect(wechsel[1][1] - wechsel[0][1]).toBeLessThanOrEqual(HERVORHEBUNG_MS + 300);
    const stil = await nachrichtMitZeit(b.page, original).evaluate((knoten) => getComputedStyle(knoten).boxShadow);
    expect(stil).toBe('none');
    expect(await angesagt()).toEqual([]);
    expect(await fensterRand(b.page)).toEqual(randVorher);
    expect(b.fehler).toEqual([]);
  });
}

test('Sprung per Antippen auf dem Telefon (P10, B0717): das Original steht im Bild und ist hervorgehoben', async ({ browser }) => {
  const { original, antwort } = await antwortImFenster();
  const b = await geraetOeffnen(browser, 'telefon', { fenster: { width: 375, height: 667 }, streng: true });
  await imBrowserBeitreten(b.page, instanz.adresse, 'ben');
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await zitatVon(b.page, antwort).tap();
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
  expect(b.fehler).toEqual([]);
});

test('Gelöschtes Original (Z4, B0717): das Zitat bleibt ein Knopf und springt zur Marke „Nachricht gelöscht“', async ({ browser }) => {
  const { anna, original, antwort } = await antwortImFenster();
  expect((await loeschenPerApi(instanz.adresse, anna.token, original, ANNA)).status).toBe(200);
  const b = await benOeffnen(browser, instanz.adresse);
  await expect(nachrichtMitZeit(b.page, antwort)).toHaveCount(1);
  await zitatVon(b.page, antwort).click();
  await sichtbarNach(b.page, hervorgehobenImBild, { argument: { zeit: original }, ab: Date.now(), timeout: 1000 });
  await expect(nachrichtMitZeit(b.page, original).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  expect(b.fehler).toEqual([]);
});

