// Text in die Zwischenablage (R00046 I0042, ADR-0052 Regel 2). Der erste Weg läuft synchron in der Geste des Aufrufers —
// vor jedem await: ein verstecktes, schreibgeschütztes Textfeld mit dem Text, markiert, dann document.execCommand('copy').
// Er geht auch über http im LAN, wo es navigator.clipboard nicht gibt. Nur wenn er scheitert und die Seite ein sicherer
// Kontext ist, schreibt navigator.clipboard.writeText als zweiter Weg. Die Zwischenablage wird nie gelesen, nie als HTML
// beschrieben; Fokus und Markierung bleiben, wie sie waren.
// → 'kopiert' | 'nicht_moeglich'
export async function kopieren(text, dokument) {
  if (ueberTextfeldKopieren(text, dokument)) {
    return 'kopiert';
  }
  return ueberSchnittstelleKopieren(text, dokument.defaultView);
}

function ueberTextfeldKopieren(text, dokument) {
  const vorigesElement = dokument.activeElement;
  const vorigeMarkierung = markierungMerken(dokument);
  const feld = verstecktesFeldAnlegen(text, dokument);
  try {
    dokument.body.append(feld);
    feld.select();
    feld.setSelectionRange(0, text.length);
    return dokument.execCommand('copy');
  } catch {
    return false;
  } finally {
    feld.remove();
    markierungZurueck(vorigeMarkierung);
    if (vorigesElement !== null && vorigesElement !== undefined) {
      vorigesElement.focus({ preventScroll: true });
    }
  }
}

// 16 px, damit iOS beim Markieren nicht zoomt; position: fixed, damit nichts verschoben wird.
function verstecktesFeldAnlegen(text, dokument) {
  const feld = dokument.createElement('textarea');
  feld.value = text;
  feld.readOnly = true;
  feld.setAttribute('aria-hidden', 'true');
  feld.setAttribute('tabindex', '-1');
  Object.assign(feld.style, { position: 'fixed', top: '0', left: '0', opacity: '0', pointerEvents: 'none', fontSize: '16px' });
  return feld;
}

async function ueberSchnittstelleKopieren(text, fenster) {
  const schnittstelle = fenster.navigator.clipboard;
  if (!fenster.isSecureContext || schnittstelle === undefined || typeof schnittstelle.writeText !== 'function') {
    return 'nicht_moeglich';
  }
  try {
    await schnittstelle.writeText(text);
    return 'kopiert';
  } catch {
    return 'nicht_moeglich';
  }
}

function markierungMerken(dokument) {
  const markierung = dokument.getSelection();
  const bereiche = [];
  for (let stelle = 0; stelle < markierung.rangeCount; stelle += 1) {
    bereiche.push(markierung.getRangeAt(stelle));
  }
  return { markierung, bereiche };
}

function markierungZurueck({ markierung, bereiche }) {
  markierung.removeAllRanges();
  for (const bereich of bereiche) {
    markierung.addRange(bereich);
  }
}

// Rückmeldung (ADR-0052 Regel 3, UX-Vorprüfung #78): wie lange „✓ Kopiert.“ bzw. „⚠ Kopieren nicht möglich“ am Knopf (oder
// im Menüeintrag) steht und wie lange das Menü auf Touch dabei offen bleibt.
export const ANZEIGE_KOPIERT_MS = 2000;
export const ANZEIGE_FEHLER_MS = 5000;
export const MENUE_OFFEN_MS = 1200;

const ANSAGE_KOPIERT = 'Kopiert.';
const ANSAGE_FEHLER = 'Kopieren nicht möglich. Text markieren und Strg+C drücken.';

// zeit/angesagtUm: für welche Nachricht und wann zuletzt „Kopiert.“ angesagt wurde.
export const RUECKMELDUNG_LEER = { zeit: null, angesagtUm: null };

// Aus dem Ergebnis von `kopieren` für die Nachricht `zeit` (Uhr `jetztMs`) die Rückmeldung: was am Knopf steht, wie lange,
// und was die Ansage-Region sagt (null: nichts). Dieselbe Nachricht innerhalb von 2 s noch einmal kopiert: die Anzeige gilt
// neu, die Region schweigt — ein Bildschirmleser sagt sonst nichts Neues. Ein Fehlschlag wird jedes Mal gesagt.
// → { zustand, anzeige: 'kopiert' | 'nicht_moeglich', dauerMs, ansage: Text | null }
export function rueckmeldungFuer(zustand, ergebnis, zeit, jetztMs) {
  if (ergebnis === 'nicht_moeglich') {
    return { zustand, anzeige: 'nicht_moeglich', dauerMs: ANZEIGE_FEHLER_MS, ansage: ANSAGE_FEHLER };
  }
  const gerade = zustand.zeit === zeit && zustand.angesagtUm !== null && jetztMs - zustand.angesagtUm < ANZEIGE_KOPIERT_MS;
  if (gerade) {
    return { zustand, anzeige: 'kopiert', dauerMs: ANZEIGE_KOPIERT_MS, ansage: null };
  }
  return { zustand: { zeit, angesagtUm: jetztMs }, anzeige: 'kopiert', dauerMs: ANZEIGE_KOPIERT_MS, ansage: ANSAGE_KOPIERT };
}
