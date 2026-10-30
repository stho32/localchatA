// Die Lage des Aktionsmenüs auf Touch-Geräten (R00046 I0042, UX K1): links neben dem Schalter „⋯“, beginnend in dessen
// Höhe, und immer vollständig sichtbar — nicht vom Verlauf abgeschnitten, nicht vom Fenster verdeckt. Rein: alle Maße
// kommen als Parameter (Rechtecke { oben, unten, links, rechts }, Menü { breite, hoehe }).
export const MENUE_RAND = 4;

// bereiche: die Orte, in denen das Menü liegen darf, nach Vorrang (der Verlauf, dann der sichtbare Teil des Fensters);
// genommen wird der erste, in den das Menü samt Rand passt, sonst der letzte.
// → { oben, links } in Fensterkoordinaten
export function menueLage({ schalter, menue, bereiche }) {
  const bereich = bereichWaehlen(bereiche, menue);
  const hoechstOben = bereich.unten - MENUE_RAND - menue.hoehe;
  const oben = Math.max(bereich.oben + MENUE_RAND, Math.min(schalter.oben, hoechstOben));
  const links = Math.max(bereich.links + MENUE_RAND, schalter.links - menue.breite);
  return { oben, links };
}

function bereichWaehlen(bereiche, menue) {
  const passend = bereiche.find((bereich) => bereich.unten - bereich.oben >= menue.hoehe + 2 * MENUE_RAND);
  if (passend === undefined) {
    return bereiche[bereiche.length - 1];
  }
  return passend;
}
