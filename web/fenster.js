// Fenster des Verlaufs (ADR-0026 Regeln 1, 2 mit Ergänzung Abwurfschutz), rein: wann nachgeladen wird und wie ein
// Block ins Fenster kommt — in der Regel höchstens fünf Blöcke, bis zu acht, wenn das andere Ende noch nah am Blick
// ist (R00013 K8: Blöcke aus großen Nachrichten sind niedriger als zwei Sichthöhen).
// masse: { abstandOben, abstandUnten, hoeheOben, hoeheUnten, sicht, obenGeschuetzt?, untenGeschuetzt? } — Abstand des
// Blicks zu den Rändern des Inhalts und Höhe des obersten und untersten Blocks, in Pixeln; ein geschütztes Ende (es
// trägt ein offenes Bearbeiten-Feld, R00041 F0142, oder ein spielendes Medium, R00030 B0754) wird nie abgeworfen.

const SCHWELLE_IN_SICHTHOEHEN = 2;
const REGEL_BLOECKE = 5;
const HOECHSTENS_BLOECKE = 8;

// Nachladen, sobald der Abstand zu einem Rand unter 2 Sichthöhen fällt und es dort mehr gibt; höchstens ein Abruf
// zugleich, oben zuerst. Bei 8 Blöcken nur, wenn das andere Ende danach abgeworfen werden darf.
// lage: { bloecke, aelter, neuer, laeuft, ...masse } → 'aelter' | 'neuer' | null
export function nachladen({
  bloecke, aelter, neuer, abstandOben, abstandUnten, hoeheOben, hoeheUnten, sicht, laeuft, obenGeschuetzt = false, untenGeschuetzt = false,
}) {
  if (laeuft || bloecke === 0) {
    return null;
  }
  const schwelle = SCHWELLE_IN_SICHTHOEHEN * sicht;
  let richtung = null;
  if (aelter && abstandOben < schwelle) {
    richtung = 'aelter';
  } else if (neuer && abstandUnten < schwelle) {
    richtung = 'neuer';
  }
  const voll = bloecke >= HOECHSTENS_BLOECKE;
  const masse = { abstandOben, abstandUnten, hoeheOben, hoeheUnten, sicht, obenGeschuetzt, untenGeschuetzt };
  if (richtung !== null && voll && !anderesEndeAbwerfbar(richtung, masse)) {
    return null;
  }
  return richtung;
}

// Ein Block vorn ('aelter') oder hinten ('neuer'); hat das Fenster dann mehr als 5 Blöcke, fällt am anderen Ende
// einer weg, wenn der Blick danach noch mindestens 2 Sichthöhen von diesem Ende entfernt ist, sonst wächst das
// Fenster (über 8 fällt immer einer weg); dort gibt es danach mehr. fenster: { bloecke: [[nachricht…]…], aelter,
// neuer }; block: { nachrichten, aelter, neuer } aus der API. → { fenster, abgeworfen: [nachricht…] | null }
export function einfuegen(fenster, block, richtung, masse) {
  const oben = richtung === 'aelter';
  let bloecke = fenster.bloecke;
  if (block.nachrichten.length > 0) {
    bloecke = oben ? [block.nachrichten, ...fenster.bloecke] : [...fenster.bloecke, block.nachrichten];
  }
  const aelter = oben ? block.aelter : fenster.aelter;
  const neuer = oben ? fenster.neuer : block.neuer;
  const ueberRegel = bloecke.length > REGEL_BLOECKE && anderesEndeAbwerfbar(richtung, masse);
  const abwerfen = (bloecke.length > HOECHSTENS_BLOECKE && !anderesEndeGeschuetzt(richtung, masse)) || ueberRegel;
  if (!abwerfen) {
    return { fenster: { bloecke, aelter, neuer }, abgeworfen: null };
  }
  if (oben) {
    return { fenster: { bloecke: bloecke.slice(0, -1), aelter, neuer: true }, abgeworfen: bloecke[bloecke.length - 1] };
  }
  return { fenster: { bloecke: bloecke.slice(1), aelter: true, neuer }, abgeworfen: bloecke[0] };
}

// Ist das andere Ende nicht geschützt, und bleibt der Blick nach dem Abwerfen seines Blocks mindestens 2 Sichthöhen von
// ihm entfernt?
function anderesEndeAbwerfbar(richtung, masse) {
  const { abstandOben, abstandUnten, hoeheOben, hoeheUnten, sicht } = masse;
  if (anderesEndeGeschuetzt(richtung, masse)) {
    return false;
  }
  const danach = richtung === 'aelter' ? abstandUnten - hoeheUnten : abstandOben - hoeheOben;
  return danach >= SCHWELLE_IN_SICHTHOEHEN * sicht;
}

function anderesEndeGeschuetzt(richtung, { obenGeschuetzt = false, untenGeschuetzt = false }) {
  if (richtung === 'aelter') {
    return untenGeschuetzt;
  }
  return obenGeschuetzt;
}
