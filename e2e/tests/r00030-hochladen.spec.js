'use strict';
// R00030 F0101 B0451 — Hochladen mit Fortschritt: je Datei ein `PUT` über XMLHttpRequest mit Sitzungs-Header, Balken
// (`progressbar` „<name> wird hochgeladen“, aria-valuenow 0–100) mit Prozent; höchstens 2 zugleich je Browser über alle
// Tabs (Web Locks), weitere zeigen „wartet“; 422 `metadaten` → ⚠ mit dem Text der Instanz und „Entfernen“; Fehlschlag →
// Grund und „Erneut hochladen“. Hochladen gedrosselt über CDP (Network.emulateNetworkConditions, 1 MiB/s).
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { fehlerSammeln } = require('./hilfen');
const {
  anhangStarten, anfragenMitschreiben, waehlen, berichtPdf, archivZip,
} = require('./r00030-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const MIB = 1024 * 1024;
const eintrag = (page) => page.getByRole('log').locator('.lc-hochladung');
const dateiImEintrag = (page, name) => eintrag(page).locator('.lc-hochladung-datei').filter({ hasText: name });

/// Drosselt das Hochladen der Seite auf `bytesJeSekunde`.
async function drosseln(page, bytesJeSekunde = MIB) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde,
  });
}

/// Schreibt jeden Wert von aria-valuenow der Balken im Verlauf mit.
async function balkenMitschreiben(page) {
  await page.evaluate(() => {
    window.balkenWerte = [];
    new MutationObserver((aenderungen) => {
      for (const aenderung of aenderungen) {
        const balken = [aenderung.target, ...aenderung.addedNodes]
          .filter((knoten) => knoten.nodeType === 1)
          .flatMap((knoten) => [knoten, ...knoten.querySelectorAll('[role="progressbar"]')])
          .filter((knoten) => knoten.getAttribute('role') === 'progressbar' && knoten.hasAttribute('aria-valuenow'));
        balken.forEach((knoten) => window.balkenWerte.push(Number(knoten.getAttribute('aria-valuenow'))));
      }
    }).observe(document.querySelector('[role="log"]'), { subtree: true, childList: true, attributes: true, attributeFilter: ['aria-valuenow'] });
  });
  return () => page.evaluate(() => window.balkenWerte);
}

test('Fortschritt (Desktop, 1 MiB/s): bericht.pdf → Balken „bericht.pdf wird hochgeladen“ steigt in mindestens zwei Schritten, danach kein Balken; PUT mit Sitzung', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const puts = [];
  page.on('request', (anfrage) => {
    if (anfrage.method() === 'PUT' && anfrage.url().includes('/api/anhaenge/')) {
      puts.push(anfrage.headers());
    }
  });
  await drosseln(page);
  const werte = await balkenMitschreiben(page);

  await waehlen(page, [berichtPdf()]);
  await eingabe(page).fill('Bericht Q3');
  await eingabe(page).press('Enter');

  const balken = page.getByRole('progressbar', { name: 'bericht.pdf wird hochgeladen' });
  await expect(balken).toBeVisible();
  await expect(balken).toHaveAttribute('aria-valuemin', '0');
  await expect(balken).toHaveAttribute('aria-valuemax', '100');
  await expect(eintrag(page)).toContainText('wird hochgeladen …');
  await expect(balken).toHaveCount(0, { timeout: 15000 });
  const gesehen = await werte();
  const dazwischen = [...new Set(gesehen)].filter((wert) => wert > 0 && wert < 100);
  expect(dazwischen.length, JSON.stringify(gesehen)).toBeGreaterThanOrEqual(2);
  expect(gesehen).toEqual([...gesehen].sort((a, b) => a - b));
  expect(puts).toHaveLength(1);
  expect(puts[0]['x-localchata-sitzung']).toMatch(/.+/);
  expect(puts[0]['content-type']).toBe('application/octet-stream');
  expect(fehler).toEqual([]);
});

/// Im Browser: je PUT die Zeit vom send bis zum Ende des Körpers (`upload` loadend), auf der gemeinsamen Uhr aller Tabs
/// (performance.timeOrigin + now). Die Ereignisse `request`/`requestfinished` in Playwright kommen versetzt an und
/// täuschen einen dritten PUT vor (Code-Review #43 F0101 A3).
function koerperZeitenMitschreiben() {
  window.koerperZeiten = [];
  const oeffnen = XMLHttpRequest.prototype.open;
  const senden = XMLHttpRequest.prototype.send;
  const jetzt = () => performance.timeOrigin + performance.now();
  XMLHttpRequest.prototype.open = function oeffnenMitMethode(methode, ...rest) {
    this.methodeR00030 = methode;
    return oeffnen.call(this, methode, ...rest);
  };
  XMLHttpRequest.prototype.send = function sendenMitZeit(...rest) {
    if (this.methodeR00030 === 'PUT') {
      const zeit = { beginn: jetzt(), ende: null };
      window.koerperZeiten.push(zeit);
      this.upload.addEventListener('loadend', () => {
        zeit.ende = jetzt();
      });
    }
    return senden.apply(this, rest);
  };
}

/// Höchstzahl gleichzeitig offener Spannen [beginn, ende).
function hoechstensZugleich(zeiten) {
  const ereignisse = zeiten.flatMap(({ beginn, ende }) => [[beginn, 1], [ende, -1]]).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let offen = 0;
  let hoechstens = 0;
  for (const [, schritt] of ereignisse) {
    offen += schritt;
    hoechstens = Math.max(hoechstens, offen);
  }
  return hoechstens;
}

test('Höchstens zwei zugleich (Desktop): zwei Tabs, je 3 Dateien gedrosselt → nie mehr als 2 PUT gleichzeitig, alle 6 kommen an, kein 429; die dritte zeigt „wartet“', async ({ browser }) => {
  instanz = await anhangStarten();
  const kontext = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await kontext.addInitScript(koerperZeitenMitschreiben);
  const eins = await kontext.newPage();
  const fehlerEins = fehlerSammeln(eins);
  await imBrowserBeitreten(eins, instanz.adresse, 'shoff');
  const zwei = await kontext.newPage();
  const fehlerZwei = fehlerSammeln(zwei);
  await zwei.goto(`${instanz.adresse}#/c/allgemein`);
  await expect(zwei.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');

  const antworten = [];
  for (const seite of [eins, zwei]) {
    await drosseln(seite);
    const ende = async (anfrage) => {
      if (anfrage.method() === 'PUT' && anfrage.url().includes('/api/anhaenge/')) {
        const antwort = await anfrage.response();
        antworten.push(antwort === null ? 0 : antwort.status());
      }
    };
    seite.on('requestfinished', ende);
    seite.on('requestfailed', ende);
  }
  const anmeldungen = [];
  kontext.on('response', (antwort) => {
    if (antwort.request().method() === 'POST' && new URL(antwort.url()).pathname === '/api/anhaenge') {
      anmeldungen.push(antwort.status());
    }
  });

  for (const [seite, kennung] of [[eins, 'a'], [zwei, 'b']]) {
    await waehlen(seite, [1, 2, 3].map((nr) => archivZip(`${kennung}${nr}.zip`, MIB)));
    await knopf(seite).click();
  }
  const wartende = async () => {
    const zahlen = await Promise.all([eins, zwei].map((seite) => eintrag(seite).locator('.lc-hochladung-zustand', { hasText: /^wartet$/ }).count()));
    return zahlen[0] + zahlen[1];
  };
  await expect.poll(wartende, { intervals: [25] }).toBeGreaterThan(0);

  await expect.poll(() => antworten.length, { timeout: 25000 }).toBe(6);
  expect(antworten).toEqual([200, 200, 200, 200, 200, 200]);
  const zeiten = (await Promise.all([eins, zwei].map((seite) => seite.evaluate(() => window.koerperZeiten)))).flat();
  expect(zeiten).toHaveLength(6);
  expect(hoechstensZugleich(zeiten)).toBe(2);
  expect(anmeldungen.every((status) => status === 201), JSON.stringify(anmeldungen)).toBe(true);
  expect(fehlerEins).toEqual([]);
  expect(fehlerZwei).toEqual([]);
});

test('Beschädigt (Desktop): ein JPEG, dessen EXIF-Block über das Ende zeigt → nach dem Hochladen ⚠ mit dem Text der 422 `metadaten`, „Entfernen“, nichts gesendet', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page, fehler } = await geraetOeffnen(browser, 'desktop', { zusaetzlich: { '/api/anhaenge/': [422] } });
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(page);
  const kaputt = Buffer.concat([Buffer.from([0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xF0]), Buffer.from('Exif\0\0GPS 52.5200')]);

  await waehlen(page, [{ name: 'kaputt.jpg', mimeType: 'image/jpeg', buffer: kaputt }]);
  await knopf(page).click();

  const datei = dateiImEintrag(page, 'kaputt.jpg');
  await expect(datei.locator('.lc-hochladung-grund'))
    .toHaveText('⚠ Nicht hochgeladen: die Datei ist beschädigt — Ortsangaben ließen sich nicht sicher entfernen.');
  const put = anfragen.find((anfrage) => anfrage.methode === 'PUT');
  expect(put).toBeDefined();
  await expect(datei.getByRole('button', { name: 'kaputt.jpg entfernen', exact: true })).toBeVisible();
  expect(anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten'))).toEqual([]);
  expect(fehler.filter((zeile) => !zeile.includes('/api/anhaenge/'))).toEqual([]);
});

test('Fehlschlag (Desktop): der PUT wird abgebrochen → ⚠ „Keine Antwort vom Server.“ und „bericht.pdf erneut hochladen“', async ({ browser }) => {
  instanz = await anhangStarten();
  const { page } = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(page, instanz.adresse, 'shoff');
  await page.route('**/api/anhaenge/*', (route) => {
    if (route.request().method() === 'PUT') {
      return route.abort();
    }
    return route.continue();
  });

  await waehlen(page, [berichtPdf()]);
  await knopf(page).click();

  const datei = dateiImEintrag(page, 'bericht.pdf');
  await expect(datei.locator('.lc-hochladung-grund')).toHaveText('⚠ Keine Antwort vom Server.');
  await expect(datei.getByRole('button', { name: 'bericht.pdf erneut hochladen', exact: true })).toHaveText('Erneut hochladen');
});
