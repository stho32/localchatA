'use strict';
// R00030 F0101 B0453 — nach dem Hochladen senden: sind alle Dateien oben, tritt die Nachricht mit `{text, anhaenge}` in
// die Sendeschlange (ADR-0037) und erscheint bei A und B genau einmal; einmal je Nachricht angesagt; Text, der während
// des Hochladens gesendet wird, geht vor (N1); ein Channelwechsel unterbricht nichts; Enter bzw. Strg Enter senden auch
// nur mit Dateien; eine Antwort trägt Bezug und Dateien zusammen.
const { test, expect } = require('./rahmen');
const { imBrowserBeitreten, eingabe, nachrichten } = require('./senden-hilfen');
const { geraetOeffnen, knopf } = require('./r00027-hilfen');
const { annaSchreibt } = require('./r00047-hilfen');
const { nachrichtMitZeit } = require('./r00041-hilfen');
const {
  anhangStarten, perApi, anfragenMitschreiben, ansagenMitschreiben, dateizeilen, waehlen, fotoPng, berichtPdf, textDatei,
  archivZip,
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
const bestaetigteVon = (page, von) => page.getByRole('log').locator(`.lc-msg:not(.lc-vorlaeufig)[data-von="${von}"]`);
const gesendet = (anfragen) => anfragen.filter((anfrage) => anfrage.pfad.endsWith('/nachrichten')).map((anfrage) => anfrage.koerper);

async function drosseln(page, bytesJeSekunde) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: bytesJeSekunde });
}

/// A (`shoff`) auf `geraet`, B (`anna`) am Desktop, beide in # allgemein.
async function zweiOeffnen(browser, geraet) {
  const a = await geraetOeffnen(browser, geraet);
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const b = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(b.page, instanz.adresse, 'anna');
  return { a, b };
}

test('Wählen und senden auf dem Telefon: ein POST /api/anhaenge, ein PUT, ein POST …/nachrichten mit {"text":"","anhaenge":[id]}; bei A und B genau einmal', async ({ browser }) => {
  instanz = await anhangStarten();
  const { a, b } = await zweiOeffnen(browser, 'telefon');
  const anfragen = anfragenMitschreiben(a.page);

  await waehlen(a.page, [fotoPng()], { geste: 'tippen' });
  await expect(dateizeilen(a.page).first()).toContainText('300 KiB');
  await knopf(a.page).tap();

  for (const seite of [a.page, b.page]) {
    await expect(bestaetigteVon(seite, 'shoff@127.0.0.1')).toHaveCount(1);
  }
  await expect(eintrag(a.page)).toHaveCount(0);
  const anmeldungen = anfragen.filter((anfrage) => anfrage.methode === 'POST' && anfrage.pfad === '/api/anhaenge');
  const puts = anfragen.filter((anfrage) => anfrage.methode === 'PUT');
  expect(anmeldungen).toHaveLength(1);
  expect(puts).toHaveLength(1);
  const id = puts[0].pfad.split('/').at(-1);
  expect(gesendet(anfragen)).toEqual([{ text: '', anhaenge: [id] }]);
  await expect(eingabe(a.page)).toHaveValue('');
  await expect(dateizeilen(a.page)).toHaveCount(0);
  await expect(eingabe(a.page)).toBeFocused();
  expect(a.fehler).toEqual([]);
  expect(b.fehler).toEqual([]);
});

test('Tablet: bericht.pdf mit `Bericht Q3` → die Nachricht trägt Text und Anhang', async ({ browser }) => {
  instanz = await anhangStarten();
  const { a, b } = await zweiOeffnen(browser, 'tablet');
  const anfragen = anfragenMitschreiben(a.page);

  await waehlen(a.page, [berichtPdf()], { geste: 'tippen' });
  await eingabe(a.page).fill('Bericht Q3');
  await knopf(a.page).tap();

  for (const seite of [a.page, b.page]) {
    await expect(bestaetigteVon(seite, 'shoff@127.0.0.1').locator('.lc-body')).toHaveText('Bericht Q3');
  }
  const koerper = gesendet(anfragen);
  expect(koerper).toHaveLength(1);
  expect(koerper[0].text).toBe('Bericht Q3');
  expect(koerper[0].anhaenge).toHaveLength(1);
});

test('Desktop: log.txt und archiv.zip, `zwei Dateien`, Enter → gesendet mit beiden Anhängen in Wahlreihenfolge; einmal angesagt', async ({ browser }) => {
  instanz = await anhangStarten();
  const a = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(a.page);
  const namen = new Map();
  a.page.on('response', async (antwort) => {
    if (antwort.request().method() === 'POST' && new URL(antwort.url()).pathname === '/api/anhaenge') {
      const { anhang } = await antwort.json();
      namen.set(anhang.id, anhang.name);
    }
  });
  const angesagt = await ansagenMitschreiben(a.page);

  await waehlen(a.page, [textDatei(), archivZip()]);
  await eingabe(a.page).fill('zwei Dateien');
  await eingabe(a.page).press('Enter');

  await expect(bestaetigteVon(a.page, 'shoff@127.0.0.1')).toHaveCount(1);
  const koerper = gesendet(anfragen);
  expect(koerper).toHaveLength(1);
  expect(koerper[0].text).toBe('zwei Dateien');
  expect(koerper[0].anhaenge.map((id) => namen.get(id))).toEqual(['log.txt', 'archiv.zip']);
  expect((await angesagt()).filter((text) => text.includes('hochgeladen'))).toEqual(['2 Dateien hochgeladen, Nachricht wird gesendet.']);
});

for (const [geraet, taste] of [['desktop', 'Enter'], ['tablet', 'Control+Enter']]) {
  test(`Taste ohne Text (${geraet}): Feld leer, log.txt gewählt, ${taste} → genau ein Absenden mit {"text":"","anhaenge":[…]}`, async ({ browser }) => {
    instanz = await anhangStarten();
    const a = await geraetOeffnen(browser, geraet);
    await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
    const anfragen = anfragenMitschreiben(a.page);

    await waehlen(a.page, [textDatei()], { geste: geraet === 'desktop' ? 'klick' : 'tippen' });
    await eingabe(a.page).focus();
    await eingabe(a.page).press(taste);

    await expect(bestaetigteVon(a.page, 'shoff@127.0.0.1')).toHaveCount(1);
    const koerper = gesendet(anfragen);
    expect(koerper).toHaveLength(1);
    expect(koerper[0].text).toBe('');
    expect(koerper[0].anhaenge).toHaveLength(1);
  });
}

test('Text geht vor (Desktop, gedrosselt, N1): während film.webm (8 MiB) lädt, steht `zwischendurch` ≤ 1 s nach Enter bestätigt bei A und B; `Video` erscheint danach', async ({ browser }) => {
  test.setTimeout(60000);
  instanz = await anhangStarten();
  const { a, b } = await zweiOeffnen(browser, 'desktop');
  await drosseln(a.page, 2 * MIB);

  await waehlen(a.page, [archivZip('film.webm', 8 * MIB)]);
  await eingabe(a.page).fill('Video');
  await eingabe(a.page).press('Enter');
  await expect(a.page.getByRole('progressbar', { name: 'film.webm wird hochgeladen' })).toBeVisible();
  await eingabe(a.page).fill('zwischendurch');
  const warten = [a.page, b.page].map((seite) => seite.waitForFunction(() => [...document.querySelectorAll('[role="log"] .lc-msg:not(.lc-vorlaeufig)')]
    .some((knoten) => knoten.querySelector('.lc-body') !== null && knoten.querySelector('.lc-body').textContent === 'zwischendurch'), null, { polling: 'raf', timeout: 10000 }));
  const gedrueckt = Date.now();
  await eingabe(a.page).press('Enter');
  await Promise.all(warten);
  expect(Date.now() - gedrueckt).toBeLessThanOrEqual(1000);
  await expect(eintrag(a.page)).toHaveCount(1);

  for (const seite of [a.page, b.page]) {
    const video = seite.getByRole('log').locator('.lc-msg:not(.lc-vorlaeufig)').filter({ has: seite.locator('.lc-body', { hasText: /^Video$/ }) });
    await expect(video).toHaveCount(1, { timeout: 20000 });
    const texte = await nachrichten(seite).locator('.lc-body').allTextContents();
    expect(texte.indexOf('Video')).toBeGreaterThan(texte.indexOf('zwischendurch'));
  }
});

test('Channelwechsel während des Hochladens (Desktop, gedrosselt): das Hochladen läuft weiter, die Nachricht kommt in # allgemein an; zurück steht der Eintrag bzw. die Nachricht', async ({ browser }) => {
  instanz = await anhangStarten();
  const a = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  await drosseln(a.page, MIB);

  await waehlen(a.page, [berichtPdf()]);
  await eingabe(a.page).fill('Bericht');
  await eingabe(a.page).press('Enter');
  await expect(eintrag(a.page)).toHaveCount(1);
  await a.page.getByRole('link', { name: 'technik' }).click();
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # technik');
  await expect(eintrag(a.page)).toHaveCount(0);
  await a.page.getByRole('link', { name: 'allgemein' }).click();
  await expect(a.page.getByRole('log')).toHaveAttribute('aria-label', 'Verlauf # allgemein');
  const eintragOderNachricht = eintrag(a.page).or(bestaetigteVon(a.page, 'shoff@127.0.0.1'));
  await expect(eintragOderNachricht.first()).toBeVisible();
  await a.page.getByRole('link', { name: 'technik' }).click();

  const token = await a.page.evaluate(() => localStorage.getItem('localchatA.sitzung'));
  await expect.poll(async () => {
    const antwort = await perApi(instanz.adresse, { methode: 'GET', pfad: 'api/channels/allgemein/nachrichten?n=50', token, herkunft: '127.0.0.1' });
    return antwort.daten.nachrichten.filter((nachricht) => nachricht.text === 'Bericht').map((nachricht) => (nachricht.anhaenge || []).length);
  }, { timeout: 15000 }).toEqual([1]);
  await a.page.getByRole('link', { name: 'allgemein' }).click();
  await expect(bestaetigteVon(a.page, 'shoff@127.0.0.1').locator('.lc-body')).toHaveText('Bericht');
  await expect(eintrag(a.page)).toHaveCount(0);
});

test('Antwort mit Dateien (Desktop, gedrosselt): an `Hallo` von B, log.txt, `siehe Log` → genau ein POST mit antwort_auf und einer ID; Zitat und Text', async ({ browser }) => {
  instanz = await anhangStarten();
  const anna = await annaSchreibt(instanz.adresse, ['Hallo']);
  const a = await geraetOeffnen(browser, 'desktop');
  await imBrowserBeitreten(a.page, instanz.adresse, 'shoff');
  const anfragen = anfragenMitschreiben(a.page);
  await drosseln(a.page, 20 * 1024);

  const hallo = nachrichtMitZeit(a.page, anna.zeiten[0]);
  await hallo.hover();
  await hallo.getByRole('button', { name: 'Antworten', exact: true }).click();
  await waehlen(a.page, [textDatei()]);
  const antwortzeile = await a.page.locator('.lc-antwortzeile').boundingBox();
  const liste = await a.page.locator('.lc-dateien').boundingBox();
  expect(antwortzeile.y + antwortzeile.height).toBeLessThanOrEqual(liste.y + 0.5);
  await eingabe(a.page).fill('siehe Log');
  await eingabe(a.page).press('Enter');
  await expect(eintrag(a.page).locator('.lc-zitat'), 'Sollte S5: der Eintrag zeigt die Zitatzeile').toHaveCount(1);

  const antwort = bestaetigteVon(a.page, 'shoff@127.0.0.1');
  await expect(antwort.locator('.lc-body')).toHaveText('siehe Log');
  await expect(antwort.locator('.lc-zitat')).toHaveCount(1);
  const koerper = gesendet(anfragen);
  expect(koerper).toHaveLength(1);
  expect(Date.parse(koerper[0].antwort_auf)).toBe(Date.parse(anna.zeiten[0]));
  expect(koerper[0].anhaenge).toHaveLength(1);
});
