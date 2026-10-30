// Tasten, die den Fokus in die Eingabe holen (ADR-0038 Regel 8, 1-tastatur.md), rein: `/` außerhalb von Textfeldern
// und `Esc` im Verlauf. ort: 'textfeld' | 'verlauf' | 'dialog' | 'sonst' → 'ins-feld' | null
export function fokusTaste(ereignis, ort) {
  const mitUmschalter = ereignis.ctrlKey || ereignis.altKey || ereignis.metaKey;
  if (mitUmschalter || ort === 'dialog') {
    return null;
  }
  const schraegstrichAusserhalb = ereignis.key === '/' && ort !== 'textfeld';
  const escImVerlauf = ereignis.key === 'Escape' && ort === 'verlauf';
  if (schraegstrichAusserhalb || escImVerlauf) {
    return 'ins-feld';
  }
  return null;
}

// Rollfokus im Verlauf (R00041 F0141): ↑ geht zur vorigen, ↓ zur nächsten Nachricht — nur die Taste allein; mit Strg,
// Alt, Meta oder Umschalt gehört sie anderen Tasten (Alt ↑ wechselt den Channel, 1-tastatur.md).
export function pfeilRichtung(ereignis) {
  const mitUmschalter = ereignis.ctrlKey || ereignis.altKey || ereignis.metaKey || ereignis.shiftKey;
  if (mitUmschalter) {
    return null;
  }
  if (ereignis.key === 'ArrowUp') {
    return 'zurueck';
  }
  if (ereignis.key === 'ArrowDown') {
    return 'vor';
  }
  return null;
}

// Die Stelle, zu der die Richtung von `stelle` aus führt; am Rand keine.
export function pfeilZiel(richtung, stelle, anzahl) {
  if (richtung === 'zurueck' && stelle > 0) {
    return stelle - 1;
  }
  if (richtung === 'vor' && stelle < anzahl - 1) {
    return stelle + 1;
  }
  return null;
}

// Genau eine Nachricht ist Tab-Stopp: die zuletzt fokussierte (`gemerkt`, ihre Zeit), solange sie im Verlauf steht,
// sonst die neueste. → Stelle in `zeiten` oder null ohne Nachrichten.
export function tabStopp(zeiten, gemerkt) {
  if (zeiten.length === 0) {
    return null;
  }
  const gemerkteStelle = zeiten.indexOf(gemerkt);
  if (gemerkteStelle >= 0) {
    return gemerkteStelle;
  }
  return zeiten.length - 1;
}

// ↑ allein in der leeren Eingabe (kein Leerraum, keine Komposition) führt zur eigenen letzten Nachricht; mit Text
// bewegt es die Schreibmarke (1-tastatur.md).
export function zuEigenerLetzten(ereignis, text) {
  const komponiert = ereignis.isComposing || ereignis.keyCode === 229;
  return pfeilRichtung(ereignis) === 'zurueck' && text === '' && !komponiert;
}

// Wie weit zurück ↑ in der leeren Eingabe nach der eigenen letzten Nachricht sucht (R00041 US-13: die neuesten 100).
export const SUCHTIEFE_EIGENE_LETZTE = 100;

// Die Stelle der jüngsten bestätigten, nicht gelöschten Nachricht von `kennung` unter den neuesten `suchtiefe` von
// `nachrichten` ({ von, zustand }, älteste zuerst); null, wenn es dort keine gibt.
export function eigeneLetzte(nachrichten, kennung, suchtiefe = SUCHTIEFE_EIGENE_LETZTE) {
  const ersteGesuchte = Math.max(0, nachrichten.length - suchtiefe);
  for (let stelle = nachrichten.length - 1; stelle >= ersteGesuchte; stelle -= 1) {
    const nachricht = nachrichten[stelle];
    if (nachricht.von === kennung && nachricht.zustand !== 'geloescht') {
      return stelle;
    }
  }
  return null;
}
