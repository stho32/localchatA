// Unit-Tests des Fensters im Verlauf (R00007 F0031 B0162, B0163; R00013 B0257 L3b; ADR-0026 Regeln 1, 2 mit
// Ergänzung Abwurfschutz; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachladen, einfuegen } from '../../web/fenster.js';

const SICHT = 800;
const lage = (werte) => ({
  bloecke: 1, aelter: false, neuer: false, abstandOben: 5000, abstandUnten: 5000, sicht: SICHT, laeuft: false, ...werte,
});

test('oben: unter 2 Sichthöhen mit älteren lädt älter, ab 2 Sichthöhen oder ohne ältere nicht', () => {
  assert.equal(nachladen(lage({ abstandOben: 1599, aelter: true })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 1600, aelter: true })), null);
  assert.equal(nachladen(lage({ abstandOben: 1599, aelter: false })), null);
});

test('unten: unter 2 Sichthöhen mit neueren lädt neuer, ab 2 Sichthöhen oder ohne neuere nicht', () => {
  assert.equal(nachladen(lage({ abstandUnten: 1599, neuer: true })), 'neuer');
  assert.equal(nachladen(lage({ abstandUnten: 1600, neuer: true })), null);
  assert.equal(nachladen(lage({ abstandUnten: 1599, neuer: false })), null);
});

test('die Schwelle folgt der Sichthöhe', () => {
  assert.equal(nachladen(lage({ abstandOben: 799, aelter: true, sicht: 400 })), 'aelter');
  assert.equal(nachladen(lage({ abstandOben: 800, aelter: true, sicht: 400 })), null);
});

test('läuft ein Abruf oder ist nichts geladen, wird nicht nachgeladen', () => {
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true, laeuft: true })), null);
  assert.equal(nachladen(lage({ abstandUnten: 0, neuer: true, laeuft: true })), null);
  assert.equal(nachladen(lage({ abstandOben: 0, aelter: true, bloecke: 0 })), null);
});

const block = (name, { aelter = true, neuer = true } = {}) => ({ nachrichten: [{ zeit: name }], aelter, neuer });
const fensterAus = (anzahl) => ({
  bloecke: Array.from({ length: anzahl }, (_, index) => [{ zeit: `b${index}` }]), aelter: true, neuer: false,
});
const ersteZeiten = (fenster) => fenster.bloecke.map((bloecke) => bloecke[0].zeit);
// Maße, bei denen das andere Ende weit weg ist — abwerfen erlaubt (wie vor dem Abwurfschutz).
const WEIT = { abstandOben: 100000, abstandUnten: 100000, hoeheOben: 1000, hoeheUnten: 1000, sicht: SICHT };

test('fünf Blöcke und einer oben: der unterste fällt weg, danach gibt es neuere', () => {
  const ergebnis = einfuegen(fensterAus(5), block('neu', { aelter: false }), 'aelter', WEIT);

  assert.deepEqual(ersteZeiten(ergebnis.fenster), ['neu', 'b0', 'b1', 'b2', 'b3']);
  assert.deepEqual(ergebnis.abgeworfen, [{ zeit: 'b4' }]);
  assert.equal(ergebnis.fenster.aelter, false);
  assert.equal(ergebnis.fenster.neuer, true);
});

test('fünf Blöcke und einer unten: der oberste fällt weg, davor gibt es ältere', () => {
  const fenster = { ...fensterAus(5), aelter: false, neuer: true };

  const ergebnis = einfuegen(fenster, block('neu', { neuer: false }), 'neuer', WEIT);

  assert.deepEqual(ersteZeiten(ergebnis.fenster), ['b1', 'b2', 'b3', 'b4', 'neu']);
  assert.deepEqual(ergebnis.abgeworfen, [{ zeit: 'b0' }]);
  assert.equal(ergebnis.fenster.aelter, true);
  assert.equal(ergebnis.fenster.neuer, false);
});

test('vier Blöcke und einer: nichts fällt weg (Spanne: höchstens fünf)', () => {
  const oben = einfuegen(fensterAus(4), block('neu', { aelter: false }), 'aelter', WEIT);
  const unten = einfuegen({ ...fensterAus(4), neuer: true }, block('neu', { neuer: false }), 'neuer', WEIT);

  assert.equal(oben.fenster.bloecke.length, 5);
  assert.equal(oben.abgeworfen, null);
  assert.equal(oben.fenster.neuer, false);
  assert.equal(unten.fenster.bloecke.length, 5);
  assert.equal(unten.abgeworfen, null);
  assert.equal(unten.fenster.aelter, true);
});

test('ein leerer Block ändert nur die Angaben, keinen Block', () => {
  const ergebnis = einfuegen(fensterAus(2), { nachrichten: [], aelter: false, neuer: true }, 'aelter', WEIT);

  assert.equal(ergebnis.fenster.bloecke.length, 2);
  assert.equal(ergebnis.fenster.aelter, false);
  assert.equal(ergebnis.abgeworfen, null);
});

// L3b (R00013 K8): über 5 Blöcken wird am anderen Ende nur abgeworfen, wenn der Blick danach noch mindestens 2
// Sichthöhen von diesem Ende entfernt ist; sonst wächst das Fenster bis höchstens 8.
test('L3b: oben einfügen — Abstand nach unten nach dem Abwerfen genau 2 Sichthöhen → abgeworfen, 1 px weniger → behalten', () => {
  const genau = { ...WEIT, abstandUnten: 2 * SICHT + 700, hoeheUnten: 700 };
  const knapp = { ...genau, abstandUnten: 2 * SICHT + 700 - 1 };

  const abgeworfen = einfuegen(fensterAus(5), block('neu'), 'aelter', genau);
  const behalten = einfuegen(fensterAus(5), block('neu'), 'aelter', knapp);

  assert.deepEqual(abgeworfen.abgeworfen, [{ zeit: 'b4' }]);
  assert.equal(abgeworfen.fenster.bloecke.length, 5);
  assert.equal(behalten.abgeworfen, null);
  assert.deepEqual(ersteZeiten(behalten.fenster), ['neu', 'b0', 'b1', 'b2', 'b3', 'b4']);
  assert.equal(behalten.fenster.neuer, false);
});

test('L3b: unten einfügen — dieselbe Spanne am oberen Ende', () => {
  const fenster = { ...fensterAus(5), aelter: false, neuer: true };
  const genau = { ...WEIT, abstandOben: 2 * SICHT + 500, hoeheOben: 500 };
  const knapp = { ...genau, abstandOben: 2 * SICHT + 500 - 1 };

  const abgeworfen = einfuegen(fenster, block('neu'), 'neuer', genau);
  const behalten = einfuegen(fenster, block('neu'), 'neuer', knapp);

  assert.deepEqual(abgeworfen.abgeworfen, [{ zeit: 'b0' }]);
  assert.equal(abgeworfen.fenster.aelter, true);
  assert.equal(behalten.abgeworfen, null);
  assert.equal(behalten.fenster.bloecke.length, 6);
  assert.equal(behalten.fenster.aelter, false);
});

test('L3b: bei 8 Blöcken und nicht abwerfbarem anderem Ende lädt das Fenster nicht nach; abwerfbar oder 7 Blöcke schon', () => {
  const nah = { aelter: true, neuer: true, abstandOben: 100, abstandUnten: 100, hoeheOben: 1000, hoeheUnten: 1000, sicht: SICHT };

  assert.equal(nachladen(lage({ ...nah, bloecke: 8 })), null);
  assert.equal(nachladen(lage({ ...nah, bloecke: 7 })), 'aelter');
  assert.equal(nachladen(lage({ ...nah, bloecke: 8, abstandUnten: 2 * SICHT + 1000 })), 'aelter');
  assert.equal(nachladen(lage({ ...nah, bloecke: 8, aelter: false, abstandOben: 2 * SICHT + 1000 })), 'neuer');
});

test('L3b Simulation: Sicht 2.160 px, Blöcke zu 1.680 px — 6 Abrufe, danach keiner, 7 Blöcke im Fenster', () => {
  const sicht = 2160;
  const hoehe = 1680;
  let fenster = { bloecke: [[{ zeit: 'start' }]], aelter: true, neuer: true };
  let hoehen = [hoehe];
  let abstandOben = -240;
  let abstandUnten = -240;
  let abrufe = 0;
  for (let schritt = 0; schritt < 20; schritt += 1) {
    const masse = { abstandOben, abstandUnten, hoeheOben: hoehen[0], hoeheUnten: hoehen[hoehen.length - 1], sicht };
    const richtung = nachladen({ bloecke: fenster.bloecke.length, aelter: fenster.aelter, neuer: fenster.neuer, laeuft: false, ...masse });
    if (richtung === null) {
      continue;
    }
    abrufe += 1;
    const ergebnis = einfuegen(fenster, { nachrichten: [{ zeit: `s${schritt}` }], aelter: true, neuer: true }, richtung, masse);
    if (richtung === 'aelter') {
      hoehen = [hoehe, ...hoehen];
      abstandOben += hoehe;
      if (ergebnis.abgeworfen !== null) {
        abstandUnten -= hoehen.pop();
      }
    } else {
      hoehen = [...hoehen, hoehe];
      abstandUnten += hoehe;
      if (ergebnis.abgeworfen !== null) {
        abstandOben -= hoehen.shift();
      }
    }
    fenster = ergebnis.fenster;
  }

  assert.equal(abrufe, 6);
  assert.equal(fenster.bloecke.length, 7);
});

// R00041 F0142 B0635: der Block mit dem offenen Bearbeiten-Feld wird nicht abgeworfen — das Fenster wächst bis 8 und
// lädt dann an dieser Seite nicht weiter nach.
test('B0635: ist das untere Ende geschützt, fällt oben einfügend nichts weg; das obere geschützt, unten einfügend auch nicht', () => {
  const oben = einfuegen(fensterAus(5), block('neu'), 'aelter', { ...WEIT, untenGeschuetzt: true });
  assert.equal(oben.abgeworfen, null);
  assert.equal(oben.fenster.bloecke.length, 6);
  const unten = einfuegen({ ...fensterAus(5), neuer: true }, block('neu'), 'neuer', { ...WEIT, obenGeschuetzt: true });
  assert.equal(unten.abgeworfen, null);
  assert.equal(unten.fenster.bloecke.length, 6);
});

test('B0635: ein geschütztes Ende hält nur seine Seite — das andere Ende fällt weiter weg', () => {
  const ergebnis = einfuegen(fensterAus(5), block('neu'), 'aelter', { ...WEIT, obenGeschuetzt: true });
  assert.deepEqual(ergebnis.abgeworfen, [{ zeit: 'b4' }]);
});

test('B0635: bei 8 Blöcken und geschütztem anderem Ende lädt das Fenster nicht nach', () => {
  assert.equal(nachladen(lage({ bloecke: 8, aelter: true, abstandOben: 0, abstandUnten: 99999, hoeheUnten: 100, untenGeschuetzt: true })), null);
  assert.equal(nachladen(lage({ bloecke: 8, aelter: true, abstandOben: 0, abstandUnten: 99999, hoeheUnten: 100 })), 'aelter');
});
