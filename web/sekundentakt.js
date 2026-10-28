// Sekundentakt (B0112, ADR-0019 Regel 5): läuft nur, solange mindestens eine Zeile „gleich weg“ ist, und zeichnet
// bei jedem Schlag nur diese Zeilen neu. Jeder Aufruf beendet den vorigen Takt — die Liste hat sich geändert.
import { restzeit } from './restzeit.js';

export function sekundentaktAnlegen({
  planen = (rueckruf, ms) => setInterval(rueckruf, ms),
  abbrechen = (zeitgeber) => clearInterval(zeitgeber),
} = {}) {
  let zeitgeber = null;
  return function takten({ zeilen, uhr, zeileNeuZeichnen }) {
    if (zeitgeber !== null) {
      abbrechen(zeitgeber);
      zeitgeber = null;
    }
    const gleichWeg = zeilen.filter((zeile) => istGleichWeg(zeile, uhr()));
    if (gleichWeg.length === 0) {
      return;
    }
    const SEKUNDE_MS = 1000;
    zeitgeber = planen(() => gleichWeg.forEach((zeile) => zeileNeuZeichnen(zeile)), SEKUNDE_MS);
  };
}

function istGleichWeg(zeile, { versatz, jetzt }) {
  if (zeile.status !== 'offline') {
    return false;
  }
  return restzeit({ wegUm: zeile.weg_um, versatz, uhr: jetzt }).art === 'gleich-weg';
}
