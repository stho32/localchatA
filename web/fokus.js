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

// Tasten auf der fokussierten Nachricht (R00041 F0135, F0139; PO 127): `E` beginnt das Bearbeiten, `Entf` das Löschen —
// nur ohne Strg, Alt und Meta; Umschalt und Feststelltaste zählen nicht. Ob die Aktion an der Nachricht gilt, prüft der
// Aufrufer. → 'bearbeiten' | 'loeschen' | null
export function nachrichtTaste(ereignis) {
  if (ereignis.ctrlKey || ereignis.altKey || ereignis.metaKey) {
    return null;
  }
  if (ereignis.key === 'e' || ereignis.key === 'E') {
    return 'bearbeiten';
  }
  if (ereignis.key === 'Delete') {
    return 'loeschen';
  }
  return null;
}

// Taste `C` auf der fokussierten Nachricht (R00046 I0042, ADR-0052 Regel C1): nur Abkürzung für „Kopieren“, nie der einzige
// Weg. Ausgewertet wird `key` (nicht `code`: auf anderen Belegungen sitzt `C` anderswo); `c` und `C` bei Feststelltaste
// zählen, Strg, Alt, Meta und Umschalt nicht (Strg+C gehört dem Browser), ebenso wenig Wiederholung und Eingabemethode.
// lage: zielIstNachricht — der Fokus steht auf der Nachricht selbst, nicht in einem Feld, auf einem Knopf oder einem Zitat;
// markierungLeer — nichts ist markiert (sonst gilt das gewohnte Kopieren des Browsers); dialogOffen.
export function kopierTaste(ereignis, { zielIstNachricht, markierungLeer, dialogOffen }) {
  if (ereignis.ctrlKey || ereignis.altKey || ereignis.metaKey || ereignis.shiftKey || ereignis.repeat || ereignis.isComposing) {
    return false;
  }
  if (ereignis.key !== 'c' && ereignis.key !== 'C') {
    return false;
  }
  return zielIstNachricht && markierungLeer && !dialogOffen;
}
