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
