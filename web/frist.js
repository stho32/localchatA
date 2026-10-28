// Frist im Browser (R00008 F0037, ADR-0033): Eingabe prüfen und Texte formen — rein, ohne DOM.

const FRIST_MIN = 1;
const FRIST_MAX = 3650;

// Nur Ziffern, Wert 1 bis 3650 wie am Server (ADR-0029 Regel 1) → { gueltig: true, tage } | { gueltig: false, fehler }.
export function fristPruefen(text) {
  const nurZiffern = /^[0-9]+$/.test(text);
  const tage = Number(text);
  if (!nurZiffern || tage < FRIST_MIN || tage > FRIST_MAX) {
    return { gueltig: false, fehler: 'Frist: ganze Zahl von 1 bis 3650 Tagen.' };
  }
  return { gueltig: true, tage };
}
