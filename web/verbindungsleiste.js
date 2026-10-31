// Leiste oben im Verlauf (Designsystem VerlaufsMarken „Verbindung“, ADR-0037 Regel 7), rein: getrennt → „Verbindung
// verloren“ als Fehler; nach dem Nachholen 4 s lang „✓ Wieder verbunden · n nachgeladen“ bzw. „· Verlauf geladen“.
import { zahlMitEinheit } from './groesse.js';

// ergebnis des Nachholens: { art: 'nachgeholt', zahl } | { art: 'neu' } | { art: 'geladen' } (ein nie geladener
// Channel, ADR-0037 Ergänzung „Karte #61“, 7k)
export function wiederVerbundenText(ergebnis) {
  if (ergebnis.art === 'neu') {
    return '✓ Wieder verbunden · neueste geladen';
  }
  if (ergebnis.art === 'geladen') {
    return '✓ Wieder verbunden · Verlauf geladen';
  }
  if (ergebnis.zahl === 0) {
    return '✓ Wieder verbunden';
  }
  return `✓ Wieder verbunden · ${new Intl.NumberFormat('de-DE').format(ergebnis.zahl)} nachgeladen`;
}

// zustand: { getrennt, naechsterVersuchUm: ms seit der Epoche | null, versuchLaeuft, meldung: Text | null, meldungSeitMs }
// → { text, fehler } | null
export function leisteInhalt({ getrennt, naechsterVersuchUm = null, versuchLaeuft = false, meldung, meldungSeitMs }, jetztMs) {
  const MELDUNG_STEHT_MS = 4000;
  if (getrennt) {
    return { text: warnungText(naechsterVersuchUm, jetztMs, versuchLaeuft), fehler: true };
  }
  const meldungSteht = meldung !== null && jetztMs - meldungSeitMs < MELDUNG_STEHT_MS;
  if (!meldungSteht) {
    return null;
  }
  return { text: meldung, fehler: false };
}

// Die Warnung mit Countdown (R00025 F0084, ADR-0015 Ergänzung Karte #36): Rest bis zum nächsten Versuch in ganzen
// Sekunden, aufgerundet, begrenzt auf 0–10 s (eine verstellte Uhr zählt nie über 10); erreicht heißt „wird verbunden …“.
// Ohne Zeitpunkt (Netz des Tabs aus, der Strom hat noch nicht gemeldet) kein Countdown. Läuft gerade ein Versuch
// (Karte #65, R00037 Regel 2), steht „wird verbunden …“ mit Vorrang vor Countdown und fehlendem Zeitpunkt.
export function warnungText(naechsterVersuchUm, jetztMs, versuchLaeuft = false) {
  const HOECHSTENS_MS = 10000;
  if (versuchLaeuft) {
    return '⚠ Verbindung verloren · wird verbunden …';
  }
  if (naechsterVersuchUm === null) {
    return '⚠ Verbindung verloren';
  }
  const restMs = Math.min(Math.max(naechsterVersuchUm - jetztMs, 0), HOECHSTENS_MS);
  if (restMs === 0) {
    return '⚠ Verbindung verloren · wird verbunden …';
  }
  return `⚠ Verbindung verloren · neuer Versuch in ${zahlMitEinheit(Math.ceil(restMs / 1000), 's')}`;
}

// Welcher Zeitpunkt den Countdown trägt (ADR-0015 Ergänzung Karte #36 Regel 5): ist der Strom unterbrochen, der des
// Stroms; ist er verbunden und das Nachholen gescheitert, der des nächsten Nachhol-Versuchs; sonst (nur das Netz des
// Tabs ist aus, der Strom hat noch nicht gemeldet) keiner. → ms seit der Epoche | null
export function geltenderZeitpunkt({ stromUnterbrochen, strom, nachholen }) {
  if (stromUnterbrochen) {
    return strom;
  }
  return nachholen;
}

// Die Leiste besteht aus zwei Teilen (R00033 F0124, ux-reviewer #36 S1): dem Anlass „⚠ Verbindung verloren“, der bei
// Platzmangel mit „…“ kürzt, und dem Stand „ · neuer Versuch in n s“ bzw. „ · wird verbunden …“, der nie gekürzt wird —
// am schmalen Telefon bleibt die eine Zahl, die sagt, ob sich Warten lohnt. Der Text als Ganzes bleibt der von
// `warnungText`; jeder andere Text (Meldungen, Warnung ohne Zeitpunkt) ist ein Anlass ohne Stand.
const STAND = / · (?:neuer Versuch in \d+ s|wird verbunden …)$/;

export function leisteTeile(text) {
  const treffer = STAND.exec(text);
  if (treffer === null) {
    return { anlass: text, stand: '' };
  }
  return { anlass: text.slice(0, treffer.index), stand: treffer[0] };
}

export function leisteFuellen(leiste, text) {
  const { anlass, stand } = leisteTeile(text);
  const teile = [teil('lc-leiste-anlass', anlass)];
  if (stand !== '') {
    teile.push(teil('lc-leiste-stand', stand));
  }
  leiste.replaceChildren(...teile);
}

function teil(klasse, text) {
  const element = document.createElement('span');
  element.className = klasse;
  element.textContent = text;
  return element;
}
