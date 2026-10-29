'use strict';
// R00019 F0065 Channel-Name auf dem Schalter lesbar (US-5, US-5b, US-5c; UX-Vorprüfung #27 M1, M2; ADR-0027 und
// ADR-0033 je Ergänzung Karte #27): die Kopfzeile kürzt zuerst Beschreibung und Instanzname, unter 480 px steht sie in
// zwei Zeilen von zusammen 80 px, mit grobem Zeiger ist `# name ▾` ein Ziel von 44 px.
const fs = require('fs');
const path = require('path');
const { test, expect } = require('./rahmen');
const { instanzStarten, fehlerSammeln } = require('./hilfen');

const NAME_13 = 'abcdefghijklm';
const NAME_32 = 'abcdefghijklmnopqrstuvwxyz012345';
const BESCHREIBUNG = 'Absprachen zwischen Entwicklung, Test und Betrieb: wer welchen Zweig gerade baut, welche Probe noch fehlt, '
  + 'welcher Dienst neu gestartet werden muss und wer die nächste Abnahme fährt — alles hier drin.';
const LANGER_NAME = 'm'.repeat(32);

/// Geräte aus R00019 „Nicht-funktionale Anforderungen“: Telefon und Tablet mit Touch (grober Zeiger), Desktop ohne.
const TELEFONE = [[320, 640], [375, 667], [414, 896]];
const TABLETS = [[768, 1024], [1024, 768]];
const DESKTOPS = [[480, 800], [720, 800], [1280, 800]];
const mitTouch = ([breite, hoehe]) => ({ viewport: { width: breite, height: hoehe }, hasTouch: true, isMobile: true });
const ohneTouch = ([breite, hoehe]) => ({ viewport: { width: breite, height: hoehe } });

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

test.beforeAll(() => {
  expect(BESCHREIBUNG).toHaveLength(200);
});

function channelMitEinstellungen(ordner, name, inhalt) {
  const pfad = path.join(ordner, 'channels', name);
  fs.mkdirSync(pfad, { recursive: true });
  fs.writeFileSync(path.join(pfad, 'einstellungen.toml'), inhalt);
}

async function geoeffnet(browser, kontext, { channel = 'entwicklung', name = NAME_13 } = {}) {
  instanz = await instanzStarten({
    vorbereiten: (ordner) => {
      channelMitEinstellungen(ordner, 'entwicklung', `beschreibung = "${BESCHREIBUNG}"\n`);
      fs.mkdirSync(path.join(ordner, 'channels', LANGER_NAME), { recursive: true });
    },
  });
  const neu = await browser.newContext(kontext);
  const page = await neu.newPage();
  const fehler = fehlerSammeln(page);
  await page.goto(`${instanz.adresse}#/c/${channel}`);
  await page.getByLabel('Name').fill(name);
  await page.getByLabel('Name').press('Enter');
  await expect(page.getByRole('banner').locator('.lc-me')).toHaveText(`${name}@127.0.0.1`);
  await expect(page.getByRole('banner').locator('.lc-head-chan')).toHaveText(`# ${channel}`);
  return { page, fehler };
}

const schalter = (page, channel = 'entwicklung') => page.getByRole('banner').getByRole('button', { name: `# ${channel} — Channel-Einstellungen` });

/// Maße der Kopfzeile im Fenster: Name, Schalter, Zone „Ort“, Beschreibung, Instanzname, Kennung, „Teilnehmer“, und
/// welche Elemente `elementFromPoint` 2 px innerhalb der Ecken und senkrecht mittig links/rechts im Schalter findet.
function vermessen() {
  const kopf = document.querySelector('.lc-head');
  const knopf = kopf.querySelector('.lc-head-schalter');
  const name = knopf.querySelector('.lc-head-chan');
  const pfeil = knopf.querySelector('.lc-head-pfeil');
  const ort = kopf.querySelector('.lc-head-ort');
  const desc = kopf.querySelector('.lc-head-desc');
  const inst = kopf.querySelector('.lc-head-inst');
  const ich = kopf.querySelector('.lc-me');
  const teilnehmer = kopf.querySelector('.lc-hbtn');
  const r = (element) => element.getBoundingClientRect().toJSON();
  const k = r(knopf);
  const trifft = (x, y) => knopf.contains(document.elementFromPoint(x, y));
  const mitte = k.top + k.height / 2;
  let beschreibung = null;
  if (desc !== null) {
    beschreibung = { sichtbar: desc.checkVisibility(), breite: desc.clientWidth, noetig: desc.scrollWidth };
  }
  const kopfStil = getComputedStyle(kopf);
  return {
    fenster: { breite: window.innerWidth, hoehe: window.innerHeight },
    seite: document.documentElement.scrollWidth,
    kopf: { ...r(kopf), innenLinks: r(kopf).left + parseFloat(kopfStil.paddingLeft), innenRechts: r(kopf).right - parseFloat(kopfStil.paddingRight) },
    ort: r(ort),
    schalter: k,
    name: { noetig: name.scrollWidth, breite: name.clientWidth },
    pfeil: r(pfeil),
    trifftLinksRechts: trifft(k.left + 2, mitte) && trifft(k.right - 2, mitte),
    trifftEcken: trifft(k.left + 2, k.top + 2) && trifft(k.right - 2, k.top + 2) && trifft(k.left + 2, k.bottom - 2) && trifft(k.right - 2, k.bottom - 2),
    beschreibung,
    instSichtbar: inst.checkVisibility(),
    ich: { ...r(ich), text: ich.textContent },
    teilnehmer: { ...r(teilnehmer), sichtbar: teilnehmer.checkVisibility() },
  };
}

const ganzIn = (innen, aussen) => innen.left >= aussen.left - 0.5 && innen.right <= aussen.right + 0.5
  && innen.top >= aussen.top - 0.5 && innen.bottom <= aussen.bottom + 0.5 && innen.width > 0;
const imFenster = (rand, fenster) => ganzIn(rand, { left: 0, top: 0, right: fenster.breite, bottom: fenster.hoehe });

// Kriterium „Breiten“ (UX-Vorprüfung #27 M1, wörtlich): Name ungekürzt, Schalter ganz in der Zone und im Fenster,
// elementFromPoint links/rechts trifft ihn, ab 720 px Beschreibung sichtbar und gekürzt, Seite ohne Querscrollen.
function breitenPruefen(mass) {
  expect(mass.name.noetig, 'Name ungekürzt').toBeLessThanOrEqual(mass.name.breite);
  expect(ganzIn(mass.schalter, mass.ort), 'Schalter ganz in .lc-head-ort').toBe(true);
  expect(imFenster(mass.schalter, mass.fenster), 'Schalter im Fenster').toBe(true);
  expect(mass.trifftLinksRechts, 'elementFromPoint trifft den Schalter').toBe(true);
  expect(mass.seite, 'ohne Querscrollen').toBeLessThanOrEqual(mass.fenster.breite);
  if (mass.fenster.breite >= 720) {
    expect(mass.beschreibung.sichtbar, 'Beschreibung sichtbar').toBe(true);
    expect(mass.beschreibung.breite, 'Beschreibung mindestens 40 px').toBeGreaterThanOrEqual(40);
    expect(mass.beschreibung.noetig, 'Beschreibung gekürzt').toBeGreaterThan(mass.beschreibung.breite);
  }
}

for (const [geraet, kontext] of [
  ...DESKTOPS.map((mass) => [`${mass[0]} × ${mass[1]}`, ohneTouch(mass)]),
  ...TABLETS.map((mass) => [`Tablet ${mass[0]} × ${mass[1]}`, mitTouch(mass)]),
]) {
  test(`Breiten (B0323, ${geraet}): # entwicklung ▾ ungekürzt, ganz sichtbar und treffbar, die Beschreibung gibt nach`, async ({ browser }) => {
    const a = await geoeffnet(browser, kontext);

    breitenPruefen(await a.page.evaluate(vermessen));
    expect(a.fehler).toEqual([]);
  });
}

for (const breite of [480]) {
  test(`Langer Name (B0323, ${breite} px): 32 × m endet sichtbar auf …, ▾ und Schalter ganz im Fenster, zugänglicher Name ganz`, async ({ browser }) => {
    const a = await geoeffnet(browser, ohneTouch([breite, 800]), { channel: LANGER_NAME });

    const mass = await a.page.evaluate(vermessen);

    expect(mass.name.noetig, 'Name gekürzt').toBeGreaterThan(mass.name.breite);
    expect(imFenster(mass.pfeil, mass.fenster), '▾ im Fenster').toBe(true);
    expect(imFenster(mass.schalter, mass.fenster), 'Schalter im Fenster').toBe(true);
    expect(ganzIn(mass.schalter, mass.ort), 'Schalter ganz in .lc-head-ort').toBe(true);
    expect(mass.trifftLinksRechts, 'elementFromPoint trifft den Schalter').toBe(true);
    await expect(schalter(a.page, LANGER_NAME)).toHaveAccessibleName(`# ${LANGER_NAME} — Channel-Einstellungen`);
    expect(mass.seite, 'ohne Querscrollen').toBeLessThanOrEqual(mass.fenster.breite);
    expect(a.fehler).toEqual([]);
  });
}

// Kriterium „Kopfzeile bleibt flach“ (UX-Vorprüfung #27 M2, Betreiber F1, US-5b): unter 480 px zwei Zeilen — oben die
// Zone „Ort“ über die Innenbreite ohne Instanzname, darunter Kennung und „Teilnehmer“ —, zusammen 80 px.
function zweizeiligPruefen(mass, kennung) {
  expect(mass.kopf.height, 'Kopfzeile 80 px').toBe(80);
  expect(mass.instSichtbar, 'Instanzname nicht sichtbar').toBe(false);
  expect(Math.abs(mass.ort.left - mass.kopf.innenLinks), 'Zone links an der Innenkante').toBeLessThanOrEqual(1);
  expect(Math.abs(mass.ort.right - mass.kopf.innenRechts), 'Zone rechts an der Innenkante').toBeLessThanOrEqual(1);
  expect(mass.ich.top, 'Kennung unter der Zone').toBeGreaterThanOrEqual(mass.ort.bottom);
  expect(mass.teilnehmer.top, '„Teilnehmer“ unter der Zone').toBeGreaterThanOrEqual(mass.ort.bottom);
  expect(mass.ich.text, 'Kennung ganz').toBe(kennung);
  expect(imFenster(mass.ich, mass.fenster), 'Kennung im Fenster').toBe(true);
  expect(mass.teilnehmer.sichtbar, '„Teilnehmer“ sichtbar').toBe(true);
}

async function teilnehmerNachDemSchalter(page) {
  await schalter(page).focus();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('banner').getByRole('button', { name: /^Teilnehmer/ })).toBeFocused();
}

for (const telefon of TELEFONE) {
  for (const name of [NAME_13, NAME_32]) {
    test(`Kopfzeile bleibt flach (B0540, Telefon ${telefon[0]} × ${telefon[1]}, Name ${name.length} Zeichen): zwei Zeilen, 80 px, Name ungekürzt und treffbar`, async ({ browser }) => {
      const a = await geoeffnet(browser, mitTouch(telefon), { name });

      const mass = await a.page.evaluate(vermessen);

      breitenPruefen(mass);
      zweizeiligPruefen(mass, `${name}@127.0.0.1`);
      await teilnehmerNachDemSchalter(a.page);
      expect(a.fehler).toEqual([]);
    });
  }
}

for (const desktop of [[480, 800], [1280, 800]]) {
  test(`Kopfzeile bleibt flach, Kontrollpaar ${desktop[0]} × ${desktop[1]} (B0540): einzeilig, 40 px, Instanzname sichtbar`, async ({ browser }) => {
    const a = await geoeffnet(browser, ohneTouch(desktop));

    const mass = await a.page.evaluate(vermessen);

    expect(mass.kopf.height).toBe(40);
    expect(mass.instSichtbar).toBe(true);
    expect(mass.ich.top, 'eine Zeile: Kennung neben der Zone').toBeLessThan(mass.ort.bottom);
    expect(a.fehler).toEqual([]);
  });
}

test('Langer Name (B0540, 320 px): 32 × m endet in Zeile 1 sichtbar auf …, ▾ und Schalter ganz im Fenster, zugänglicher Name ganz', async ({ browser }) => {
  const a = await geoeffnet(browser, mitTouch([320, 640]), { channel: LANGER_NAME });

  const mass = await a.page.evaluate(vermessen);

  expect(mass.name.noetig, 'Name gekürzt').toBeGreaterThan(mass.name.breite);
  expect(imFenster(mass.pfeil, mass.fenster), '▾ im Fenster').toBe(true);
  expect(ganzIn(mass.schalter, mass.ort), 'Schalter ganz in .lc-head-ort').toBe(true);
  expect(mass.trifftLinksRechts, 'elementFromPoint trifft den Schalter').toBe(true);
  await expect(schalter(a.page, LANGER_NAME)).toHaveAccessibleName(`# ${LANGER_NAME} — Channel-Einstellungen`);
  expect(mass.seite, 'ohne Querscrollen').toBeLessThanOrEqual(mass.fenster.breite);
  expect(a.fehler).toEqual([]);
});

// Frage F2 (product-owner 2026-10-29): die Kennung gewinnt — braucht sie in Zeile 2 mehr als zwei Zeilen, wächst die
// Kopfzeile über 80 px. Eine 48-Zeichen-Kennung mit IPv4 ist über 127.0.0.1 nicht herstellbar: die Kopfzeile wird für
// diese Kennung neu gezeichnet (wie r00009-kopfzeile-schmal.spec.js).
for (const [art, herkunft, laenge] of [['IPv4', '192.168.100.200', 48], ['IPv6, Agent', 'ffff:ffff:ffff:ffff:ffff:ffff:255.255.255.255', 78]]) {
  test(`Kennung gewinnt (B0540, F2, 320 px, ${laenge} Zeichen ${art}): ganz, ohne Querscrollen, Zone oben 44 px`, async ({ browser }) => {
    const a = await geoeffnet(browser, mitTouch([320, 640]), { name: NAME_32 });
    await a.page.evaluate(async ({ name, herkunft, art: artDerKennung }) => {
      const { zeichnen } = await import('/web/kopfzeile.js');
      const echteKopfzeile = document.querySelector('#app > .lc-head');
      const behaelter = document.createElement('div');
      zeichnen(behaelter, { name, herkunft, art: artDerKennung }, { schalter: [echteKopfzeile.querySelector('.lc-hbtn')] });
      echteKopfzeile.replaceWith(behaelter.firstElementChild);
    }, { name: NAME_32, herkunft, art: art.includes('Agent') ? 'agent' : 'mensch' });
    expect(`${NAME_32}@${herkunft}`).toHaveLength(laenge);

    const mass = await a.page.evaluate(() => {
      const kopf = document.querySelector('.lc-head').getBoundingClientRect();
      const ort = document.querySelector('.lc-head-ort').getBoundingClientRect();
      const ich = document.querySelector('.lc-me');
      return { kopf: kopf.height, ortUnten: ort.bottom - kopf.top, ich: ich.textContent, noetig: ich.scrollWidth, breite: ich.clientWidth, seite: document.documentElement.scrollWidth };
    });

    expect(mass.ich).toBe(`${NAME_32}@${herkunft}`);
    expect(mass.noetig).toBeLessThanOrEqual(mass.breite);
    expect(mass.seite).toBeLessThanOrEqual(320);
    expect(mass.kopf).toBeGreaterThanOrEqual(80);
    expect(mass.ortUnten).toBeLessThanOrEqual(44);
    expect(a.fehler).toEqual([]);
  });
}

// Kriterium „Touch-Ziel“ (UX-Vorprüfung #27 Sollte, ADR-0033 Ergänzung Karte #27): mit grobem Zeiger ist der Schalter
// mindestens 44 × 44 px, liegt ganz in der Kopfzeile und ist an allen vier Ecken zu treffen.
for (const geraet of [...TELEFONE, ...TABLETS]) {
  test(`Touch-Ziel (B0541, ${geraet[0]} × ${geraet[1]} mit Touch): # entwicklung ▾ mindestens 44 × 44 px, ganz in der Kopfzeile, Ecken treffen`, async ({ browser }) => {
    const a = await geoeffnet(browser, mitTouch(geraet));

    const mass = await a.page.evaluate(vermessen);

    expect(mass.schalter.width).toBeGreaterThanOrEqual(44);
    expect(mass.schalter.height).toBeGreaterThanOrEqual(44);
    expect(ganzIn(mass.schalter, mass.kopf), 'Schalter ganz in .lc-head').toBe(true);
    expect(mass.trifftEcken, 'elementFromPoint an den vier Ecken').toBe(true);
    if (geraet[0] < 480) {
      expect(mass.kopf.height).toBe(80);
    }
    expect(a.fehler).toEqual([]);
  });
}

test('Touch-Ziel, Kontrollpaar 1280 × 800 ohne Touch (B0541): Schalter 28 px hoch, Kopfzeile 40 px', async ({ browser }) => {
  const a = await geoeffnet(browser, ohneTouch([1280, 800]));

  const mass = await a.page.evaluate(vermessen);

  expect(mass.schalter.height).toBe(28);
  expect(mass.kopf.height).toBe(40);
  expect(a.fehler).toEqual([]);
});
