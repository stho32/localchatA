// Unit-Tests der Verbindung im Tab (R00012 F0047 „NeuerStand“, B0245; ADR-0037 Regel 7, ADR-0028 Regel 6; node
// --test): getrennt heißt Strom unterbrochen oder Netz des Tabs aus; wann nachgeholt wird.
import test from 'node:test';
import assert from 'node:assert/strict';
import { VERBINDUNG_ANFANG, verbindung } from '../../web/wiederverbinden.js';

function ablauf(...ereignisse) {
  let zustand = VERBINDUNG_ANFANG;
  return ereignisse.map((ereignis) => {
    const ergebnis = verbindung(zustand, ereignis);
    zustand = ergebnis.zustand;
    return ergebnis.aktion;
  });
}

const STROM_WEG = { art: 'strom', zustand: 'unterbrochen' };
const STROM_DA = { art: 'strom', zustand: 'verbunden' };
const NETZ_AUS = { art: 'netz', online: false };
const NETZ_AN = { art: 'netz', online: true };
const STAND = { art: 'stand' };

test('Strom weg und wieder: trennen, dann wiederverbinden; der erste Stand des neuen Stroms holt nicht noch einmal', () => {
  assert.deepEqual(ablauf(STROM_WEG, STROM_DA, STAND, STAND), ['trennen', 'wiederverbinden', null, 'nachholen']);
});

test('NeuerStand bei verbundenem Strom löst dasselbe Nachholen aus wie ein Wiederverbinden', () => {
  assert.deepEqual(ablauf(STAND, STAND), [null, 'nachholen']);
});

test('Netz des Tabs aus und wieder an: trennen, dann wiederverbinden — ohne einen neuen Stand abzuwarten', () => {
  assert.deepEqual(ablauf(STAND, NETZ_AUS, NETZ_AN, STAND), [null, 'trennen', 'wiederverbinden', 'nachholen']);
});

test('getrennt bleibt getrennt, solange einer von beiden fehlt; Stände holen dann nichts', () => {
  assert.deepEqual(ablauf(NETZ_AUS, STROM_WEG, NETZ_AN, STAND, STROM_DA), ['trennen', null, null, null, 'wiederverbinden']);
});

test('wiederholte Zustände ändern nichts', () => {
  assert.deepEqual(ablauf(STROM_DA, NETZ_AN, STROM_WEG, STROM_WEG), [null, null, 'trennen', null]);
});

test('ein Tab, der dem Worker beitritt, bekommt Stand und dann „verbunden“ — der nächste Stand ist ein NeuerStand', () => {
  assert.deepEqual(ablauf(STAND, STROM_DA, STAND), [null, null, 'nachholen']);
});
