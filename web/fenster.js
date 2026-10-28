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
