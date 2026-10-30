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
