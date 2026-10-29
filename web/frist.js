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

// Kopfzeile ab 720 px (ADR-0033 Regel 2); null heißt: Einstellungen defekt, nichts entfällt.
export function fristText(tage) {
  if (tage === null) {
    return '· ⚠ Frist gesperrt';
  }
  return `· ${tageText(tage)}`;
}

// Zweiter Schritt des Dialogs (ADR-0033 Regel 4) mit den Zahlen der 409 `bestaetigen`.
export function bestaetigungText(tage, entfaellt) {
  let beiFrist = `${zahl(tage)} Tagen`;
  if (tage === 1) {
    beiFrist = '1 Tag';
  }
  return `Bei ${beiFrist} ${entfallenGebeugt(entfaellt.tage)} sofort ${tageText(entfaellt.tage)} bis einschließlich ${datumAusgeschrieben(entfaellt.bis)} `
    + `mit ${nachrichtenText(entfaellt.nachrichten)}. Das lässt sich nicht rückgängig machen.`;
}

// Das Verb richtet sich nach der Zahl der entfallenden Tage, nie nach der Frist.
function entfallenGebeugt(anzahl) {
  if (anzahl === 1) {
    return 'entfällt';
  }
  return 'entfallen';
}

// Statuszeile nach dem Speichern; entfallen { tage, nachrichten } aus der 200.
export function statusText(name, tage, entfallen) {
  if (entfallen.tage === 0) {
    return `Frist für # ${name}: ${tageText(tage)}.`;
  }
  return `Frist für # ${name}: ${tageText(tage)} — ${tageText(entfallen.tage)} mit ${nachrichtenText(entfallen.nachrichten)} entfallen.`;
}

// Hinweis im offenen Dialog, wenn jemand anders die Frist geändert hat (ADR-0033 Regel 5).
export function inzwischenText(tage) {
  if (tage === null) {
    return 'Inzwischen geändert: Frist gesperrt.';
  }
  return `Inzwischen geändert: ${tageText(tage)}.`;
}

function tageText(anzahl) {
  if (anzahl === 1) {
    return '1 Tag';
  }
  return `${zahl(anzahl)} Tage`;
}

function nachrichtenText(anzahl) {
  if (anzahl === 1) {
    return '1 Nachricht';
  }
  return `${zahl(anzahl)} Nachrichten`;
}

// Tausenderpunkt wie im Designsystem („4.210“).
function zahl(wert) {
  return new Intl.NumberFormat('de-DE').format(wert);
}

const MONATE = ['Januar', 'Februar', 'März', 'April', 'Mai', 'Juni', 'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember'];

// `JJJJ-MM-TT` → „21. Oktober 2026“, ohne Zeitzone (der Tag ist schon das Datum des Servers).
function datumAusgeschrieben(tag) {
  const [jahr, monat, tagImMonat] = tag.split('-').map(Number);
  return `${tagImMonat}. ${MONATE[monat - 1]} ${jahr}`;
}

// Fehlerzeile nach einer Antwort (ADR-0033 Ergänzung Karte #27, Regeln 1 und 3) → { text, eingabeFalsch }: nur die
// 422 `frist` ist ein Prüffehler; ein Serverfehler sagt, dass die Eingabe stimmt, und nennt einen Ausweg; die 429
// nennt die Uhrzeit, ab der es wieder geht — eine Restzeit veraltete, solange der Dialog offen steht.
// eintreffenMs: Uhr des Browsers beim Eintreffen der Antwort.
export function fehlerDerAntwort({ serverWeg, status, daten }, eintreffenMs) {
  if (serverWeg) {
    return { text: 'Nicht gespeichert — Verbindung prüfen.', eingabeFalsch: false };
  }
  const eingabeFalsch = status === 422 && daten !== null && daten.regel === 'frist';
  if (status === 429 && daten !== null && typeof daten.fehler === 'string') {
    const wer = daten.fehler.split(' — ')[0];
    return { text: `${wer} — wieder möglich ab ${wiederMoeglichAb(eintreffenMs, daten.wieder_in_ms)}.`, eingabeFalsch };
  }
  if (daten !== null && typeof daten.fehler === 'string') {
    return { text: daten.fehler, eingabeFalsch };
  }
  return { text: `Nicht gespeichert (Status ${status}) — später erneut versuchen.`, eingabeFalsch };
}

// Ortszeit „HH:MM:SS“ von Eintreffen + Wartezeit, auf die volle Sekunde aufgerundet — ohne toLocaleTimeString, damit
// die Form nicht an der Sprache des Browsers hängt.
export function wiederMoeglichAb(eintreffenMs, wiederInMs) {
  const ab = new Date(Math.ceil((eintreffenMs + wiederInMs) / 1000) * 1000);
  return [ab.getHours(), ab.getMinutes(), ab.getSeconds()].map((teil) => String(teil).padStart(2, '0')).join(':');
}
