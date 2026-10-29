// Textregel im Browser (ADR-0038 Regel 2): dieselbe Regel wie der Server (src/gespraeche/textregel.rs), rein.
// Reihenfolge: leer (nur Leerraum) → zu lang (Bytes UTF-8) → verbotenes Zeichen (das erste, Stelle in Zeichen ab 0):
// C0 außer Tab und Zeilenumbruch, DEL, C1; ein Wagenrücklauf nur unmittelbar vor einem Zeilenumbruch.
// → { art: 'ok' | 'leer' | 'laenge', bytes } | { art: 'zeichen', bytes, stelle, zeichen: 'U+001B' }
export const TEXT_HOECHSTENS_BYTES = 16384;

export function textPruefen(text) {
  const bytes = new TextEncoder().encode(text).length;
  if (nurLeerraum(text)) {
    return { art: 'leer', bytes };
  }
  if (bytes > TEXT_HOECHSTENS_BYTES) {
    return { art: 'laenge', bytes };
  }
  const verboten = erstesVerbotenes(Array.from(text));
  if (verboten === null) {
    return { art: 'ok', bytes };
  }
  return { art: 'zeichen', bytes, ...verboten };
}

// Leer ist ein Text nur aus Zeichen der Unicode-Eigenschaft White_Space — dieselbe Klasse wie char::is_whitespace des
// Servers; U+FEFF und U+200B gehören nicht dazu (ADR-0038 Ergänzung Karte #31).
export function nurLeerraum(text) {
  return /^\p{White_Space}*$/u.test(text);
}

function erstesVerbotenes(zeichen) {
  for (let stelle = 0; stelle < zeichen.length; stelle += 1) {
    if (verboten(zeichen, stelle)) {
      return { stelle, zeichen: schreibweise(zeichen[stelle]) };
    }
  }
  return null;
}

function verboten(zeichen, stelle) {
  const einzeln = zeichen[stelle];
  const wagenruecklaufVorUmbruch = einzeln === '\r' && zeichen[stelle + 1] === '\n';
  const erlaubt = einzeln === '\t' || einzeln === '\n' || wagenruecklaufVorUmbruch;
  return /^\p{Cc}$/u.test(einzeln) && !erlaubt;
}

function schreibweise(einzeln) {
  return `U+${einzeln.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')}`;
}

// „Steuerzeichen entfernen“ (R00025 F0086): nimmt genau die Zeichen, die erstesVerbotenes meldet — nicht ganze
// ANSI-Folgen. → { text, entfernt }
export function steuerzeichenEntfernen(text) {
  const zeichen = Array.from(text);
  const bleibende = zeichen.filter((einzeln, stelle) => !verboten(zeichen, stelle));
  return { text: bleibende.join(''), entfernt: zeichen.length - bleibende.length };
}
