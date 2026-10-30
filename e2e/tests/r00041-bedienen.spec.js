'use strict';
// R00041 F0135 Eigene Nachricht im Browser bearbeiten: Aktionen am Desktop und per Taste (B0609), Feld an Ort und
// Stelle (B0610), Speichern (B0611), Ansagen (B0612), Tasten im Feld (B0627), Menü „⋯“ auf Touch (B0628), Feld am
// Telefon (B0629).
const path = require('path');
const { execFileSync } = require('child_process');
const { test, expect } = require('./rahmen');
const { instanzStarten, channelAnlegen, sichtbarNach } = require('./hilfen');
const {
  JETZT, sendenStarten, imBrowserBeitreten, eingabe, nachrichten, beitreten, senden, anfragen,
} = require('./senden-hilfen');
const {
  betaSendet, browserOeffnen, imBrowserSenden, nachrichtMitZeit, zeigtBearbeitet, loeschenPerApi, textknotenMit,
  ansagenMitschreiben,
} = require('./r00041-hilfen');

const QUOTE_ORDNER = path.resolve(__dirname, '../../target/debug/examples/quote-ordner');

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

test('Tab-Stopps (B0609, R00046, R00047): von der fokussierten eigenen Nachricht erreicht Tab „Antworten“, „Kopieren“, „Bearbeiten“, „Löschen“, dann die Eingabe; von einer fremden nur „Antworten“ und „Kopieren“, dann die Eingabe', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserSenden(a.page, 'Hallo');
  const beta = await betaSendet(instanz.adresse, ['Fremd']);
  await expect(nachrichtMitZeit(a.page, beta.zeiten[0])).toHaveCount(1);

  await eingabe(a.page).focus();
  await a.page.keyboard.press('ArrowUp');
  expect(await tabBisEingabe(a.page)).toEqual(['Antworten', 'Kopieren', 'Bearbeiten', 'Löschen']);
  await nachrichtMitZeit(a.page, beta.zeiten[0]).focus();
  expect(await tabBisEingabe(a.page)).toEqual(['Antworten', 'Kopieren']);
  expect(a.fehler).toEqual([]);
});

test('Nur eigene (B0609, R00046, R00047): an Nachrichten von anna und beta kein Bearbeiten und Löschen, nur Antworten und Kopieren; an „wird gesendet …“ und fehlgeschlagenen gar keine Aktion', async ({ browser }) => {
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
    for (const name of ['Bearbeiten', 'Löschen']) {
      await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name })).toHaveCount(0);
    }
    await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-aktion')).toHaveText(['Antworten', 'Kopieren']);
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

const BEARBEITEN = '/api/channels/allgemein/nachrichten/bearbeiten';

/// Öffnet an der eigenen Nachricht `zeit` das Feld über Hover und „Bearbeiten“.
async function feldOeffnen(page, zeit) {
  await nachrichtMitZeit(page, zeit).hover();
  await knopf(page, zeit, 'Bearbeiten').click();
  await expect(feld(page)).toBeFocused();
}

test('Desktop mit Maus (B0611): „Speichern“ schickt genau ein POST mit Idempotency-Key; A und B zeigen den neuen Text mit „bearbeitet“, B unter 1 s; Fokus bei A auf der Nachricht', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const b = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await expect(nachrichtMitZeit(b.page, zeit)).toHaveCount(1);
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' zusammen');

  const antwort = a.page.waitForResponse((ankunft) => new URL(ankunft.url()).pathname === BEARBEITEN);
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).click();
  expect((await antwort).status()).toBe(200);
  const ab = Date.now();
  const bisB = await sichtbarNach(b.page, zeigtBearbeitet, { argument: { zeit, text: 'Hallo zusammen' }, ab, timeout: 10000 });
  expect(bisB).toBeLessThan(1000);
  await expect(feld(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo zusammen');
  await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'bearbeitet', exact: true })).toBeVisible();
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  expect(bearbeitungen).toHaveLength(1);
  expect(bearbeitungen[0].koerper).toEqual({ zeit, text: 'Hallo zusammen' });
  expect(bearbeitungen[0].schluessel).toMatch(/^[A-Za-z0-9_-]{1,64}$/);
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Doppelklick und Strg Enter (B0611, S7): ein Doppelklick auf „Speichern“ schickt genau ein POST; Strg Enter speichert ebenso', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' eins');
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).dblclick();
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo eins');
  await a.page.waitForTimeout(300);
  expect(bearbeitungen).toHaveLength(1);

  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' zwei');
  await a.page.keyboard.press('Control+Enter');
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo eins zwei');
  expect(bearbeitungen).toHaveLength(2);
  expect(bearbeitungen[1].schluessel).not.toBe(bearbeitungen[0].schluessel);
  expect(a.fehler).toEqual([]);
});

test('Unverändert (B0611): Speichern ohne Änderung schickt genau ein POST, das Feld ist zu, keine Marke „bearbeitet“', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  const bearbeitungen = bearbeitungenMitschreiben(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.press('Enter');
  await expect(feld(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo');
  await expect(nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'bearbeitet', exact: true })).toHaveCount(0);
  expect(bearbeitungen).toHaveLength(1);
  expect(a.fehler).toEqual([]);
});

/// Init-Skript (Tab ohne SharedWorker): Ereignisse `aenderung` erreichen die Seite nicht — so bleibt ein Feld offen,
/// dessen Nachricht der Test inzwischen gelöscht hat (sonst schlösse es B0636 schon vor dem Speichern).
async function aenderungenVerschlucken(page) {
  await page.addInitScript(() => {
    const echtesFetch = window.fetch.bind(window);
    window.fetch = async (pfad, optionen) => {
      const antwort = await echtesFetch(pfad, optionen);
      if (!String(pfad).endsWith('/api/ereignisse') || antwort.body === null) {
        return antwort;
      }
      const leser = antwort.body.pipeThrough(new TextDecoderStream()).getReader();
      const kodierer = new TextEncoder();
      const strom = new ReadableStream({
        async start(steuerung) {
          let rest = '';
          for (;;) {
            const { value, done } = await leser.read().catch(() => ({ done: true }));
            if (done) {
              steuerung.close();
              return;
            }
            rest += value;
            const bloecke = rest.split('\n\n');
            rest = bloecke.pop();
            bloecke.filter((block) => !block.includes('"art":"aenderung"')).forEach((block) => steuerung.enqueue(kodierer.encode(`${block}\n\n`)));
          }
        },
      });
      return new Response(strom, { status: antwort.status, headers: antwort.headers });
    };
    window.zwischenablageBeschrieben = 0;
    if (navigator.clipboard) {
      navigator.clipboard.writeText = async () => { window.zwischenablageBeschrieben += 1; };
    }
  });
}

test('Inzwischen gelöscht (B0611, PO 122): 409 → Feld zu, „Nachricht gelöscht“, Hinweiszeile; das Getippte wird nicht gerettet', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { ohneWorker: true, zusaetzlich: { [BEARBEITEN]: [409] } });
  await aenderungenVerschlucken(a.page);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await eingabe(a.page).fill('Entwurf unten');
  await feldOeffnen(a.page, zeit);
  await feld(a.page).fill('Ganz anders getippt');
  const token = await a.page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  expect((await loeschenPerApi(instanz.adresse, token, zeit, '127.0.0.1')).status).toBe(200);
  await a.page.waitForTimeout(300);
  await expect(feld(a.page)).toBeVisible();

  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).click();
  await expect(feld(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Nachricht gelöscht');
  await expect(a.page.locator('.lc-statuszeile')).toHaveText('Die Nachricht wurde inzwischen gelöscht.');
  expect(await textknotenMit(a.page, 'Ganz anders getippt')).toEqual([]);
  await expect(eingabe(a.page)).toHaveValue('Entwurf unten');
  expect(await a.page.evaluate(() => window.zwischenablageBeschrieben)).toBe(0);
  expect(a.fehler).toEqual([]);
});

test('Keine Antwort beim Speichern (B0611, S5): abgebrochener POST → „⚠ Nicht gespeichert: keine Antwort vom Server.“ am Feld, Text bleibt; ein zweites Speichern gelingt', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { serverWegErwartet: false });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await feld(a.page).fill('Hallo neu');
  await a.page.route(`**${BEARBEITEN}`, (route) => route.abort(), { times: 1 });
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).click();
  const hinweis = nachrichtMitZeit(a.page, zeit).getByText('⚠ Nicht gespeichert: keine Antwort vom Server.', { exact: true });
  await expect(hinweis).toBeVisible();
  await expect(feld(a.page)).toHaveValue('Hallo neu');
  await expect(feld(a.page)).toHaveAccessibleDescription(/⚠ Nicht gespeichert: keine Antwort vom Server\./);
  await expect(feld(a.page)).not.toHaveAttribute('aria-invalid', 'true');

  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).click();
  await expect(feld(a.page)).toHaveCount(0);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Hallo neu');
  expect(a.fehler.filter((zeile) => !/ERR_FAILED/.test(zeile))).toEqual([]);
});

test('Platte voll beim Speichern (B0611, 507): Text der Instanz mit ⚠ am Feld über aria-describedby, kein aria-invalid, das Getippte bleibt', async ({ browser }) => {
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
  await feldOeffnen(a.page, zeit);
  const lang = 'x'.repeat(400);
  await feld(a.page).fill(lang);
  const antwort = a.page.waitForResponse((ankunft) => new URL(ankunft.url()).pathname === BEARBEITEN);
  await nachrichtMitZeit(a.page, zeit).getByRole('button', { name: 'Speichern' }).click();
  const fehler = await (await antwort).json();
  expect((await antwort).status()).toBe(507);
  await expect(nachrichtMitZeit(a.page, zeit).getByText(`⚠ ${fehler.fehler}`, { exact: true })).toBeVisible();
  await expect(feld(a.page)).toHaveAccessibleDescription(new RegExp(`⚠ ${fehler.fehler.slice(0, 30).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`));
  await expect(feld(a.page)).not.toHaveAttribute('aria-invalid', 'true');
  await expect(feld(a.page)).toHaveValue(lang);
  expect(a.fehler).toEqual([]);
});

test('Frühere Sitzung (B0611): shoff sendet per curl und verlässt; Browser A tritt als shoff bei → an „Früher“ gibt es „Bearbeiten“, Speichern gelingt', async ({ browser }) => {
  instanz = await sendenStarten();
  const token = await beitreten(instanz.adresse, 'shoff', { art: 'mensch' });
  const gesendet = await senden(instanz.adresse, token, 'allgemein', 'Früher', '127.0.0.1');
  expect(gesendet.status).toBe(201);
  const zeit = gesendet.daten.nachricht.zeit;
  expect((await anfragen(instanz.adresse, { methode: 'POST', pfad: 'api/verlassen', token })).status).toBeLessThan(300);
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' und jetzt');
  const antwort = a.page.waitForResponse((ankunft) => new URL(ankunft.url()).pathname === BEARBEITEN);
  await a.page.keyboard.press('Enter');
  expect((await antwort).status()).toBe(200);
  await expect(nachrichtMitZeit(a.page, zeit).locator('.lc-body')).toHaveText('Früher und jetzt');
  expect(a.fehler).toEqual([]);
});

test('Während des Bearbeitens (B0611): beta sendet drei Nachrichten, während A tippt → sie erscheinen, das Feld behält Text und Fokus', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.type(' zus');
  await betaSendet(instanz.adresse, ['Eins', 'Zwei', 'Drei']);
  await expect(nachrichten(a.page)).toHaveCount(4);
  await a.page.keyboard.type('ammen');
  await expect(feld(a.page)).toHaveValue('Hallo zusammen');
  await expect(feld(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
});

test('Ansagen (B0612, S2, S3): beim Öffnen keine; Speichern genau einmal „Änderung gespeichert.“, unverändert „Keine Änderung.“, Esc „Bearbeiten abgebrochen.“; Fokus sichtbar an den Knöpfen', async ({ browser }) => {
  instanz = await sendenStarten();
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const zeit = await imBrowserSenden(a.page, 'Hallo');
  await a.page.waitForTimeout(300);
  const angesagt = await ansagenMitschreiben(a.page);

  await feldOeffnen(a.page, zeit);
  await a.page.waitForTimeout(300);
  expect(await angesagt()).toEqual([]);
  for (const name of ['Abbrechen', 'Speichern']) {
    await nachrichtMitZeit(a.page, zeit).getByRole('button', { name }).focus();
    await a.page.keyboard.press('Shift+Tab');
    await a.page.keyboard.press('Tab');
    expect(await a.page.evaluate(() => document.activeElement.matches(':focus-visible') && getComputedStyle(document.activeElement).outlineStyle !== 'none')).toBe(true);
  }
  await feld(a.page).focus();
  await a.page.keyboard.type(' zusammen');
  await a.page.keyboard.press('Enter');
  await expect(nachrichtMitZeit(a.page, zeit)).toBeFocused();
  await expect.poll(angesagt).toEqual(['Änderung gespeichert.']);

  await a.page.keyboard.press('e');
  await a.page.keyboard.press('Enter');
  await expect.poll(angesagt).toEqual(['Änderung gespeichert.', 'Keine Änderung.']);

  await a.page.keyboard.press('e');
  await a.page.keyboard.press('Escape');
  await expect.poll(angesagt).toEqual(['Änderung gespeichert.', 'Keine Änderung.', 'Bearbeiten abgebrochen.']);
  await a.page.waitForTimeout(300);
  expect(await angesagt()).toHaveLength(3);
  expect(a.fehler).toEqual([]);
});

test('Tasten im Feld (B0627, UX M2): Ende, Pos1, ↑, ↓, Leertaste, E und Entf wirken nur auf den Text — Feld und Fokus bleiben, der Verlauf scrollt nicht, kein Dialog', async ({ browser }) => {
  instanz = await sendenStarten({ anzahl: 60 });
  const a = await browserOeffnen(browser, { streng: true });
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await expect(nachrichten(a.page)).toHaveCount(60);
  const zeit = await imBrowserSenden(a.page, 'Eins');
  await feldOeffnen(a.page, zeit);
  await a.page.keyboard.press('Shift+Enter');
  await a.page.keyboard.type('Zwei');
  await a.page.getByRole('log').evaluate((log) => { log.scrollTop -= 40; });
  await a.page.waitForTimeout(200);
  const scrollVorher = await a.page.getByRole('log').evaluate((log) => log.scrollTop);

  for (const taste of ['End', 'Home', 'ArrowUp', 'ArrowDown', 'Space']) {
    await a.page.keyboard.press(taste);
    await expect(feld(a.page)).toBeFocused();
  }
  await a.page.keyboard.press('End');
  await a.page.keyboard.type('E');
  await a.page.keyboard.press('Home');
  await a.page.keyboard.press('Delete');
  await expect(feld(a.page)).toBeFocused();
  await expect(feld(a.page)).toHaveCount(1);
  await expect(a.page.locator('dialog[open]')).toHaveCount(0);
  expect(await a.page.getByRole('log').evaluate((log) => log.scrollTop)).toBe(scrollVorher);
  const text = await feld(a.page).inputValue();
  expect(text).toContain('E');
  expect(text.length).toBe('Eins\nZwei'.length + ' '.length + 'E'.length - 1);
  expect(a.fehler).toEqual([]);
});
