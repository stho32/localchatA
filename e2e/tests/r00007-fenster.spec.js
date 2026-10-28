'use strict';
// R00007 F0031 Fenster beim Scrollen (US-16 bis US-18, L3–L5): gegen das echte Programm, Chromium 1280 × 800, Uhr
// 2026-10-28 15:00 (+01:00). `gross` hat 100.000 Nachrichten über 7 Tage; `grenzen` 150 am 27.10. und 100 am
// 28.10. im Minutentakt — die erste Blockgrenze liegt am Tageswechsel, die zweite mitten in einer Gruppe. L3–L5
// sind deterministisch; die Grenze L4 ist die der Anforderung (höchstens 1 px).
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln, channelAnlegen, grossAnlegen } = require('./hilfen');
const { juengsteZeit, bloeckeRueckwaerts, ankerBeobachten, ruhig } = require('./verlauf-hilfen');

const JETZT = '2026-10-28T15:00:00.000+01:00';
const GRENZE_L4_PX = 1;
const ALPHA = 'alpha@127.0.0.2';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

function minutentakt(datum, anzahl, versatz) {
  return Array.from({ length: anzahl }, (_, index) => {
    const stunde = String(8 + Math.floor(index / 60)).padStart(2, '0');
    const minute = String(index % 60).padStart(2, '0');
    return [`${datum}T${stunde}:${minute}:00.000${versatz}`, ALPHA, `Nachricht ${index + 1} am ${datum}`];
  });
}

const verlauf = (page) => page.getByRole('log');
const nachrichten = (page) => verlauf(page).locator('.lc-msg');
const pille = (page) => page.getByRole('button', { name: /Zum Neuesten/ });

async function geoeffnet(page, channel) {
  instanz = await instanzStarten({
    umgebung: { LOCALCHATA_JETZT: JETZT },
    vorbereiten: (ordner) => {
      grossAnlegen(ordner);
      channelAnlegen(ordner, 'grenzen', {
        '2026-10-27': minutentakt('2026-10-27', 150, '+01:00'),
        '2026-10-28': minutentakt('2026-10-28', 100, '+01:00'),
      });
    },
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(`${instanz.adresse}#/c/${channel}`);
  await page.getByLabel('Name').fill('shoff');
  await page.getByLabel('Name').press('Enter');
  await expect(nachrichten(page)).toHaveCount(100);
  await ruhig(page);
}

function abrufeZaehlen(page) {
  const abrufe = { vor: 0, nach: 0 };
  page.on('requestfinished', (anfrage) => {
    const suche = new URL(anfrage.url()).search;
    if (suche.startsWith('?vor=')) {
      abrufe.vor += 1;
    }
    if (suche.startsWith('?nach=')) {
      abrufe.nach += 1;
    }
  });
  return abrufe;
}

// Scrollt in Schritten von 1.000 px nach oben (bzw. unten), bis `bis()` wahr ist; nach jedem Schritt Ruhe.
async function scrollen(page, schritt, bis) {
  for (let runde = 0; runde < 2000 && !bis(); runde += 1) {
    await verlauf(page).evaluate((log, weite) => log.scrollBy(0, weite), schritt);
    await ruhig(page);
  }
  expect(bis()).toBe(true);
}

async function zeitenImDom(page) {
  return nachrichten(page).evaluateAll((elemente) => elemente.map((element) => element.dataset.zeit));
}

test('L3 (US-16): nach zehnmal Nachladen nach oben stehen genau 500 Nachrichten im DOM — Nummer 98.901 bis 99.400', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');
  const abrufe = abrufeZaehlen(page);
  const erwartet = await bloeckeRueckwaerts(instanz.adresse, 'gross', 11);

  // Je Nachladen ein Sprung an den oberen Rand (wie im zweiten L4-Test) statt 1.000-px-Schritten: zehn Schritte statt
  // rund 150, damit L3 auch unter Last im Budget von 30 s bleibt; ein Sprung zu viel hieße ein elfter Abruf.
  for (let sprung = 0; sprung < 10; sprung += 1) {
    await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
    await ruhig(page);
  }

  expect(abrufe.vor).toBe(10);
  await expect(nachrichten(page)).toHaveCount(500);
  expect(await zeitenImDom(page)).toEqual(erwartet.slice(6, 11).reverse().flat());
  await expect(pille(page)).toBeVisible();
  expect(fehler).toEqual([]);
});

test('L4 (US-18): an der Tages- und der Gruppengrenze und beim Nachladen unten springt nichts', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'grenzen');
  await ankerBeobachten(page);
  const abrufe = abrufeZaehlen(page);

  await scrollen(page, -200, () => abrufe.vor >= 2);
  await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
  await ruhig(page);

  const spruenge = await page.evaluate(() => window.lcSpruenge);
  expect(spruenge.length).toBeGreaterThanOrEqual(4);
  for (const sprung of spruenge) {
    expect(Math.abs(sprung)).toBeLessThanOrEqual(GRENZE_L4_PX);
  }
  await expect(verlauf(page).locator('.lc-day')).toHaveText(['Gestern', 'Heute']);
  const nachricht51 = verlauf(page).locator('.lc-msg[data-zeit="2026-10-27T08:50:00.000+01:00"]');
  await expect(nachricht51.locator('.lc-meta')).toHaveCount(0);
  await expect(nachrichten(page)).toHaveCount(250);
  expect(fehler).toEqual([]);
});

test('L4: zehn Sprünge an den oberen Rand und Nachladen unten in gross halten die oberste Nachricht', async ({ page }) => {
  test.setTimeout(120000);
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');
  await ankerBeobachten(page);
  const abrufe = abrufeZaehlen(page);

  for (let sprung = 0; sprung < 10; sprung += 1) {
    await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
    await ruhig(page);
  }
  expect(abrufe.vor).toBe(10);
  await scrollen(page, 1000, () => abrufe.nach >= 3);

  const spruenge = await page.evaluate(() => window.lcSpruenge);
  expect(spruenge.length).toBeGreaterThanOrEqual(13);
  for (const sprung of spruenge) {
    expect(Math.abs(sprung)).toBeLessThanOrEqual(GRENZE_L4_PX);
  }
  expect(fehler).toEqual([]);
});

test('L5: hoch über 1.000 Nachrichten und zurück bis zum Neuesten — jede Zeit genau einmal, streng steigend', async ({ page }) => {
  test.setTimeout(120000);
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');
  await ankerBeobachten(page);
  const abrufe = abrufeZaehlen(page);
  const erwartet = await bloeckeRueckwaerts(instanz.adresse, 'gross', 11);
  const juengste = await juengsteZeit(instanz.adresse, 'gross');

  await scrollen(page, -3000, () => abrufe.vor >= 10);
  await scrollen(page, 3000, () => abrufe.nach >= 6);
  await verlauf(page).evaluate((log) => { log.scrollTop = log.scrollHeight; });
  await ruhig(page);
  expect(abrufe.nach).toBe(6);

  const neuer = await verlauf(page).evaluate((log) => log.scrollHeight - log.scrollTop - log.clientHeight);
  expect(neuer).toBeLessThanOrEqual(1);
  expect(await page.evaluate(() => window.lcOrdnungVerletzt)).toBe(false);
  const gesehen = await page.evaluate(() => [...window.lcGesehen].sort((links, rechts) => Date.parse(links) - Date.parse(rechts)));
  expect(gesehen).toEqual(erwartet.slice().reverse().flat());
  const imDom = await zeitenImDom(page);
  expect(imDom[imDom.length - 1]).toBe(juengste);
  expect(new Set(imDom).size).toBe(imDom.length);
  expect(fehler).toEqual([]);
});

test('Ladezeile (US-16): solange der Abruf nach oben läuft, steht oben „Ältere Nachrichten werden geladen …“, kein zweiter Abruf', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');
  const abrufe = [];
  await page.route('**/nachrichten?vor=**', async (route) => {
    abrufe.push(route.request().url());
    await new Promise((weiter) => setTimeout(weiter, 1000));
    await route.continue();
  });
  const ladezeile = page.locator('.lc-loadmore');

  await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
  await expect(ladezeile).toBeVisible();
  await expect(ladezeile).toHaveText('Ältere Nachrichten werden geladen …');
  await verlauf(page).evaluate((log) => { log.scrollTop = 0; log.dispatchEvent(new Event('scroll')); });
  await page.waitForTimeout(300);
  expect(abrufe.length).toBe(1);

  await expect(ladezeile).toBeHidden({ timeout: 5000 });
  await expect(nachrichten(page)).toHaveCount(200);
  expect(fehler).toEqual([]);
});

test('Zum Neuesten (US-17): Pille erst über 1 Bildschirmhöhe, Klick und Ende öffnen neu, der Fokus liegt im Verlauf', async ({ page }) => {
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');
  const juengste = await juengsteZeit(instanz.adresse, 'gross');
  const ueberDemEnde = (abstand) => verlauf(page).evaluate((log, mehr) => {
    log.scrollTop = log.scrollHeight - log.clientHeight - log.clientHeight - mehr;
  }, abstand);

  await ueberDemEnde(-10);
  await ruhig(page);
  await expect(pille(page)).toBeHidden();
  await ueberDemEnde(10);
  await ruhig(page);
  await expect(pille(page)).toBeVisible();
  await expect(pille(page)).toContainText('Zum Neuesten');

  await pille(page).click();
  await expect(nachrichten(page).last()).toHaveAttribute('data-zeit', juengste);
  await ruhig(page);
  await expect(verlauf(page)).toBeFocused();
  const ganz = () => nachrichten(page).last().evaluate((element) => {
    const log = element.closest('[role="log"]').getBoundingClientRect();
    const lage = element.getBoundingClientRect();
    return lage.top >= log.top - 1 && lage.bottom <= log.bottom + 1;
  });
  expect(await ganz()).toBe(true);
  await expect(pille(page)).toBeHidden();

  await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
  await ruhig(page);
  await expect(pille(page)).toBeVisible();
  await verlauf(page).focus();
  await page.keyboard.press('End');
  await expect(nachrichten(page)).toHaveCount(100);
  await ruhig(page);
  expect(await ganz()).toBe(true);
  await expect(nachrichten(page).last()).toHaveAttribute('data-zeit', juengste);
  await expect(verlauf(page)).toBeFocused();
  expect(fehler).toEqual([]);
});

test('Kleben: über einen Tageswechsel hochgescrollt steht der Trenner des obersten sichtbaren Tages oben', async ({ page }) => {
  test.setTimeout(180000);
  const fehler = fehlerSammeln(page);
  await geoeffnet(page, 'gross');

  for (let sprung = 0; sprung < 400; sprung += 1) {
    const datum = await verlauf(page).evaluate((log) => {
      const rand = log.getBoundingClientRect().top;
      const oberste = [...log.querySelectorAll('.lc-msg')].find((element) => element.getBoundingClientRect().bottom > rand);
      return oberste.dataset.zeit.slice(0, 10);
    });
    if (datum !== '2026-10-28') {
      break;
    }
    await verlauf(page).evaluate((log) => { log.scrollTop = 0; });
    await ruhig(page);
  }

  const lage = await verlauf(page).evaluate((log) => {
    const rand = log.getBoundingClientRect().top;
    const oberste = [...log.querySelectorAll('.lc-msg')].find((element) => element.getBoundingClientRect().bottom > rand + 30);
    const datum = oberste.dataset.zeit.slice(0, 10);
    const trenner = log.querySelector(`.lc-day[data-datum="${datum}"]`);
    return { datum, trennerOben: trenner.getBoundingClientRect().top - rand, text: trenner.textContent };
  });
  expect(lage.datum).toBe('2026-10-27');
  expect(lage.text).toBe('Gestern');
  expect(Math.abs(lage.trennerOben)).toBeLessThanOrEqual(1);
  expect(fehler).toEqual([]);
});
