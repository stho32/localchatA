// Teilnehmerliste (A00005, ADR-0016 Regel 5, ADR-0019 Regeln 5–7, Designsystem components/Teilnehmerliste).
import { kuerzel } from './kopfzeile.js';
import { restzeit } from './restzeit.js';
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
// Kennung in Mono, „Agent“, Status als Form und Wort (statusform), die eigene Zeile mit „(du)“. Text nur über
// textContent; keine Animation, keine Ansage (die Region hat kein aria-live). Die Restzeit rechnet mit der Uhr
// des Browsers und dem Versatz zur Serverzeit.
export function zeichnen(region, gruppen, { versatz = 0, jetzt = Date.now() } = {}) {
  const uhr = { versatz, jetzt, standUnbekannt: region.closest('.lc-unbekannt') !== null };
  const zeileFuer = (zeile) => zeileZeichnen(zeile, uhr);
  const inhalt = [];
  if (gruppen.menschen.length > 0) {
    inhalt.push(ueberschrift(`Menschen · ${gruppen.menschen.length}`));
    inhalt.push(...gruppen.menschen.map(zeileFuer));
  }
  const anzahlAgenten = gruppen.agenten.reduce((summe, gruppe) => summe + gruppe.zeilen.length, 0);
  if (anzahlAgenten > 0) {
    inhalt.push(ueberschrift(`Agenten · ${anzahlAgenten}`));
  }
  for (const gruppe of gruppen.agenten) {
    if (gruppe.herkunft !== null) {
      inhalt.push(unterkopf(gruppe.herkunft));
    }
    inhalt.push(...gruppe.zeilen.map(zeileFuer));
  }
  region.replaceChildren(...inhalt);
}

// Die Form einer Zeile (rein): ● online — bei unbekanntem Stand „zuletzt online“ —, ○ „offline“, oder bis 60 s
// vor dem Verschwinden gestrichelt mit „weg in m:ss“; der Name „offline, gleich weg“ bleibt statisch, damit
// nichts sekündlich angesagt wird. Eine Zeile „gleich weg“ zählt auch bei unbekanntem Stand weiter herunter.
export function statusform(zeile, { versatz, jetzt, standUnbekannt }) {
  if (zeile.status !== 'offline') {
    if (standUnbekannt) {
      return { form: 'zuletzt-online', name: 'zuletzt online', text: null };
    }
    return { form: 'online', name: 'online', text: null };
  }
  const rest = restzeit({ wegUm: zeile.weg_um, versatz, uhr: jetzt });
  if (rest.art === 'gleich-weg') {
    return { form: 'gleich-weg', name: 'offline, gleich weg', text: rest.text };
  }
  return { form: 'offline', name: 'offline', text: 'offline' };
}

function ueberschrift(text) {
  return element('div', { className: 'lc-sect' }, [element('span', { textContent: text })]);
}

function unterkopf(herkunft) {
  return element('div', { className: 'lc-sect lc-unterkopf' }, [element('span', { className: 'lc-id', textContent: herkunft })]);
}

// Kachel, dann Kennung mit „Agent“ und „(du)“ dahinter (umbrochen erst bei Platzmangel, die Kennung vor dem @),
// rechts der Status in einer festen Spalte — so verschiebt ein Wechsel nach offline oder „gleich weg“ keine Zeile.
function zeileZeichnen(zeile, uhr) {
  const art = artDer(zeile);
  const form = statusform(zeile, uhr);
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(zeile.name) });
  kachel.setAttribute('aria-hidden', 'true');
  const kennung = element('span', { className: 'lc-id' }, [
    zeile.name, element('wbr'), element('i', { textContent: `@${zeile.herkunft}` }),
  ]);
  const text = element('span', { className: 'lc-person-text' }, [kennung]);
  if (art === 'agent') {
    text.append(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  if (zeile.eigene) {
    text.append(element('span', { className: 'lc-du', textContent: '(du)' }));
  }
  const status = element('span', { className: 'lc-person-status' }, [punkt(form)]);
  if (form.text !== null) {
    status.append(statuswort(form));
  }
  let klassen = 'lc-person';
  if (zeile.status === 'offline') {
    klassen += ' off';
  }
  const zeilenElement = element('div', { className: klassen }, [kachel, text, status]);
  zeilenElement.dataset.kennung = zeile.kennung;
  return zeilenElement;
}

// Die Restzeit ist für Screenreader verborgen: der Punkt trägt den statischen Namen (ADR-0019 Regel 5).
function statuswort(form) {
  const wort = element('span', { className: 'lc-status', textContent: form.text });
  if (form.form === 'gleich-weg') {
    wort.setAttribute('aria-hidden', 'true');
  }
  return wort;
}

const PUNKT_KLASSE = { online: 'on', 'zuletzt-online': 'on', offline: 'off', 'gleich-weg': 'gleich' };

function punkt(form) {
  const punktElement = element('span', { className: `lc-dot ${PUNKT_KLASSE[form.form]}` });
  punktElement.setAttribute('role', 'img');
  punktElement.setAttribute('aria-label', form.name);
  return punktElement;
}

// Solange die Verbindung fehlt, weiß niemand, ob ein Teilnehmer noch online ist: der Punkt heißt dann
// „zuletzt online“ (Nacharbeit 2).
export function onlineBenennen(bereich, standUnbekannt) {
  const { name } = statusform({ status: 'online' }, { standUnbekannt });
  bereich.querySelectorAll('.lc-dot.on').forEach((punktElement) => {
    punktElement.setAttribute('aria-label', name);
  });
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
