// Lücken nach dem Wiederverbinden (ADR-0037 Regel 7, ADR-0028 Regel 6): ab der jüngsten bestätigten Zeit in Blöcken
// zu 200 holen, bis `neuer` falsch ist, höchstens 5 Blöcke — ist danach noch mehr da, öffnet der Tab neu.
// abrufen: async (cursor { nach }, n) → { art: 'block', block } | { art: 'fehler' | 'abgebrochen', … }
// → { art: 'nachgeholt', nachrichten } | { art: 'neu' } | { art: 'fehler' }
export async function nachholen(juengste, abrufen) {
  const BLOCKGROESSE = 200;
  const HOECHSTENS_BLOECKE = 5;
  const ANFANG_ALLER_ZEITEN = '1970-01-01T00:00:00.000+00:00';
  let nach = juengste;
  if (nach === null) {
    nach = ANFANG_ALLER_ZEITEN;
  }
  const nachrichten = [];
  for (let block = 0; block < HOECHSTENS_BLOECKE; block += 1) {
    const ergebnis = await abrufen({ nach }, BLOCKGROESSE);
    if (ergebnis.art !== 'block') {
      return { art: 'fehler' };
    }
    nachrichten.push(...ergebnis.block.nachrichten);
    if (!ergebnis.block.neuer) {
      return { art: 'nachgeholt', nachrichten };
    }
    nach = nachrichten[nachrichten.length - 1].zeit;
  }
  return { art: 'neu' };
}
