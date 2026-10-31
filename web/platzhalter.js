// Der Platzhalter des leeren Eingabefelds (R00030 F0184, B0798): `text-overflow: ellipsis` wirkt an ::placeholder nicht, darum
// wählt das Skript die längste Fassung, die in die Breite passt — nie mitten im Wort abgeschnitten:
// „Nachricht an # allgemein“ → „An # allgemein“ → „# allgemein“ → der Name mit „…“. Im Direktgespräch entfällt für die
// kürzeren Fassungen der @-Teil der Kennung. Der zugängliche Name des Felds bleibt, wie er ist.
const AUSLASSUNG = '…';

// anrede: „# allgemein“ bzw. die Kennung des Gegenübers; passt(text) → Passt der Text in die Breite?
// → der Platzhalter; passt nichts, der kürzeste Rest mit „…“
export function platzhalterWaehlen(anrede, passt) {
  const volle = `Nachricht an ${anrede}`;
  if (passt(volle)) {
    return volle;
  }
  const name = anrede.split('@')[0];
  for (const fassung of [`Nachricht an ${name}`, `An ${name}`, name]) {
    if (fassung !== volle && passt(fassung)) {
      return fassung;
    }
  }
  return mitAuslassung(name, passt);
}

function mitAuslassung(name, passt) {
  const zeichen = Array.from(name);
  for (let laenge = zeichen.length - 1; laenge >= 1; laenge -= 1) {
    const gekuerzt = `${zeichen.slice(0, laenge).join('')}${AUSLASSUNG}`;
    if (passt(gekuerzt)) {
      return gekuerzt;
    }
  }
  return `${zeichen[0]}${AUSLASSUNG}`;
}

// Misst die Breite eines Textes in der Schrift des Felds (Canvas) und vergleicht mit der nutzbaren Breite ohne Innenabstand.
// Ist das Feld nicht sichtbar (Breite 0), passt alles — es gibt nichts zu kürzen.
export function passtInFeld(feld) {
  const stil = getComputedStyle(feld);
  const verfuegbar = feld.clientWidth - parseFloat(stil.paddingLeft) - parseFloat(stil.paddingRight);
  if (feld.clientWidth === 0) {
    return () => true;
  }
  const zeichner = document.createElement('canvas').getContext('2d');
  zeichner.font = `${stil.fontStyle} ${stil.fontWeight} ${stil.fontSize} ${stil.fontFamily}`;
  return (text) => zeichner.measureText(text).width <= verfuegbar;
}
