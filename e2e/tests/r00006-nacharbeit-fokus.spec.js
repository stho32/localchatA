'use strict';
// R00006 Nacharbeit 1 (UX-Befund, NFR „Fokus nie im Nichts“): nach einer Fehlerantwort steht der Fokus im Feld
// „Name“, und unter 720 px mit eingeblendeter Teilnehmerliste schließt das erste Esc den Dialog, nicht die Liste.
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln, beitretenPerApi, channelAnlegenPerApi } = require('./hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const plus = (page) => page.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: 'Channel anlegen' });
const dialog = (page) => page.getByRole('dialog', { name: 'Channel anlegen' });
const namensfeld = (page) => dialog(page).getByLabel('Name');
const anlegenSchalter = (page) => dialog(page).getByRole('button', { name: 'Channel anlegen' });

async function beitreten(page, name) {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toContainText(`${name}@`);
}

async function mitAllgemein() {
  instanz = await instanzStarten();
  const beitritt = await beitretenPerApi(instanz.adresse, 'alpha', '127.0.0.2');
  expect(beitritt.status).toBe(201);
  const angelegt = await channelAnlegenPerApi(instanz.adresse, beitritt.daten.token, 'allgemein', '127.0.0.2');
  expect(angelegt.status).toBe(201);
}

async function aktivesElement(page) {
  return page.evaluate(() => {
    const aktiv = document.activeElement;
    return {
      tag: aktiv.tagName,
      id: aktiv.id,
      beschriftung: (aktiv.getAttribute('aria-label') || aktiv.textContent).trim(),
      sichtbar: aktiv !== document.body && aktiv.checkVisibility(),
    };
  });
}

async function perKlick(page) {
  await anlegenSchalter(page).click();
}

async function perTabUndLeertaste(page) {
  await namensfeld(page).focus();
  for (let schritt = 0; schritt < 5; schritt += 1) {
    await page.keyboard.press('Tab');
    if (await anlegenSchalter(page).evaluate((schalter) => schalter === document.activeElement)) {
      break;
    }
  }
  await expect(anlegenSchalter(page)).toBeFocused();
  await page.keyboard.press(' ');
}

const absendewege = [['Klick', perKlick], ['Tab + Leertaste', perTabUndLeertaste]];
const fehlerfaelle = [['allgemein', 409, '⚠ # allgemein gibt es schon.'], ['Allgemein', 422, /^⚠ Ein Channelname hat/]];

for (const [weg, absenden] of absendewege) {
  for (const [eingabe, status, meldung] of fehlerfaelle) {
    test(`(a) 1280 px: ${eingabe} per ${weg} abgesendet (${status}) — der Fokus steht im Feld „Name“, auch beim zweiten Mal`, async ({ browser }) => {
      await mitAllgemein();
      const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await kontext.newPage();
      const fehler = fehlerSammeln(page, { zusaetzlich: { '/api/channels': [409, 422] } });
      await beitreten(page, 'shoff');
      await plus(page).click();
      await namensfeld(page).fill(eingabe);

      for (let runde = 1; runde <= 2; runde += 1) {
        const antwort = page.waitForResponse((ankunft) => ankunft.url().endsWith('/api/channels') && ankunft.request().method() === 'POST');
        await absenden(page);
        expect((await antwort).status()).toBe(status);
        await expect(dialog(page).locator('.lc-fehler span').first()).toHaveText(meldung);
        await expect(namensfeld(page)).toBeFocused();
        expect(await aktivesElement(page), `Runde ${runde}`).toMatchObject({ id: 'lc-channel-name', sichtbar: true });
      }
      await expect(namensfeld(page)).toHaveValue(eingabe);
      expect(fehler).toEqual([]);
      await kontext.close();
    });
  }
}
