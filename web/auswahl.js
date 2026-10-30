// Channel-Auswahl (ADR-0027): die Auswahl ist eine Ableitung aus Adresse und Channel-Liste, kein eigener Zustand.
// Nur `#/c/<name>` mit einem gültigen Channelnamen wählt; ist der Name nicht in der Liste, ist er unbekannt.
import { pruefen } from './namensregel.js';
import { DIREKT_ADRESSE, direktgespraech, direktHash, gegenueberIn } from './gespraech.js';

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

// Direktnachricht (ADR-0055 Regel 3, A00005 Ergänzung Karte #13): `#/d/<kennung>` nennt das Gegenüber, prozentkodiert
// oder nicht. Gilt nur eine andere Kennung in kanonischer Schreibweise wie am Server — sonst führt die Adresse zu keinem
// Gespräch, und es wird nichts gefragt (V14). → null (keine Direktnachricht) | { gegenueber: Kennung | null }
export function direktAuswahl(hash, eigeneKennung) {
  if (!hash.startsWith(DIREKT_ADRESSE)) {
    return null;
  }
  const kennung = entschluesselt(hash.slice(DIREKT_ADRESSE.length));
  if (kennung === null || kennung === eigeneKennung || !kanonischeKennung(kennung)) {
    return { gegenueber: null };
  }
  return { gegenueber: kennung };
}

function entschluesselt(teil) {
  try {
    return decodeURIComponent(teil);
  } catch (ausnahme) {
    return null;
  }
}

// name@herkunft: ein Teilnehmername nach der Namensregel, die Herkunft eine IPv4- oder IPv6-Adresse so, wie der
// URL-Parser des Browsers sie selbst schreibt (IPv6 klein und gekürzt) — dieselbe Form, die der Server verlangt. Eine
// IPv4-gemappte Adresse (`::ffff:7f00:1`) ist am Server ein Alias der IPv4-Kennung und damit kein Gespräch.
const IPV4_GEMAPPT = /^::ffff:[0-9a-f]{1,4}:[0-9a-f]{1,4}$/;

function kanonischeKennung(kennung) {
  const teile = kennung.split('@');
  if (teile.length !== 2 || !pruefen(teile[0], 'teilnehmer').gueltig) {
    return false;
  }
  const herkunft = teile[1];
  if (herkunft.includes(':')) {
    return !IPV4_GEMAPPT.test(herkunft) && hostnameDes(`[${herkunft}]`) === `[${herkunft}]`;
  }
  return /^\d+\.\d+\.\d+\.\d+$/.test(herkunft) && hostnameDes(herkunft) === herkunft;
}

function hostnameDes(host) {
  try {
    return new URL(`http://${host}/`).hostname;
  } catch (ausnahme) {
    return null;
  }
}

// Zuletzt gewählt je Tab (ADR-0027 Regel 3): sessionStorage unter diesem Schlüssel — der Gesprächsschlüssel, ein Channel
// oder ein Direktgespräch (gespraech.js; ux-reviewer #13 F0178 Optional 4).
export const MERKER = 'localchatA.channel';

// Startet die Seite ohne Adresse, wird das gemerkte Gespräch zur Adresse (per replaceState); sonst bleibt sie.
// → Adresse | null
export function startadresse(hash, merker) {
  const ohneAdresse = hash === '' || hash === '#';
  if (!ohneAdresse || merker === null) {
    return null;
  }
  const gegenueber = gegenueberIn(merker);
  if (gegenueber !== null) {
    return gemerktesDirektgespraech(gegenueber);
  }
  const adresse = PRAEFIX + merker;
  if (auswahl(adresse, []) === null) {
    return null;
  }
  return adresse;
}

function gemerktesDirektgespraech(gegenueber) {
  if (!kanonischeKennung(gegenueber)) {
    return null;
  }
  return direktHash(gegenueber);
}

export function auswahlMerken(speicher, gewaehlt) {
  if (gewaehlt !== null) {
    speicher.setItem(MERKER, gewaehlt.name);
  }
}

export function direktMerken(speicher, gegenueber) {
  speicher.setItem(MERKER, direktgespraech(gegenueber));
}

// Der Rückweg aus einer Adresse ohne Gespräch führt nur zu einem gemerkten Channel. → Name | null
export function gemerkterChannel(merker) {
  if (merker === null || gegenueberIn(merker) !== null) {
    return null;
  }
  return merker;
}

// Alt ↓ / Alt ↑ (ADR-0027 Regel 4): richtung 'ab' | 'auf' → Name | null; ohne bekannte Auswahl ↓ der erste,
// ↑ der letzte; am Rand nichts.
export function nachbarChannel(namen, gewaehlt, richtung) {
  const stelle = namen.indexOf(gewaehlt);
  if (stelle === -1) {
    return erster(namen, richtung);
  }
  let schritt = -1;
  if (richtung === 'ab') {
    schritt = 1;
  }
  const nachbar = namen[stelle + schritt];
  if (nachbar === undefined) {
    return null;
  }
  return nachbar;
}

function erster(namen, richtung) {
  if (namen.length === 0) {
    return null;
  }
  if (richtung === 'ab') {
    return namen[0];
  }
  return namen[namen.length - 1];
}

// Die Folge, durch die Alt ↓/↑ geht (R00050 V12): die Channels, danach die Direktnachrichten — beide in der Reihenfolge der
// Seitenleiste, als Gesprächsschlüssel.
export function gespraechsfolge(channelNamen, direktKennungen) {
  return [...channelNamen, ...direktKennungen.map(direktgespraech)];
}

// Nur Alt mit ↓ bzw. ↑ — ohne Strg, Umschalt, Meta und AltGraph. → 'ab' | 'auf' | null
export function kuerzelRichtung(ereignis) {
  const nurAlt = ereignis.altKey && !ereignis.ctrlKey && !ereignis.shiftKey && !ereignis.metaKey
    && !ereignis.getModifierState('AltGraph');
  if (!nurAlt) {
    return null;
  }
  if (ereignis.key === 'ArrowDown') {
    return 'ab';
  }
  if (ereignis.key === 'ArrowUp') {
    return 'auf';
  }
  return null;
}
