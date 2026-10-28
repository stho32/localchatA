// Teilnehmerliste (A00005, ADR-0016 Regel 5, Designsystem components/Teilnehmerliste).
//
// Liste: { stand, eintraege: [{ kennung, name, herkunft, art, status, seit, stand }], entfernt: { kennung: stand } }

export const LEER = { stand: 0, eintraege: [], entfernt: {} };

// Ein Stand (art "teilnehmer") steht am Anfang jedes Stroms und ersetzt die Liste ganz — auch mit kleinerer
// Nummer, denn nach einem Neustart des Programms zählt der Server von vorn. Ein Wechsel (art "anwesenheit")
// gilt nur, wenn sein stand größer ist als der, den die Zeile zuletzt bekam; `weg` entfernt die Zeile.
export function zusammenfuehren(liste, ereignis) {
  if (ereignis.art === 'teilnehmer') {
    const eintraege = ereignis.liste.map((zeile) => ({ ...zeile, stand: ereignis.stand }));
    return { stand: ereignis.stand, eintraege, entfernt: {} };
  }
  if (ereignis.art !== 'anwesenheit') {
    return liste;
  }
  const { kennung } = ereignis.teilnehmer;
  const veraltet = ereignis.stand <= zuletztGesehenerStand(liste, kennung);
  if (veraltet) {
    return liste;
  }
  const uebrige = liste.eintraege.filter((eintrag) => eintrag.kennung !== kennung);
  if (ereignis.teilnehmer.status === 'weg') {
    return { ...liste, eintraege: uebrige, entfernt: { ...liste.entfernt, [kennung]: ereignis.stand } };
  }
  const neu = { ...ereignis.teilnehmer, stand: ereignis.stand };
  return { ...liste, eintraege: [...uebrige, neu] };
}

function zuletztGesehenerStand(liste, kennung) {
  const eintrag = liste.eintraege.find((kandidat) => kandidat.kennung === kennung);
  if (eintrag !== undefined) {
    return eintrag.stand;
  }
  if (kennung in liste.entfernt) {
    return liste.entfernt[kennung];
  }
  return liste.stand;
}

// Designsystem: zuerst Menschen, dann Agenten, je alphabetisch; ab 8 Agenten nach Herkunft gruppiert,
// damit auch 20 und mehr lesbar bleiben. Ergebnis: { menschen: [zeile], agenten: [{ herkunft, zeilen }] },
// herkunft null ohne Unterkopf; jede Zeile mit `eigene`.
const AGENTEN_AB_DENEN_GRUPPIERT_WIRD = 8;

export function ordnen(liste, eigeneKennung) {
  const zeilen = liste.eintraege
    .map((eintrag) => ({ ...eintrag, eigene: eintrag.kennung === eigeneKennung }))
    .sort(nachKennung);
  const menschen = zeilen.filter((zeile) => zeile.art === 'mensch');
  const agenten = zeilen.filter((zeile) => zeile.art !== 'mensch');
  return { menschen, agenten: agentenGruppieren(agenten) };
}

function agentenGruppieren(agenten) {
  if (agenten.length === 0) {
    return [];
  }
  if (agenten.length < AGENTEN_AB_DENEN_GRUPPIERT_WIRD) {
    return [{ herkunft: null, zeilen: agenten }];
  }
  const herkuenfte = [...new Set(agenten.map((agent) => agent.herkunft))].sort(natuerlich);
  return herkuenfte.map((herkunft) => ({
    herkunft,
    zeilen: agenten.filter((agent) => agent.herkunft === herkunft),
  }));
}

function nachKennung(links, rechts) {
  return natuerlich(links.kennung, rechts.kennung);
}

function natuerlich(links, rechts) {
  return links.localeCompare(rechts, 'de', { numeric: true });
}
