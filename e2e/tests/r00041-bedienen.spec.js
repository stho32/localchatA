'use strict';
// R00041 F0135 Eigene Nachricht im Browser bearbeiten: Aktionen am Desktop und per Taste (B0609), Feld an Ort und
// Stelle (B0610), Speichern (B0611), Ansagen (B0612), Tasten im Feld (B0627), Menü „⋯“ auf Touch (B0628), Feld am
// Telefon (B0629).
const { test, expect } = require('./rahmen');
const { sendenStarten, imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { betaSendet, browserOeffnen, imBrowserSenden, nachrichtMitZeit } = require('./r00041-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const knopf = (page, zeit, name) => nachrichtMitZeit(page, zeit).getByRole('button', { name, exact: true });

/// Drückt Tab, bis die Eingabe den Fokus hat, und liefert die zugänglichen Namen der Knöpfe unterwegs.
async function tabBisEingabe(page, grenze = 5) {
  const besucht = [];
  for (let schritt = 0; schritt < grenze; schritt += 1) {
    await page.keyboard.press('Tab');
    const stelle = await page.evaluate(() => {
      const aktiv = document.activeElement;
      return { eingabe: aktiv.matches('.lc-comp-feld'), knopf: aktiv.matches('button') ? aktiv.textContent : null };
    });
    if (stelle.eingabe) {
      return besucht;
    }
    besucht.push(stelle.knopf);
  }
  throw new Error(`Eingabe nach ${grenze} Tab nicht erreicht: ${JSON.stringify(besucht)}`);
}

test('Desktop (B0609): an der eigenen Nachricht erscheinen bei Hover „Bearbeiten“ und „Löschen“; kein „⋯“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await eingabe(a.page).blur();
  await a.page.mouse.move(0, 0);
  await expect(knopf(a.page, zeit, 'Bearbeiten')).toBeHidden();

  await nachrichtMitZeit(a.page, zeit).hover();
  await expect(knopf(a.page, zeit, 'Bearbeiten')).toBeVisible();
  await expect(knopf(a.page, zeit, 'Löschen')).toBeVisible();
  await expect(a.page.getByRole('log').getByRole('button', { name: 'Aktionen' })).toHaveCount(0);
  for (const name of ['Bearbeiten', 'Löschen']) {
    const box = await knopf(a.page, zeit, name).boundingBox();
    expect(box.width).toBeGreaterThanOrEqual(24);
    expect(box.height).toBeGreaterThanOrEqual(24);
  }
  expect(a.fehler).toEqual([]);
});

test('Tab-Stopps (B0609): von der fokussierten eigenen Nachricht erreicht Tab „Bearbeiten“, „Löschen“, dann die Eingabe; von einer fremden direkt die Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Hallo');
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  expect(await tabBisEingabe(a.page)).toEqual(['Bearbeiten', 'Löschen']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  expect(await tabBisEingabe(a.page)).toEqual([]);
  expect(a.fehler).toEqual([]);
});

test('Nur eigene (B0609): an Nachrichten von anna und beta weder Aktionen noch Leiste; an „wird gesendet …“ und fehlgeschlagenen auch nicht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { zusaetzlich: { '/api/channels/allgemein/nachrichten': [500] } });
  const b = await browserOeffnen(browser);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const vonAnna = await imBrowserSenden(b.page, 'Von Anna');
  const beta = await betaSendet(instanz.adresse, ['Von Beta']);
  await expect(nachrichten(a.page)).toHaveCount(2);
  for (const zeit of [vonAnna, beta.zeiten[0]]) {
    await nachrichtMitZeit(a.page, zeit).hover();
    await nachrichtMitZeit(a.page, zeit).focus();
    for (const name of ['Bearbeiten', 'Löschen', 'Aktionen']) {
      await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name })).toHaveCount(0);
    }
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktionen, .lc-aktionen-schalter')).toHaveCount(0);
  }

  let freigeben;
  const tor = new Promise((offen) => { freigeben = offen; });
  await a.page.route('**/api/channels/allgemein/nachrichten', async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    await tor;
    await route.fulfill({ status: 500, contentType: 'application/json', body: '{"fehler":"Nicht gespeichert."}' });
  });
  await eingabe(a.page).fill('Unterwegs');
  await eingabe(a.page).press('Enter');
  const vorlaeufig = a.page.getByRole('log').locator('.lc-msg.lc-vorlaeufig');
  await expect(vorlaeufig).toHaveCount(1);
  await vorlaeufig.hover();
  await expect(vorlaeufig.getByRole('button', { name: /^(Bearbeiten|Löschen|Aktionen)$/ })).toHaveCount(0);
  freigeben();
  await expect(a.page.getByRole('log').locator('.lc-msg.lc-fehlgeschlagen')).toHaveCount(1);
  await expect(a.page.getByRole('log').locator('.lc-msg.lc-fehlgeschlagen').getByRole('button', { name: /^(Bearbeiten|Löschen|Aktionen)$/ })).toHaveCount(0);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

const feld = (page) => page.getByRole('log').getByRole('textbox', { name: 'Nachricht bearbeiten' });

/// Zählt die Anfragen `POST …/nachrichten/bearbeiten` (Körper und Idempotency-Key).
function bearbeitungenMitschreiben(page) {
  const liste = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'POST' && anfrage.url().endsWith('/nachrichten/bearbeiten')) {
      liste.push({ koerper: anfrage.postDataJSON(), schluessel: anfrage.headers()['idempotency-key'] });
    }
  });
  return liste;
}

test('Feld an Ort und Stelle (B0610): Klick auf „Bearbeiten“ öffnet das Feld mit dem Text, Fokus darin synchron, Schreibmarke am Ende; Knöpfe „Abbrechen“ · „Speichern“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).hover();

  const fokusImKlick = await knopf(a.page, zeit, 'Bearbeiten').evaluate((bearbeitenKnopf) => {
    bearbeitenKnopf.click();
    return document.activeElement.getAttribute('aria-label');
  });
  expect(fokusImKlick).toBe('Nachricht bearbeiten');
  await expect(feld(a.page)).toHaveValue('Hallo');
  await expect(feld(a.page)).toBeFocused();
  expect(await feld(a.page).evaluate((textfeld) => [textfeld.selectionStart, textfeld.selectionEnd])).toEqual([5, 5]);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toBeHidden();

  const reihe = nachrichtMitZeit(a.page, zeit).locator('.lc-bearbeiten').getByRole('button');
  await expect(reihe).toHaveText(['Abbrechen', 'Speichern']);
  const links = await reihe.evaluateAll((knoepfe) => knoepfe.map((einzeln) => einzeln.getBoundingClientRect().left));
  expect(links[0]).toBeLessThan(links[1]);
  expect(a.fehler).toEqual([]);
});

test('Leer (B0610, UX S4): ohne Anhang leer → „Speichern“ aria-disabled, Klick und Enter senden nichts, Hinweis am Feld', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).hover();
  await knopf(a.page, zeit, 'Bearbeiten').click();

  await feld(a.page).fill('');
  const speichern = nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' });
  await expect(speichern).toHaveAttribute('aria-disabled', 'true');
  await expect(nachrichtMitZeit(a.page, zeit).getByText('Leer geht nicht — zum Entfernen „Löschen“ wählen.', { exact: true })).toBeVisible();
  await expect(feld(a.page)).toHaveAccessibleDescription(/Leer geht nicht — zum Entfernen „Löschen“ wählen\./);
  // aria-disabled gilt Playwright als gesperrt — der Klick wird erzwungen, wie ein Mensch ihn trotzdem tut.
  await speichern.click({ force: true });
  await feld(a.page).press('Enter');
  await a.page.waitForTimeout(300);
  expect(bearbeitungen).toEqual([]);
  await expect(feld(a.page)).toBeVisible();
  expect(a.fehler).toEqual([]);
});

test('Tastatur (B0609, B0610): ↑ in leerer Eingabe, E öffnet das Feld, Umschalt Enter macht eine Zeile; Esc schließt ohne POST, Fokus auf der Nachricht, zweites Esc in die Eingabe; E mit Umschaltern und an fremden nicht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  for (const mitUmschalter of ['Control+e', 'Alt+e', 'Meta+e']) {
    await a.page.keyboard.press(mitUmschalter);
    await expect(feld(a.page)).toHaveCount(0);
  }
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  await a.page.keyboard.press('e');
  await expect(feld(a.page)).toBeFocused();
  await expect(feld(a.page)).toHaveValue('Hallo');
  await a.page.keyboard.press('Shift+Enter');
  await a.page.keyboard.type('Welt');
  await expect(feld(a.page)).toHaveValue('Hallo\nWelt');

  await a.page.keyboard.press('Escape');
  await expect(feld(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo');
  await a.page.keyboard.press('Escape');
  await expect(eingabe(a.page)).toBeFocused();
  expect(bearbeitungen).toEqual([]);

  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  await a.page.keyboard.press('e');
  await expect(feld(a.page)).toHaveCount(0);
  expect(a.fehler).toEqual([]);
});

test('Bildschirmleser (B0610): das Feld heißt „Nachricht bearbeiten“, beschreibt am Desktop seine Tasten; der Rand hat ≥ 3 : 1 gegen die Fläche', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await nachrichtMitZeit(a.page, zeit).hover();
  await knopf(a.page, zeit, 'Bearbeiten').click();
  await expect(feld(a.page)).toHaveAccessibleDescription(/Enter speichert/);
  const kontrast = await feld(a.page).evaluate((textfeld) => {
    const anteile = (farbe) => farbe.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
    const leuchtdichte = (farbe) => {
      const [r, g, b] = anteile(farbe).map((wert) => {
        const kanal = wert / 255;
        if (kanal <= 0.03928) {
          return kanal / 12.92;
        }
        return ((kanal + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const verhaeltnis = (eins, zwei) => (Math.max(eins, zwei) + 0.05) / (Math.min(eins, zwei) + 0.05);
    const rand = leuchtdichte(getComputedStyle(textfeld).borderTopColor);
    const innen = leuchtdichte(getComputedStyle(textfeld).backgroundColor);
    const aussen = leuchtdichte(getComputedStyle(textfeld.closest('.lc-msg')).backgroundColor);
    return Math.min(verhaeltnis(rand, innen), verhaeltnis(rand, aussen));
  });
  expect(kontrast).toBeGreaterThanOrEqual(3);
  expect(a.fehler).toEqual([]);
});
