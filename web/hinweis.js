// Hinweiszeile unter dem Feld (ADR-0038 Regel 4), rein: aus dem Ergebnis der Textregel (textregel.js) die Stufe und
// der Text — unter 15.000 Bytes nichts, ab da „noch n Zeichen“ (n = 16.384 − Bytes), bei einem Verstoß der Fehler mit ⚠.
import { TEXT_HOECHSTENS_BYTES } from './textregel.js';

// → { stufe: 'leer' | 'zaehlen' | 'fehler', text }
export function hinweis(ergebnis) {
  const ZAEHLEN_AB_BYTES = 15000;
  if (ergebnis.art === 'laenge') {
    return { stufe: 'fehler', text: `⚠ Zu lang: ${zahl(ergebnis.bytes)} von höchstens ${zahl(TEXT_HOECHSTENS_BYTES)} Byte — bitte teilen.` };
  }
  if (ergebnis.art === 'zeichen') {
    return { stufe: 'fehler', text: `⚠ Steuerzeichen ${ergebnis.zeichen} an Stelle ${ergebnis.stelle + 1} — bitte entfernen.` };
  }
  if (ergebnis.bytes < ZAEHLEN_AB_BYTES) {
    return { stufe: 'leer', text: '' };
  }
  return { stufe: 'zaehlen', text: `noch ${zahl(TEXT_HOECHSTENS_BYTES - ergebnis.bytes)} Zeichen` };
}

// Angesagt wird nur, wenn die Zeile die Schwelle überschreitet oder in einen Fehler wechselt — nie je Taste.
export function hinweisAnsagen(vorige, neue) {
  const schwelleUeberschritten = vorige.stufe === 'leer' && neue.stufe === 'zaehlen';
  const inEinenFehler = vorige.stufe !== 'fehler' && neue.stufe === 'fehler';
  return schwelleUeberschritten || inEinenFehler;
}

function zahl(wert) {
  return new Intl.NumberFormat('de-DE').format(wert);
}
