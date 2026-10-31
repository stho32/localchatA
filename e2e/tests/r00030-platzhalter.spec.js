'use strict';
// R00030 F0184 / B0798 „Platzhalter bricht an der Wortgrenze“ — Abnahme-Probe wörtlich (ux-reviewer #85 Vorprüfung,
// korrigiert): Touch 320×568 und 375×667, Channel # allgemein und Direktgespräch beta@127.0.0.2, leeres Feld → die Breite des
// Platzhalters (Canvas mit der Schrift des Felds) ≤ clientWidth − Innenabstand, oder er endet mit „…“ und passt; er enthält
// den Channelnamen bzw. die ersten 4 Zeichen des Gegenübers; die Feldhöhe ist 40 px; das aria-label bleibt „Nachricht an # allgemein“;
// bei 768 und 1280 steht der volle Text. Der Platzhalter wird per Skript gekürzt (Nachricht an # allgemein → An # allgemein →
// # allgemein → Name mit …; im Direktgespräch ohne @-Teil), weil text-overflow an ::placeholder nicht wirkt.
const { test, expect } = require('./rahmen');
const { beitreten, imBrowserBeitreten } = require('./senden-hilfen');
const { direktStarten, tippenOhneGespraech, direktOeffnen } = require('./r00050-direkt-hilfen');
const { GERAETE } = require('./r00027-hilfen');

let instanz;
test.afterEach(async () => {
  if (instanz) {
    await instanz.stoppen();
    instanz = null;
  }
});

const GEGENUEBER = 'beta@127.0.0.2';
const FENSTER = [
  { name: 'Touch 320×568', geraet: { ...GERAETE.telefonSchmal }, schmal: true },
  { name: 'Touch 375×667', geraet: { ...GERAETE.telefon, viewport: { width: 375, height: 667 } }, schmal: true },
  { name: 'Tablet 768×1024', geraet: { ...GERAETE.tablet }, schmal: false },
  { name: 'Desktop 1280×800', geraet: { ...GERAETE.desktop }, schmal: false },
];
const ORTE = [
  { name: 'Channel # allgemein', label: 'Nachricht an # allgemein', voll: 'Nachricht an # allgemein', merkmal: '# allgemein' },
  { name: `Direktgespräch ${GEGENUEBER}`, label: `Nachricht an ${GEGENUEBER}`, voll: `Nachricht an ${GEGENUEBER}`, merkmal: 'beta' },
];

/// Platzhalter, Schrift und nutzbare Breite des Felds; die Breite des Textes misst ein Canvas mit derselben Schrift.
const vermessen = (feld) => feld.evaluate((element) => {
  const stil = getComputedStyle(element);
  const zeichner = document.createElement('canvas').getContext('2d');
  zeichner.font = `${stil.fontStyle} ${stil.fontWeight} ${stil.fontSize} ${stil.fontFamily}`;
  const innen = parseFloat(stil.paddingLeft) + parseFloat(stil.paddingRight);
  return {
    platzhalter: element.placeholder,
    breite: zeichner.measureText(element.placeholder).width,
    verfuegbar: element.clientWidth - innen,
    hoehe: element.getBoundingClientRect().height,
    label: element.getAttribute('aria-label'),
    wert: element.value,
  };
});

for (const fenster of FENSTER) {
  for (const ort of ORTE) {
    test(`Platzhalter (${fenster.name}, ${ort.name}): ${fenster.schmal ? 'passt oder endet auf „…“ und passt' : 'voller Text'}, Höhe 40 px, Name bleibt`, async ({ browser }) => {
      const gestartet = await direktStarten(browser, { geraet: fenster.geraet, sammeln: { zusaetzlich: tippenOhneGespraech(GEGENUEBER) } });
      instanz = gestartet.instanz;
      const { page, fehler } = gestartet.anna;
      await beitreten(instanz.adresse, 'beta', { art: 'mensch', herkunft: '127.0.0.2' });
      if (ort.merkmal === 'beta') {
        await direktOeffnen(page, GEGENUEBER);
      }
      const feld = page.getByRole('textbox', { name: ort.label, exact: true });
      await expect(feld).toBeVisible();
      await expect(feld).toHaveValue('');
      await page.waitForTimeout(300);

      const mass = await vermessen(feld);

      expect(mass.label, 'aria-label unverändert').toBe(ort.label);
      expect(mass.hoehe, 'Feldhöhe').toBe(40);
      expect(mass.platzhalter, 'Name im Platzhalter').toContain(ort.merkmal.slice(0, 4));
      if (fenster.schmal) {
        expect(mass.breite, `Platzhalter „${mass.platzhalter}“ passt`).toBeLessThanOrEqual(mass.verfuegbar);
      } else {
        expect(mass.platzhalter, 'voller Text').toBe(ort.voll);
      }
      expect(fehler).toEqual([]);
    });
  }
}
