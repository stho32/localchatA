// Namensregel als Hinweis beim Tippen (ADR-0013): dieselbe Regel und Umformung wie am Server,
// maßgeblich bleibt dessen 422-Antwort — geschickt wird, was der Mensch bestätigt.

const LAENGE = 32;
const HINWEIS = 'Erlaubt sind 1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende.';
const GUELTIGER_NAME = /^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$/;

export function pruefen(text) {
  if (GUELTIGER_NAME.test(text)) {
    return { gueltig: true, hinweis: '', vorschlag: '' };
  }
  return { gueltig: false, hinweis: HINWEIS, vorschlag: umformen(text) };
}

function umformen(text) {
  const ersatz = { 'ä': 'ae', 'ö': 'oe', 'ü': 'ue', 'ß': 'ss' };
  let umgeformt = '';
  for (const zeichen of text.toLowerCase()) {
    if (ersatz[zeichen] !== undefined) {
      umgeformt += ersatz[zeichen];
    } else if (/[a-z0-9]/.test(zeichen)) {
      umgeformt += zeichen;
    } else if (!umgeformt.endsWith('-')) {
      umgeformt += '-';
    }
  }
  const ohneRaender = umgeformt.replace(/^-+|-+$/g, '');
  return ohneRaender.slice(0, LAENGE).replace(/-+$/, '');
}
