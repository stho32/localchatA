// Textregel im Browser (ADR-0038 Regel 2): dieselbe Regel wie der Server (src/gespraeche/textregel.rs), rein.
// Reihenfolge: leer (nur Leerraum) → zu lang (Bytes UTF-8) → verbotenes Zeichen (das erste, Stelle in Zeichen ab 0):
// C0 außer Tab und Zeilenumbruch, DEL, C1; ein Wagenrücklauf nur unmittelbar vor einem Zeilenumbruch.
// → { art: 'ok' | 'leer' | 'laenge', bytes } | { art: 'zeichen', bytes, stelle, zeichen: 'U+001B' }
export const TEXT_HOECHSTENS_BYTES = 16384;

export function textPruefen(text) {
  const bytes = new TextEncoder().encode(text).length;
  if (/^\p{White_Space}*$/u.test(text)) {
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

function erstesVerbotenes(zeichen) {
  for (let stelle = 0; stelle < zeichen.length; stelle += 1) {
    const einzeln = zeichen[stelle];
    const wagenruecklaufVorUmbruch = einzeln === '\r' && zeichen[stelle + 1] === '\n';
    const erlaubt = einzeln === '\t' || einzeln === '\n' || wagenruecklaufVorUmbruch;
    if (/^\p{Cc}$/u.test(einzeln) && !erlaubt) {
      return { stelle, zeichen: schreibweise(einzeln) };
    }
  }
  return null;
}

function schreibweise(einzeln) {
  return `U+${einzeln.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')}`;
}
