'use strict';
// R00009 F0039 (Befundkarte #20): die eigene Kennung in der Kopfzeile sprengt das schmale Fenster nicht — sie bricht
// um, die Kopfzeile wächst mit; bei 1280 px bleibt die Kopfzeile 40 px hoch und die Kennung einzeilig.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('@playwright/test');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

const NAME_13 = 'abcdefghijklm';
const NAME_32 = 'abcdefghijklmnopqrstuvwxyz012345';
const IPV6_45 = 'ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255';

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const kopf = (page) => page.getByRole('banner');
const kennung = (page) => kopf(page).locator('.lc-me');
const teilnehmerSchalter = (page) => kopf(page).getByRole('button', { name: /^Teilnehmer/ });

async function fensterOeffnen(page, { breite, hoehe, schema }) {
  await page.emulateMedia({ colorScheme: schema });
  await page.setViewportSize({ width: breite, height: hoehe });
}

async function beitreten(page, name, url = instanz.adresse) {
  await page.goto(url);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(kennung(page)).toHaveText(`${name}@127.0.0.1`);
}

// Die längste Kennung (Name 32, IPv6-Herkunft 45 Zeichen) ist über einen Beitritt auf 127.0.0.1 nicht herstellbar:
// die Kopfzeile wird für einen Agenten neu gezeichnet und ersetzt die echte an ihrem Platz im Seitenraster; der
// Schalter „Teilnehmer“ zieht mit um.
async function kopfzeileFuerAgentZeichnen(page, name, herkunft) {
  await page.evaluate(async ({ name, herkunft }) => {
    const { zeichnen } = await import('/web/kopfzeile.js');
    const echteKopfzeile = document.querySelector('#app > .lc-head');
    const schalter = echteKopfzeile.querySelector('.lc-hbtn');
    const behaelter = document.createElement('div');
    zeichnen(behaelter, { name, herkunft, art: 'agent' }, { schalter: [schalter] });
    echteKopfzeile.replaceWith(behaelter.firstElementChild);
  }, { name, herkunft });
  await expect(kennung(page)).toHaveText(`${name}@${herkunft}`);
}

function vermessen() {
  const kennungElement = document.querySelector('.lc-head .lc-me');
  const herkunftElement = kennungElement.querySelector('i');
  const kennungRect = kennungElement.getBoundingClientRect();
  const kennungStil = getComputedStyle(kennungElement);
  const kopfzeileRect = document.querySelector('.lc-head').getBoundingClientRect();
  return {
    kopfzeile: { oben: kopfzeileRect.top, unten: kopfzeileRect.bottom },
    hauptOben: document.querySelector('.lc-main').getBoundingClientRect().top,
    seite: document.documentElement.scrollWidth,
    fenster: window.innerWidth,
    kopfzeileHoehe: kopfzeileRect.height,
    kennung: {
      oben: kennungRect.top,
      unten: kennungRect.bottom,
      links: kennungRect.left,
      rechts: kennungRect.right,
      hoehe: kennungRect.height,
      zeilenhoehe: parseFloat(kennungStil.lineHeight),
      noetig: kennungElement.scrollWidth,
      sichtbar: kennungElement.clientWidth,
      auslassung: kennungStil.textOverflow,
      farbe: kennungStil.color,
      herkunftFarbe: getComputedStyle(herkunftElement).color,
      gedaempft: getComputedStyle(document.querySelector('.lc-head-inst')).color,
    },
  };
}

function ganzImFenster(element, fenster) {
  return element.left >= 0 && element.right <= fenster && element.width > 0 && element.height > 0;
}

async function rechteckImFenster(locator) {
  return locator.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    return { left: rect.left, right: rect.right, width: rect.width, height: rect.height };
  });
}

async function kennungVollstaendigOhneQuerscrollen(page) {
  const mass = await page.evaluate(vermessen);
  expect(mass.seite, 'scrollWidth der Seite').toBeLessThanOrEqual(mass.fenster);
  expect(mass.kennung.links, 'Kennung links im Fenster').toBeGreaterThanOrEqual(0);
  expect(mass.kennung.rechts, 'Kennung rechts im Fenster').toBeLessThanOrEqual(mass.fenster);
  expect(mass.kennung.oben, 'Kennung oben in der Kopfzeile').toBeGreaterThanOrEqual(mass.kopfzeile.oben);
  expect(mass.kennung.unten, 'Kennung unten in der Kopfzeile').toBeLessThanOrEqual(mass.kopfzeile.unten);
  expect(mass.hauptOben, 'Hauptansicht unter der Kopfzeile').toBeGreaterThanOrEqual(mass.kopfzeile.unten);
  expect(mass.kennung.noetig, 'Kennung nicht abgeschnitten').toBeLessThanOrEqual(mass.kennung.sichtbar);
  expect(mass.kennung.auslassung).not.toBe('ellipsis');
  expect(mass.kennung.herkunftFarbe, 'Herkunft gedämpft').toBe(mass.kennung.gedaempft);
  expect(mass.kennung.herkunftFarbe).not.toBe(mass.kennung.farbe);
  return mass;
}

async function teilnehmerSchalterPerTabErreichbar(page) {
  const schalter = teilnehmerSchalter(page);
  await expect(schalter).toBeVisible();
  expect(ganzImFenster(await rechteckImFenster(schalter), page.viewportSize().width), 'Schalter ganz im Fenster').toBe(true);
  const beschriftung = await schalter.evaluate((element) => ({
    breiteNoetig: element.scrollWidth, breite: element.clientWidth, hoeheNoetig: element.scrollHeight, hoehe: element.clientHeight,
  }));
  expect(beschriftung.breiteNoetig, 'Schalter nicht gekürzt').toBeLessThanOrEqual(beschriftung.breite);
  expect(beschriftung.hoeheNoetig, 'Schalter einzeilig').toBeLessThanOrEqual(beschriftung.hoehe);
  await kennung(page).focus();
  await page.keyboard.press('Tab');
  await expect(schalter).toBeFocused();
}

for (const breite of [320, 360, 375]) {
  for (const schema of ['light', 'dark']) {
    for (const name of [NAME_13, NAME_32]) {
      test(`lange eigene Kennung bei ${breite} px ohne Querscrollen — ${schema}, Name ${name.length} Zeichen`, async ({ page }) => {
        const fehler = fehlerSammeln(page);
        instanz = await instanzStarten();
        await fensterOeffnen(page, { breite, hoehe: 640, schema });

        await beitreten(page, name);

        await kennungVollstaendigOhneQuerscrollen(page);
        await teilnehmerSchalterPerTabErreichbar(page);
        expect(fehler).toEqual([]);
      });
    }

    test(`längste Kennung (78 Zeichen, Agent) bei ${breite} px ohne Querscrollen — ${schema}`, async ({ page }) => {
      const fehler = fehlerSammeln(page);
      instanz = await instanzStarten();
      await fensterOeffnen(page, { breite, hoehe: 640, schema });
      await beitreten(page, 'shoff');

      await kopfzeileFuerAgentZeichnen(page, NAME_32, IPV6_45);

      expect(`${NAME_32}@${IPV6_45}`).toHaveLength(78);
      await kennungVollstaendigOhneQuerscrollen(page);
      const marke = kopf(page).locator('.lc-tag');
      await expect(marke).toHaveText('Agent');
      expect(ganzImFenster(await rechteckImFenster(marke), breite), 'Marke „Agent“ ganz im Fenster').toBe(true);
      await teilnehmerSchalterPerTabErreichbar(page);
      expect(fehler).toEqual([]);
    });
  }
}

// Ab 480 px kehrt die Navigation als dritte Spalte zurück, die Lücke wächst auf 16 px — ohne Fix 491 px breit.
for (const schema of ['light', 'dark']) {
  test(`Grenze der Media-Query 480 px: Name 32 Zeichen ohne Querscrollen — ${schema}`, async ({ page }) => {
    instanz = await instanzStarten();
    await fensterOeffnen(page, { breite: 480, hoehe: 640, schema });

    await beitreten(page, NAME_32);

    await kennungVollstaendigOhneQuerscrollen(page);
    await teilnehmerSchalterPerTabErreichbar(page);
  });
}

for (const schema of ['light', 'dark']) {
  test(`Kontrollpaar 1280 px: Kopfzeile 40 px, Kennung einzeilig — ${schema}`, async ({ page }) => {
    instanz = await instanzStarten();
    await fensterOeffnen(page, { breite: 1280, hoehe: 800, schema });

    await beitreten(page, NAME_32);

    const mass = await kennungVollstaendigOhneQuerscrollen(page);
    expect(mass.kopfzeileHoehe).toBe(40);
    expect(mass.kennung.hoehe).toBe(mass.kennung.zeilenhoehe);
  });
}

// Die Kopfzeile aus I0008 trägt nach dem Instanznamen den Channel samt Beschreibung (Zone „Ort“, minmax(0, 1fr)).
test('320 px mit gewähltem Channel und Beschreibung: lange Kennung ohne Querscrollen', async ({ page }) => {
  instanz = await instanzStarten({
    vorbereiten: (ordner) => {
      const channel = path.join(ordner, 'channels', 'security-entwicklung');
      fs.mkdirSync(channel, { recursive: true });
      fs.writeFileSync(path.join(channel, 'einstellungen.toml'), 'beschreibung = "Absprachen zwischen Security- und Entwicklungs-Factory"\n');
    },
  });
  await fensterOeffnen(page, { breite: 320, hoehe: 640, schema: 'light' });

  await beitreten(page, NAME_32, `${instanz.adresse}#/c/security-entwicklung`);

  await expect(kopf(page).locator('.lc-head-chan')).toHaveText('# security-entwicklung');
  await kennungVollstaendigOhneQuerscrollen(page);
  await teilnehmerSchalterPerTabErreichbar(page);
});
