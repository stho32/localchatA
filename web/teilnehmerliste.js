// Teilnehmerliste (A00005, ADR-0016 Regel 5, Designsystem components/Teilnehmerliste).
import { kuerzel } from './kopfzeile.js';
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

// Zeichnet die Gruppen in die Region: Überschrift je Art mit Anzahl, Unterkopf je Herkunft, je Zeile Kachel,
// Kennung in Mono, „Agent“, Status als Form (● online, ○ offline) und bei offline als Wort, die eigene Zeile
// mit „(du)“. Text nur über textContent; keine Animation, keine Ansage (die Region hat kein aria-live).
export function zeichnen(region, gruppen) {
  const inhalt = [];
  if (gruppen.menschen.length > 0) {
    inhalt.push(ueberschrift(`Menschen · ${gruppen.menschen.length}`));
    inhalt.push(...gruppen.menschen.map(zeileZeichnen));
  }
  const anzahlAgenten = gruppen.agenten.reduce((summe, gruppe) => summe + gruppe.zeilen.length, 0);
  if (anzahlAgenten > 0) {
    inhalt.push(ueberschrift(`Agenten · ${anzahlAgenten}`));
  }
  for (const gruppe of gruppen.agenten) {
    if (gruppe.herkunft !== null) {
      inhalt.push(unterkopf(gruppe.herkunft));
    }
    inhalt.push(...gruppe.zeilen.map(zeileZeichnen));
  }
  region.replaceChildren(...inhalt);
}

function ueberschrift(text) {
  return element('div', { className: 'lc-sect' }, [element('span', { textContent: text })]);
}

function unterkopf(herkunft) {
  return element('div', { className: 'lc-sect lc-unterkopf' }, [element('span', { className: 'lc-id', textContent: herkunft })]);
}

// Erste Zeile: Kachel, Kennung (umbrochen höchstens vor dem @), Status als Form und bei offline als Wort.
// Zweite Zeile, nur wenn nötig: „Agent“ und „(du)“ — so bleibt der Kennung die Breite der Spalte.
function zeileZeichnen(zeile) {
  const art = artDer(zeile);
  const istOffline = zeile.status === 'offline';
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(zeile.name) });
  kachel.setAttribute('aria-hidden', 'true');
  const kennung = element('span', { className: 'lc-id' }, [
    zeile.name, element('wbr'), element('i', { textContent: `@${zeile.herkunft}` }),
  ]);
  const status = element('span', { className: 'lc-person-status' }, [punkt(istOffline)]);
  if (istOffline) {
    status.append(element('span', { className: 'lc-status', textContent: 'offline' }));
  }
  const teile = [kachel, kennung, status];
  const merkmale = [];
  if (art === 'agent') {
    merkmale.push(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  if (zeile.eigene) {
    merkmale.push(element('span', { className: 'lc-du', textContent: '(du)' }));
  }
  if (merkmale.length > 0) {
    teile.push(element('span', { className: 'lc-person-merkmale' }, merkmale));
  }
  let klassen = 'lc-person';
  if (istOffline) {
    klassen += ' off';
  }
  return element('div', { className: klassen }, teile);
}

function punkt(istOffline) {
  let form = 'on';
  let wort = 'online';
  if (istOffline) {
    form = 'off';
    wort = 'offline';
  }
  const punktElement = element('span', { className: `lc-dot ${form}` });
  punktElement.setAttribute('role', 'img');
  punktElement.setAttribute('aria-label', wort);
  return punktElement;
}

function artDer(zeile) {
  if (zeile.art === 'mensch') {
    return 'mensch';
  }
  return 'agent';
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
