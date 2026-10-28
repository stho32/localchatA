// Fenster des Verlaufs (ADR-0026 Regeln 1, 2), rein: wann nachgeladen wird und wie ein Block ins Fenster aus
// höchstens fünf Blöcken kommt.

// Nachladen, sobald der Abstand zu einem Rand unter 2 Sichthöhen fällt und es dort mehr gibt; höchstens ein Abruf
// zugleich, oben zuerst. lage: { bloecke, aelter, neuer, abstandOben, abstandUnten, sicht, laeuft }
// → 'aelter' | 'neuer' | null
export function nachladen({ bloecke, aelter, neuer, abstandOben, abstandUnten, sicht, laeuft }) {
  const SCHWELLE_IN_SICHTHOEHEN = 2;
  if (laeuft || bloecke === 0) {
    return null;
  }
  const schwelle = SCHWELLE_IN_SICHTHOEHEN * sicht;
  if (aelter && abstandOben < schwelle) {
    return 'aelter';
  }
  if (neuer && abstandUnten < schwelle) {
    return 'neuer';
  }
  return null;
}

// Ein Block vorn ('aelter') oder hinten ('neuer'); hat das Fenster dann mehr als 5 Blöcke, fällt am anderen Ende
// einer weg, und dort gibt es danach mehr. fenster: { bloecke: [[nachricht…]…], aelter, neuer };
// block: { nachrichten, aelter, neuer } aus der API. → { fenster, abgeworfen: [nachricht…] | null }
export function einfuegen(fenster, block, richtung) {
  const HOECHSTENS_BLOECKE = 5;
  const leer = block.nachrichten.length === 0;
  if (richtung === 'aelter') {
    let bloecke = [block.nachrichten, ...fenster.bloecke];
    if (leer) {
      bloecke = fenster.bloecke;
    }
    if (bloecke.length <= HOECHSTENS_BLOECKE) {
      return { fenster: { bloecke, aelter: block.aelter, neuer: fenster.neuer }, abgeworfen: null };
    }
    return {
      fenster: { bloecke: bloecke.slice(0, -1), aelter: block.aelter, neuer: true },
      abgeworfen: bloecke[bloecke.length - 1],
    };
  }
  let bloecke = [...fenster.bloecke, block.nachrichten];
  if (leer) {
    bloecke = fenster.bloecke;
  }
  if (bloecke.length <= HOECHSTENS_BLOECKE) {
    return { fenster: { bloecke, aelter: fenster.aelter, neuer: block.neuer }, abgeworfen: null };
  }
  return { fenster: { bloecke: bloecke.slice(1), aelter: true, neuer: block.neuer }, abgeworfen: bloecke[0] };
}
