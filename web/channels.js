// Channel-Liste (B0133, ADR-0023 Regel 6): Stand und Wechsel des Themas Channels zusammenführen.
//
// Liste: { stand, channels: [{ name, angelegt, angelegt_von, beschreibung }] }, nach Name sortiert wie am
// Server (Zeichen für Zeichen, nicht nach Sprache).

export const LEER = { stand: 0, channels: [] };

// Ein Stand (art "channels") ersetzt die Liste ganz — auch mit kleinerer Nummer, denn nach einem Neustart zählt
// der Server von vorn. Ein Wechsel (art "channel") gilt nur mit größerem stand und fügt nach Name ein oder
// ersetzt den Channel gleichen Namens.
export function channelsZusammenfuehren(liste, ereignis) {
  if (ereignis.art === 'channels') {
    return { stand: ereignis.stand, channels: [...ereignis.liste].sort(nachName) };
  }
  if (ereignis.art !== 'channel' || ereignis.stand <= liste.stand) {
    return liste;
  }
  const uebrige = liste.channels.filter((channel) => channel.name !== ereignis.channel.name);
  return { stand: ereignis.stand, channels: [...uebrige, ereignis.channel].sort(nachName) };
}

function nachName(links, rechts) {
  if (links.name < rechts.name) {
    return -1;
  }
  if (links.name > rechts.name) {
    return 1;
  }
  return 0;
}
