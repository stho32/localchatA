'use strict';
// R00004 Nacharbeit 1 aus dem UX-Review (S1–S6, O1): Zeilen, Anzahl am Schalter, Liste als Fläche unter
// 720 px, „Stand unbekannt“ bei unterbrochener Verbindung, ruhiger Alarmtext, Fokus beim Einklappen.
const { test, expect } = require('./rahmen');
const { instanzStarten, freierPort, fehlerSammeln, beitretenPerApi, stromOeffnen, NICHT_MEHR_ONLINE } = require('./hilfen');

let instanz;
const offeneStroeme = [];
test.afterEach(async () => {
  offeneStroeme.splice(0).forEach((strom) => strom.schliessen());
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const region = (page) => page.getByRole('complementary', { name: 'Teilnehmer' });
const schalter = (page) => page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ });
const zeileVon = (page, kennung) => region(page).locator('.lc-person').filter({ has: page.locator('.lc-id', { hasText: kennung }) });

async function beitreten(page, name = 'shoff') {
  await page.goto(instanz.adresse);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText(`${name}@127.0.0.1`);
}

async function agentMitStrom(name) {
  const { daten } = await beitretenPerApi(instanz.adresse, name);
  const strom = await stromOeffnen(instanz.adresse, daten.token);
  offeneStroeme.push(strom);
  return strom;
}

async function listeZeigen(page) {
  if (!(await region(page).isVisible())) {
    await schalter(page).click();
  }
  await expect(region(page)).toBeVisible();
}

// S1 nach Nacharbeit 2 (N3, Spalte so breit wie --lc-members): eine Zeile, die in die Spalte passt, ist 28 px hoch;
// „Agent“ und „(du)“ stehen im selben Textblock hinter der Kennung und brechen nur bei Platzmangel um.
test('S1: passende Zeilen sind 28 px hoch, „Agent“ und „(du)“ stehen hinter der Kennung', async ({ page, browser }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await agentMitStrom('claude');
  const mia = await (await browser.newContext()).newPage();
  await beitreten(mia, 'mia');
  await beitreten(page);
  await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-dot')).toHaveClass(/\bon\b/);

  const hoehe = await zeileVon(page, 'mia@127.0.0.1').evaluate((zeile) => zeile.getBoundingClientRect().height);

  expect(hoehe).toBe(28);
  await expect(zeileVon(page, 'shoff@127.0.0.1').locator('.lc-person-text .lc-du')).toHaveText('(du)');
  await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-person-text .lc-tag')).toHaveText('Agent');
});

// R00005: die Breite steht an einer Stelle (--lc-members), 264 px nach Entscheidung des Betreibers 2026-10-28
// (ADR-0019 Regel 7, statt 240 px aus dem Designsystem).
test('N3: die Teilnehmerspalte ist bei 1280 px so breit wie --lc-members, 264 px', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);

  const breiten = await page.locator('.lc-main').evaluate((element) => ({
    spalten: getComputedStyle(element).gridTemplateColumns,
    token: getComputedStyle(document.documentElement).getPropertyValue('--lc-members').trim(),
  }));

  expect(breiten.token).toBe('264px');
  expect(breiten.spalten).toMatch(new RegExp(` ${breiten.token}$`));
});

for (const thema of ['light', 'dark']) {
  test(`N2: offline ist die Kennung gedämpft, online in kd-ink — ${thema}`, async ({ page }) => {
    instanz = await instanzStarten();
    await page.emulateMedia({ colorScheme: thema });
    await page.setViewportSize({ width: 1280, height: 800 });
    const strom = await agentMitStrom('claude');
    await beitreten(page);
    const farbe = (kennung) => zeileVon(page, kennung).locator('.lc-id').evaluate((element) => getComputedStyle(element).color);
    const online = await farbe('shoff@127.0.0.1');
    strom.schliessen();
    await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-status')).toHaveText(NICHT_MEHR_ONLINE, { timeout: 3000 });

    const erwartet = {
      light: { offline: 'rgb(79, 93, 107)', online: 'rgb(28, 37, 48)' },
      dark: { offline: 'rgb(154, 167, 180)', online: 'rgb(227, 232, 238)' },
    }[thema];
    expect(await farbe('claude@127.0.0.1')).toBe(erwartet.offline);
    expect(online).toBe(erwartet.online);
  });
}

// Kontrast nach WCAG 2.x mit eingerechneter Deckkraft aller Vorfahren: Text ≥ 4,5:1 gegen die Fläche
// darunter, Statusform (Füllung oder Rand) ≥ 3:1.
function kontrasteInDerListe() {
  function rgb(farbe) {
    return farbe.match(/[\d.]+/g).slice(0, 4).map(Number);
  }
  function leuchtdichte([r, g, b]) {
    const [lr, lg, lb] = [r, g, b].map((wert) => {
      const kanal = wert / 255;
      if (kanal <= 0.03928) {
        return kanal / 12.92;
      }
      return ((kanal + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
  }
  function flaecheUnter(element) {
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      const farbe = rgb(getComputedStyle(knoten).backgroundColor);
      if (farbe.length < 4 || farbe[3] > 0) {
        return farbe.slice(0, 3);
      }
    }
    return [255, 255, 255];
  }
  function deckkraft(element) {
    let produkt = 1;
    for (let knoten = element; knoten !== null; knoten = knoten.parentElement) {
      produkt *= Number(getComputedStyle(knoten).opacity);
    }
    return produkt;
  }
  function kontrast(vorn, hinten, alpha) {
    const gemischt = vorn.map((kanal, stelle) => kanal * alpha + hinten[stelle] * (1 - alpha));
    const [hell, dunkel] = [leuchtdichte(gemischt), leuchtdichte(hinten)].sort((a, b) => b - a);
    return (hell + 0.05) / (dunkel + 0.05);
  }
  const liste = document.querySelector('.lc-members');
  const texte = [...liste.querySelectorAll('*')]
    .filter((element) => [...element.childNodes].some((kind) => kind.nodeType === 3 && kind.textContent.trim() !== ''))
    .map((element) => ({
      text: element.textContent.trim().slice(0, 30),
      wert: kontrast(rgb(getComputedStyle(element).color).slice(0, 3), flaecheUnter(element), deckkraft(element)),
    }));
  const formen = [...liste.querySelectorAll('.lc-dot')].map((punkt) => {
    const stil = getComputedStyle(punkt);
    let farbe = rgb(stil.backgroundColor);
    if (farbe.length === 4 && farbe[3] === 0) {
      farbe = rgb(stil.borderTopColor);
    }
    return { text: punkt.getAttribute('aria-label'), wert: kontrast(farbe.slice(0, 3), flaecheUnter(punkt.parentElement), deckkraft(punkt)) };
  });
  return { texte: texte.filter((eintrag) => eintrag.wert < 4.5), formen: formen.filter((eintrag) => eintrag.wert < 3) };
}

for (const thema of ['light', 'dark']) {
  for (const breite of [1280, 320]) {
    test(`N1 und Name: „Stand unbekannt“ hält die Kontraste und nennt den Punkt „zuletzt online“ — ${thema}, ${breite} px`, async ({ page }) => {
      instanz = await instanzStarten({ port: await freierPort() });
      await page.emulateMedia({ colorScheme: thema });
      await page.setViewportSize({ width: breite, height: 800 });
      const strom = await agentMitStrom('claude');
      await agentMitStrom('mia');
      await beitreten(page);
      await listeZeigen(page);
      strom.schliessen();
      await expect(zeileVon(page, 'claude@127.0.0.1').locator('.lc-status')).toHaveText(NICHT_MEHR_ONLINE, { timeout: 3000 });

      await instanz.beenden();
      await expect(region(page).locator('.lc-stand')).toHaveText(/Stand unbekannt/);

      const zuSchwach = await page.evaluate(kontrasteInDerListe);
      expect(zuSchwach).toEqual({ texte: [], formen: [] });
      const namen = await region(page).locator('.lc-dot.on').evaluateAll((punkte) => punkte.map((punkt) => punkt.getAttribute('aria-label')));
      expect(namen.length).toBeGreaterThan(0);
      expect(new Set(namen)).toEqual(new Set(['zuletzt online']));

      await instanz.starten();
      await expect(region(page).locator('.lc-stand')).toBeHidden({ timeout: 15000 });
      const wieder = await region(page).locator('.lc-dot.on').evaluateAll((punkte) => punkte.map((punkt) => punkt.getAttribute('aria-label')));
      expect(new Set(wieder)).toEqual(new Set(['online']));
    });
  }
}

for (const breite of [1280, 320]) {
  test(`S2: geht ein Agent mit 32 Zeichen offline, verschiebt sich die Zeile darunter um 0 px — ${breite} px`, async ({ page }) => {
    instanz = await instanzStarten();
    await page.setViewportSize({ width: breite, height: 800 });
    const langerName = 'agent-mit-zweiunddreissig-zeichen'.slice(0, 32);
    const strom = await agentMitStrom(langerName);
    await agentMitStrom('zz-darunter');
    await beitreten(page);
    await listeZeigen(page);
    const lang = zeileVon(page, `${langerName}@127.0.0.1`);
    const darunter = zeileVon(page, 'zz-darunter@127.0.0.1');
    await expect(lang.locator('.lc-dot')).toHaveClass(/\bon\b/);
    const vorher = await darunter.evaluate((zeile) => zeile.getBoundingClientRect().top);

    strom.schliessen();
    await expect(lang.locator('.lc-status')).toHaveText(NICHT_MEHR_ONLINE, { timeout: 3000 });
    const nachher = await darunter.evaluate((zeile) => zeile.getBoundingClientRect().top);

    expect(nachher - vorher).toBe(0);
  });
}

test('S3: der Schalter nennt die Anzahl — 3, nach einem „weg“ 2, ohne Neuladen', async ({ page }) => {
  instanz = await instanzStarten({ umgebung: { LOCALCHATA_GNADENFRIST_MS: '3000' } });
  await page.setViewportSize({ width: 1024, height: 800 });
  await agentMitStrom('claude');
  expect((await beitretenPerApi(instanz.adresse, 'kurz')).status).toBe(201);
  await beitreten(page);

  await expect(schalter(page)).toHaveAccessibleName('Teilnehmer 3');
  await expect(schalter(page).locator('.lc-count')).toHaveText('3');
  await expect(schalter(page)).toHaveAccessibleName('Teilnehmer 2', { timeout: 6000 });
});

test('S4: unter 720 px legt sich die Liste über die Mitte; Fokus hinein, Esc schließt, Fokus zurück auf den Schalter', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page);
  await expect(region(page)).toBeHidden();

  await schalter(page).focus();
  await page.keyboard.press('Enter');
  await expect(region(page)).toBeVisible();
  await expect(region(page)).toBeFocused();
  const flaeche = await page.evaluate(() => {
    const liste = document.querySelector('.lc-members').getBoundingClientRect();
    const mitte = document.querySelector('.lc-center').getBoundingClientRect();
    return { links: liste.left, breite: liste.width, mitteBreite: mitte.width, fenster: window.innerWidth };
  });
  await page.keyboard.press('Escape');

  expect(flaeche.links).toBe(0);
  expect(flaeche.breite).toBe(flaeche.fenster);
  await expect(region(page)).toBeHidden();
  await expect(schalter(page)).toBeFocused();
});

test('S4: bei 320 px mit eingeblendeter Liste ist der Hinweis bei unterbrochener Verbindung lesbar, ohne Querscrollen', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 320, height: 640 });
  await beitreten(page);
  await listeZeigen(page);

  await instanz.beenden();
  // Seit R00025 F0084 ist der Hinweis die Leiste oben über der Mitte (vorher #lc-verbindung).
  const hinweis = page.locator('.lc-center > .lc-leiste.lc-leiste-fehler');
  await expect(hinweis).toBeVisible();

  const lage = await hinweis.evaluate((element) => {
    const rechteck = element.getBoundingClientRect();
    const oben = document.elementFromPoint(rechteck.left + rechteck.width / 2, rechteck.top + rechteck.height / 2);
    return { vorn: element.contains(oben), inhalt: document.documentElement.scrollWidth, fenster: window.innerWidth };
  });
  expect(lage.vorn).toBe(true);
  expect(lage.inhalt).toBeLessThanOrEqual(lage.fenster);
});

test('S5: Server beendet — nach < 2 s ist die Liste als „Stand unbekannt“ markiert, kein ● mehr unmarkiert', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await agentMitStrom('claude');
  await beitreten(page);
  await expect(region(page).locator('.lc-dot.on')).toHaveCount(2);

  await instanz.beenden();

  await expect(region(page).locator('.lc-stand')).toHaveText(/Stand unbekannt/, { timeout: 2000 });
  const unmarkiert = await page.evaluate(() =>
    [...document.querySelectorAll('.lc-dot.on')].filter((punkt) => punkt.closest('.lc-unbekannt') === null).length);
  expect(unmarkiert).toBe(0);
  // R00005 (ADR-0019 Regel 6): „zuletzt online“ ist grau gefüllt in lc-offline, nicht mehr gestrichelt.
  const punkt = await region(page).locator('.lc-dot.on').first().evaluate((element) => {
    const stil = getComputedStyle(element);
    const probe = document.createElement('span');
    probe.style.color = 'var(--lc-offline)';
    document.body.append(probe);
    const token = getComputedStyle(probe).color;
    probe.remove();
    return { fuellung: stil.backgroundColor, token, rand: stil.borderTopStyle };
  });
  expect(punkt.fuellung).toBe(punkt.token);
  expect(punkt.rand).not.toBe('dashed');
});

// S6 seit R00025 F0084: die Warnung ist die Leiste über der Mitte, deren Countdown sich je Sekunde ändert — sie ist darum
// keine Live-Region (kein role, kein aria-live), und auf der Seite gibt es keinen Alarm (role="alert") mehr; angesagt
// wird über die eine Ansage-Region (R00025 „Ansage“).
test('S6: die Warnung bei unterbrochener Verbindung ist keine Live-Region, kein Alarm auf der Seite', async ({ page }) => {
  instanz = await instanzStarten({ port: await freierPort() });
  fehlerSammeln(page, { serverWegErwartet: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await expect(region(page).locator('.lc-dot.on')).toHaveCount(1);

  await instanz.beenden();
  const warnung = page.locator('.lc-center > .lc-leiste.lc-leiste-fehler');
  await expect(warnung).toBeVisible();

  expect(await warnung.getAttribute('role')).toBeNull();
  expect(await warnung.getAttribute('aria-live')).toBeNull();
  await expect(page.locator('[role="alert"]')).toHaveCount(0);
});

test('O1: klappt die Liste durch Verkleinern ein, während der Fokus in ihr steht, geht der Fokus auf den Schalter', async ({ page }) => {
  instanz = await instanzStarten();
  await page.setViewportSize({ width: 1280, height: 800 });
  await beitreten(page);
  await region(page).focus();

  await page.setViewportSize({ width: 1024, height: 800 });

  await expect(region(page)).toBeHidden();
  await expect(schalter(page)).toBeFocused();
});
