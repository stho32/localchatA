// Channel-Auswahl (ADR-0027): die Auswahl ist eine Ableitung aus Adresse und Channel-Liste, kein eigener Zustand.
// Nur `#/c/<name>` mit einem gültigen Channelnamen wählt; ist der Name nicht in der Liste, ist er unbekannt.
import { pruefen } from './namensregel.js';

const PRAEFIX = '#/c/';

// → { name, bekannt } | null
export function auswahl(hash, namen) {
  if (!hash.startsWith(PRAEFIX)) {
    return null;
  }
  const name = hash.slice(PRAEFIX.length);
  if (!pruefen(name, 'channel').gueltig) {
    return null;
  }
  return { name, bekannt: namen.includes(name) };
}

// Zuletzt gewählt je Tab (ADR-0027 Regel 3): sessionStorage unter diesem Schlüssel.
export const MERKER = 'localchatA.channel';

// Startet die Seite ohne Adresse, wird der gemerkte Channel zur Adresse (per replaceState); sonst bleibt sie.
// → Adresse | null
export function startadresse(hash, merker) {
  const ohneAdresse = hash === '' || hash === '#';
  if (!ohneAdresse || merker === null) {
    return null;
  }
  const adresse = PRAEFIX + merker;
  if (auswahl(adresse, []) === null) {
    return null;
  }
  return adresse;
}

export function auswahlMerken(speicher, gewaehlt) {
  if (gewaehlt !== null) {
    speicher.setItem(MERKER, gewaehlt.name);
  }
}
