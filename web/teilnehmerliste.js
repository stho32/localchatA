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
