// Direktnachrichten der Seitenleiste (R00050 F0180 B0786, V8; ADR-0055 Regel 3, D11), rein: welche Gespräche — die
// Kennungen der Gegenüber aus `GET /api/direkt` und aus jeder `nachricht` mit `direkt` —, alphabetisch nach Kennung,
// nie nach Aktivität; dazu je Eintrag, was er über das Gegenüber sagt. Kein Zähler, kein @n, kein Marker (PO O20).
import { statusform } from './teilnehmerliste.js';
import { kuerzel } from './kopfzeile.js';

// bekannt: [Kennung] | null (noch keine Antwort) · kennungen: [Kennung] → [Kennung]; ohne Neues dieselbe Liste.
export function direktlisteZusammenfuehren(bekannt, kennungen) {
  const bisher = bekannt || [];
  const neue = kennungen.filter((kennung) => !bisher.includes(kennung));
  if (bekannt !== null && neue.length === 0) {
    return bekannt;
  }
  return [...new Set([...bisher, ...neue])].sort(natuerlich);
}

function natuerlich(links, rechts) {
  return links.localeCompare(rechts, 'de', { numeric: true });
}

// Ein Eintrag (V8): Statuspunkt wie in der Teilnehmerliste, offline und „gleich weg“ zusätzlich als Wort — statisch, die
// Seitenleiste zählt nicht herunter; ein Gegenüber, das nicht mehr in der Liste steht, ist „weg“. Auf dem Telefon trägt
// das Kürzel den Eintrag, der Name bleibt ganz (V8, V7: nicht nur Farbe).
// anwesende: Einträge der Teilnehmerliste · uhr: { versatz, jetzt, standUnbekannt }
// → { kennung, kuerzel, punkt: 'on' | 'off' | 'gleich', wort: Text | null, name }
export function direkteintrag(kennung, anwesende, uhr) {
  const gegenueber = anwesende.find((eintrag) => eintrag.kennung === kennung);
  const form = formDes(gegenueber, uhr);
  return {
    kennung,
    kuerzel: kuerzel(kennung.split('@')[0]),
    punkt: form.punkt,
    wort: form.wort,
    name: `Direktnachricht mit ${kennung}, ${form.name}`,
  };
}

function formDes(gegenueber, uhr) {
  if (gegenueber === undefined) {
    return { punkt: 'off', wort: 'weg', name: 'weg' };
  }
  const form = statusform(gegenueber, uhr);
  switch (form.form) {
    case 'online':
    case 'zuletzt-online':
      return { punkt: 'on', wort: null, name: form.name };
    case 'offline':
      return { punkt: 'off', wort: 'offline', name: form.name };
    case 'gleich-weg':
      return { punkt: 'gleich', wort: 'gleich weg', name: form.name };
    default:
      throw new Error(`Direktliste: unbekannte Form ${form.form}`);
  }
}
