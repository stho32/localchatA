// Namensregel als Hinweis beim Tippen (ADR-0013, ADR-0020): dieselbe Regel und Umformung wie am Server,
// maßgeblich bleibt dessen 422-Antwort — geschickt wird, was der Mensch bestätigt. Channelnamen schließen
// zusätzlich die Gerätenamen aus, die Windows nicht als Ordner zulässt.

const LAENGE = 32;
const HINWEIS = 'Erlaubt sind 1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende.';
const GUELTIGER_NAME = /^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$/;

// Buchstaben mit Akzent aus U+00C0–U+017F und ihr Grundbuchstabe — dieselbe Tabelle wie AKZENTE in src/name.rs.
export const AKZENTE = [
  ['àáâãåāăą', 'a'],
  ['æ', 'ae'],
  ['çćĉċč', 'c'],
  ['ðďđ', 'd'],
  ['èéêëēĕėęě', 'e'],
  ['ĝğġģ', 'g'],
  ['ĥħ', 'h'],
  ['ìíîïĩīĭįıİ', 'i'],
  ['ĳ', 'ij'],
  ['ĵ', 'j'],
  ['ķĸ', 'k'],
  ['ĺļľŀł', 'l'],
  ['ñńņňŉŋ', 'n'],
  ['òóôõøōŏő', 'o'],
  ['œ', 'oe'],
  ['ŕŗř', 'r'],
  ['śŝşšſ', 's'],
  ['ţťŧ', 't'],
  ['þ', 'th'],
  ['ùúûũūŭůűų', 'u'],
  ['ŵ', 'w'],
  ['ýÿŷ', 'y'],
  ['źżž', 'z'],
];

// art: 'teilnehmer' (Standard) oder 'channel'.
export function pruefen(text, art = 'teilnehmer') {
  const mitGeraetenamen = art === 'channel';
  if (mitGeraetenamen && geraetename(text)) {
    const hinweis = `${text} ist ein Gerätename, den Windows nicht als Ordner zulässt — einen anderen wählen.`;
    return { gueltig: false, hinweis, vorschlag: '' };
  }
  if (GUELTIGER_NAME.test(text)) {
    return { gueltig: true, hinweis: '', vorschlag: '' };
  }
  let vorschlag = umformen(text);
  if (mitGeraetenamen && geraetename(vorschlag)) {
    vorschlag = '';
  }
  return { gueltig: false, hinweis: HINWEIS, vorschlag };
}

function geraetename(name) {
  return /^(con|prn|aux|nul|com[0-9]|lpt[0-9])$/.test(name);
}

function umformen(text) {
  const ersatz = { 'ä': 'ae', 'ö': 'oe', 'ü': 'ue', 'ß': 'ss' };
  let umgeformt = '';
  for (const zeichen of [...text].flatMap(klein)) {
    const grund = grundbuchstabe(zeichen);
    if (ersatz[zeichen] !== undefined) {
      umgeformt += ersatz[zeichen];
    } else if (/[a-z0-9]/.test(zeichen)) {
      umgeformt += zeichen;
    } else if (grund !== undefined) {
      umgeformt += grund;
    } else if (!umgeformt.endsWith('-')) {
      umgeformt += '-';
    }
  }
  const ohneRaender = umgeformt.replace(/^-+|-+$/g, '');
  return ohneRaender.slice(0, LAENGE).replace(/-+$/, '');
}

// Ein Zeichen der Akzenttabelle bleibt, wie es ist (İ würde sonst zu i mit kombinierendem Punkt).
function klein(zeichen) {
  if (grundbuchstabe(zeichen) !== undefined) {
    return [zeichen];
  }
  return [...zeichen.toLowerCase()];
}

function grundbuchstabe(zeichen) {
  const zeile = AKZENTE.find(([buchstaben]) => buchstaben.includes(zeichen));
  if (zeile === undefined) {
    return undefined;
  }
  return zeile[1];
}
