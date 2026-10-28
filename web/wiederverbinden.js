// Verbindung im Tab (ADR-0037 Regeln 4, 7; ADR-0028 Regel 6), rein: getrennt heißt, der Strom ist unterbrochen oder
// das Netz des Tabs ist aus (der Strom eines geteilten Workers merkt das nicht immer). Wird der Tab wieder verbunden,
// holt er nach; ein Stand bei verbundenem Strom (NeuerStand) holt ebenso nach — nur der erste Stand eines neuen
// Stroms nicht, für den hat das Wiederverbinden schon geholt.
//
// ereignis: { art: 'strom', zustand: 'verbunden' | 'unterbrochen' } · { art: 'netz', online } · { art: 'stand' }
// → { zustand, aktion: 'trennen' | 'wiederverbinden' | 'nachholen' | null }
export const VERBINDUNG_ANFANG = { strom: true, netz: true, standErwartet: true };

export function verbindung(zustand, ereignis) {
  switch (ereignis.art) {
    case 'strom':
      return stromGewechselt(zustand, ereignis.zustand === 'verbunden');
    case 'netz':
      return wechseln(zustand, { ...zustand, netz: ereignis.online });
    case 'stand':
      return standGekommen(zustand);
    default:
      throw new Error(`Verbindung: unbekanntes Ereignis ${ereignis.art}`);
  }
}

function wechseln(vorher, nachher) {
  const warGetrennt = getrennt(vorher);
  const istGetrennt = getrennt(nachher);
  if (!warGetrennt && istGetrennt) {
    return { zustand: nachher, aktion: 'trennen' };
  }
  if (warGetrennt && !istGetrennt) {
    return { zustand: nachher, aktion: 'wiederverbinden' };
  }
  return { zustand: nachher, aktion: null };
}

// Ein neuer Strom beginnt mit einem Stand; ein wiederholtes „verbunden“ (ein Tab tritt dem Worker bei) ist keiner.
function stromGewechselt(zustand, verbunden) {
  const neuerStrom = verbunden && !zustand.strom;
  return wechseln(zustand, { ...zustand, strom: verbunden, standErwartet: zustand.standErwartet || neuerStrom });
}

function standGekommen(zustand) {
  if (getrennt(zustand)) {
    return { zustand, aktion: null };
  }
  if (zustand.standErwartet) {
    return { zustand: { ...zustand, standErwartet: false }, aktion: null };
  }
  return { zustand, aktion: 'nachholen' };
}

function getrennt(zustand) {
  return !zustand.strom || !zustand.netz;
}
