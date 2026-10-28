// Eingabe unter dem Verlauf (ADR-0038 Regeln 1–3, Designsystem Eingabe).

const IME_TASTENCODE = 229;

// Was eine Taste im Feld bewirkt (Regel 2): 'senden' | 'zeile' | null. Während einer IME-Komposition und bei Text
// nur aus Leerraum nie senden; Alt Enter tut nichts.
export function taste(ereignis, text) {
  const komponiert = ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
  if (ereignis.key !== 'Enter' || komponiert || ereignis.altKey) {
    return null;
  }
  if (ereignis.shiftKey) {
    return 'zeile';
  }
  if (text.trim() === '') {
    return null;
  }
  return 'senden';
}
