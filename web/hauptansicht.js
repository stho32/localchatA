// Hauptansicht (Designsystem: Bildschirm 4.2, Baustein Hauptansicht): Kopfzeile, links die Seitenleiste
// „Channels“, Mitte und rechts die Region „Teilnehmer“. Tab-Reihenfolge: Kopfzeile → Seitenleiste → Mitte →
// Teilnehmer (ADR-0024 Regel 1). Der Hinweis auf eine unterbrochene
// Verbindung steht in Fehlergestalt (⚠ und Wort, kd-danger) unter der Kopfzeile über die ganze Breite. `Strg .` und der Schalter
// „Teilnehmer“ blenden die Liste ein und aus (1-tastatur.md). `+` und `Strg Alt N` öffnen „Channel anlegen“; die
// Statuszeile meldet den angelegten Channel (ADR-0024 Regeln 4, 5). optionen.channelAnlegen: async (name, beschreibung) → Antwort.
// `# name` in der Kopfzeile öffnet die Channel-Einstellungen (ADR-0033); optionen.fristSetzen: async (name, tage,
// bestaetigt) → Antwort.
// Die Auswahl folgt allein Adresse und Channel-Liste (ADR-0027): Start, hashchange und jede neue Liste leiten sie
// neu ab — Seitenleiste, Kopfzeile und Mitte zeigen sie.
import * as kopfzeile from './kopfzeile.js';
import { auswahl, auswahlMerken, startadresse, MERKER, nachbarChannel, kuerzelRichtung } from './auswahl.js';
import { verlaufAnlegen } from './verlauf.js';
import * as seitenleiste from './seitenleiste.js';
import { dialogEinrichten } from './channel-anlegen.js';
import { einstellungenEinrichten } from './channel-einstellungen.js';
import { onlineBenennen } from './teilnehmerliste.js';
import { eingabeAnlegen } from './eingabe.js';
import { sendenEinrichten } from './senden.js';
import { entwurfAnlegen } from './entwurf.js';
import { fokusTaste } from './fokus.js';
import { VERBINDUNG_ANFANG, verbindung } from './wiederverbinden.js';
import { nachholLaufAnlegen } from './nachhollauf.js';
import { wiederVerbundenText, leisteInhalt } from './verbindungsleiste.js';
import { ANSAGE_LEER, ansage, ansageSchreiberAnlegen } from './ansage.js';

const UNTERBROCHEN = '⚠ Verbindung unterbrochen — localchatA verbindet neu.';
const STAND_UNBEKANNT = 'Stand unbekannt — zuletzt bekannte Liste.';
let vorigeAnsichtAbmelden = new AbortController();

export function zeichnen(wurzel, teilnehmer, optionen) {
  vorigeAnsichtAbmelden.abort();
  vorigeAnsichtAbmelden = new AbortController();
  const anzahl = element('span', { className: 'lc-count', textContent: '0' });
  const schalter = element('button', { type: 'button', className: 'lc-hbtn' }, ['Teilnehmer ', anzahl]);
  schalter.setAttribute('aria-controls', 'lc-teilnehmer');
  let gewaehlterChannel = null;
  const einstellungen = einstellungenEinrichten({
    wurzel,
    setzen: optionen.fristSetzen,
    statusZeigen: statusMelden,
    neuOeffnen: (name) => verlauf.oeffnen(name),
  });
  const kopf = kopfzeile.zeichnen(wurzel, teilnehmer, {
    ...optionen,
    schalter: [schalter],
    einstellungenOeffnen: (ausloeser) => einstellungen.oeffnen(gewaehlterChannel, ausloeser),
  });
  const hinweis = element('p', { id: 'lc-verbindung', className: 'lc-meldung lc-fehler lc-verbindung', hidden: true });
  hinweis.setAttribute('role', 'alert');
  const standUnbekannt = element('p', { className: 'lc-stand', hidden: true });
  const zeilen = element('div', { className: 'lc-members-zeilen' });
  const region = element('aside', { id: 'lc-teilnehmer', className: 'lc-members', tabIndex: 0 }, [standUnbekannt, zeilen]);
  region.setAttribute('aria-label', 'Teilnehmer');
  const channels = seitenleiste.anlegen();
  const mitte = element('main', { className: 'lc-center' });
  const haupt = element('div', { className: 'lc-main' }, [channels.nav, mitte, region]);
  // Die Statuszeile hat keine eigene Rolle: ihr Text geht zugleich an die eine Ansage-Region (R00025 F0084).
  const statuszeile = element('p', { className: 'lc-statuszeile', hidden: true });
  const ansagen = ansageEinrichten();
  function statusMelden(text) {
    statusZeigen(statuszeile, text);
    ansagen.melden({ art: 'sofort', text });
  }
  wurzel.classList.add('lc-app');
  wurzel.append(hinweis, haupt, statuszeile, ansagen.region);
  dialogEinrichten({
    wurzel,
    plus: channels.plus,
    anlegen: optionen.channelAnlegen,
    statusZeigen: statusMelden,
    signal: vorigeAnsichtAbmelden.signal,
  });
  einAusblendenBedienen({ schalter, region, haupt }, vorigeAnsichtAbmelden.signal);
  let letzterZustand = 'verbunden';
  channels.channelsZeigen([]);
  const verlauf = verlaufAnlegen({
    abrufen: optionen.verlaufAbrufen,
    schalter: (aktion, schluessel) => {
      senden.schalter(aktion, schluessel);
      eingabe.fokussieren();
    },
  });
  verlauf.kennungSetzen(teilnehmer.kennung);
  const senden = sendenEinrichten({
    verlauf,
    kennung: teilnehmer.kennung,
    abschicken: optionen.nachrichtSenden,
    wiederBeitreten: optionen.wiederBeitreten,
    netzAus: () => navigator.onLine === false,
    ansagen: ansagen.melden,
  });
  const eingabe = eingabeAnlegen({
    senden: senden.senden,
    entwurf: entwurfAnlegen({ speicher: window.sessionStorage }),
    ansagen: ansagen.melden,
  });
  const wieder = wiederverbindenEinrichten({ verlauf, senden, abrufen: optionen.verlaufNachholen }, vorigeAnsichtAbmelden.signal);
  fokusTastenBedienen({ verlauf, eingabe }, vorigeAnsichtAbmelden.signal);
  const beiAuswahl = (channel) => {
    gewaehlterChannel = channel;
    if (channel !== null) {
      einstellungen.channelGeaendert(channel);
    }
  };
  const beiOeffnen = (name) => {
    wieder.channelGewechselt();
    eingabe.channelSetzen(name);
    senden.channelGeoeffnet(name);
    ansagen.melden({ art: 'gewechselt' });
  };
  const auswahlFolgen = auswahlEinrichten({
    seitenleiste: channels, kopf, mitte, verlauf, eingabe, beiAuswahl, beiOeffnen,
  }, vorigeAnsichtAbmelden.signal);
  return {
    region: zeilen,
    channelsZeigen: auswahlFolgen,
    // Serverzeit aus einem Ereignis (RFC 3339 mit Versatz): ihr Datum ist das heutige des Servers.
    serverzeitZeigen: (jetzt) => verlauf.heuteSetzen(jetzt.slice(0, 10)),
    nachrichtEmpfangen: (ereignis) => senden.empfangen(ereignis),
    anzahlZeigen: (wieViele) => {
      anzahl.textContent = String(wieViele);
    },
    verbindungZeigen: (zustand) => {
      letzterZustand = verbindungZeigen({ hinweis, region, standUnbekannt }, letzterZustand, zustand);
      wieder.melden({ art: 'strom', zustand });
    },
    // Ein vollständiger Stand der Teilnehmer: am Anfang jedes Stroms und nach NeuerStand (ADR-0028 Regel 6).
    standEmpfangen: () => wieder.melden({ art: 'stand' }),
  };
}

// Getrennt heißt Strom unterbrochen oder Netz des Tabs aus (wiederverbinden.js): die Leiste oben im Verlauf sagt es,
// die Warteschlange ruht. Wieder verbunden → Ereignisse wieder annehmen, für den offenen Channel nachholen
// (nachhollauf.js), erst nach dem Gelingen 4 s melden und weitersenden — so stehen fremde Nachrichten aus der Trennung
// vor den eigenen gepufferten (ADR-0037 Regel 7). Scheitert das Nachholen, bleibt die Leiste „Verbindung verloren“.
function wiederverbindenEinrichten({ verlauf, senden, abrufen }, signal) {
  const leiste = element('div', { className: 'lc-leiste', hidden: true });
  verlauf.element.prepend(leiste);
  let zustand = VERBINDUNG_ANFANG;
  let anzeige = { getrennt: false, meldung: null, meldungSeitMs: 0 };
  let ausblenden = null;

  function leisteZeigen() {
    const inhalt = leisteInhalt(anzeige, Date.now());
    leiste.hidden = inhalt === null;
    leiste.textContent = '';
    if (inhalt !== null) {
      leiste.textContent = inhalt.text;
      leiste.classList.toggle('lc-leiste-fehler', inhalt.fehler);
    }
  }

  function meldungSetzen(text) {
    const STEHT_MS = 4000;
    anzeige = { getrennt: false, meldung: text, meldungSeitMs: Date.now() };
    leisteZeigen();
    clearTimeout(ausblenden);
    ausblenden = setTimeout(leisteZeigen, STEHT_MS);
  }

  const lauf = nachholLaufAnlegen({
    stand: () => verlauf.stand(),
    abrufen,
    annehmen: (nachrichten) => nachrichten.forEach((nachricht) => verlauf.nachrichtAnnehmen(nachricht, false)),
    neuOeffnen: (channel) => verlauf.oeffnen(channel),
    fertig: (ergebnis, mitLeiste) => {
      if (!mitLeiste) {
        return;
      }
      anzeige = { ...anzeige, getrennt: false };
      senden.verbunden();
      requestAnimationFrame(() => meldungNachDemEinfuegen(wiederVerbundenText(ergebnis)));
    },
  });

  // Das Nachgeholte fügt der Verlauf im nächsten Bild ein (ADR-0037 Regel 6); die Meldung kommt erst danach, damit
  // „✓ Wieder verbunden“ nie über einer Lücke steht — außer der Tab ist inzwischen wieder getrennt.
  function meldungNachDemEinfuegen(text) {
    if (anzeige.getrennt) {
      return;
    }
    meldungSetzen(text);
  }

  function melden(ereignis) {
    const ergebnis = verbindung(zustand, ereignis);
    zustand = ergebnis.zustand;
    if (ergebnis.aktion === 'trennen') {
      lauf.getrennt();
      anzeige = { ...anzeige, getrennt: true };
      leisteZeigen();
      senden.getrennt();
    }
    if (ergebnis.aktion === 'wiederverbinden') {
      senden.stromWieder();
      lauf.wiederverbunden();
    }
    if (ergebnis.aktion === 'nachholen') {
      lauf.neuerStand();
    }
  }

  window.addEventListener('offline', () => melden({ art: 'netz', online: false }), { signal });
  window.addEventListener('online', () => melden({ art: 'netz', online: true }), { signal });
  return { melden, channelGewechselt: lauf.channelGewechselt };
}

// Gebündelte Ansage (ADR-0038 Regel 10, ansage.js): eine Statusregion außerhalb des Verlaufs, visuell verborgen; ein
// Zeitgeber sagt Gesammeltes frühestens 10 s nach der letzten Ansage an. Gleicher Text zweimal wird neu gesetzt, damit
// er wieder angesagt wird.
function ansageEinrichten() {
  const region = element('p', { className: 'lc-unsichtbar lc-ansage' });
  region.setAttribute('role', 'status');
  region.setAttribute('aria-live', 'polite');
  let zustand = ANSAGE_LEER;
  let takt = null;

  const sagen = ansageSchreiberAnlegen({
    schreiben: (text) => {
      region.textContent = text;
    },
    imNaechstenBild: (rueckruf) => requestAnimationFrame(rueckruf),
  });

  function taktStellen() {
    const ABSTAND_MS = 10000;
    clearTimeout(takt);
    if (zustand.gesammelt === 0) {
      return;
    }
    const wartenMs = Math.max(0, zustand.letzteAnsageMs + ABSTAND_MS - Date.now());
    takt = setTimeout(() => melden({ art: 'takt' }), wartenMs);
  }

  function melden(ereignis) {
    const ergebnis = ansage(zustand, ereignis, Date.now());
    zustand = ergebnis.zustand;
    if (ergebnis.text !== null) {
      sagen(ergebnis.text);
    }
    taktStellen();
  }

  return { region, melden };
}

// `/` außerhalb von Textfeldern und `Esc` im Verlauf holen den Fokus in die Eingabe (ADR-0038 Regel 8, fokus.js).
function fokusTastenBedienen({ verlauf, eingabe }, signal) {
  document.addEventListener('keydown', (ereignis) => {
    if (!eingabe.element.isConnected || ereignis.defaultPrevented) {
      return;
    }
    if (fokusTaste(ereignis, fokusOrt(verlauf)) === 'ins-feld') {
      ereignis.preventDefault();
      eingabe.fokussieren();
    }
  }, { signal });
}

function fokusOrt(verlauf) {
  const aktiv = document.activeElement;
  if (document.querySelector('dialog[open]') !== null) {
    return 'dialog';
  }
  const textfeld = aktiv !== null && (aktiv.matches('input, textarea, select') || aktiv.isContentEditable);
  if (textfeld) {
    return 'textfeld';
  }
  if (verlauf.element.contains(aktiv)) {
    return 'verlauf';
  }
  return 'sonst';
}

// Liefert channelsZeigen(liste): die Liste ist erst mit dem ersten Stand bekannt — bis dahin bleibt ein
// gewählter Name ohne Urteil, damit „gibt es nicht“ nie vor dem Stand erscheint.
function auswahlEinrichten({ seitenleiste, kopf, mitte, verlauf, eingabe, beiAuswahl, beiOeffnen }, signal) {
  const adresse = startadresse(window.location.hash, window.sessionStorage.getItem(MERKER));
  if (adresse !== null) {
    window.history.replaceState(null, '', adresse);
  }
  let liste = null;
  let offen = null;
  function folgen() {
    const namen = (liste || []).map((channel) => channel.name);
    const gewaehlt = auswahl(window.location.hash, namen);
    auswahlMerken(window.sessionStorage, gewaehlt);
    let bekannt = null;
    if (gewaehlt !== null && gewaehlt.bekannt) {
      bekannt = liste.find((channel) => channel.name === gewaehlt.name);
    }
    seitenleiste.auswahlZeigen(bekannt && bekannt.name);
    kopf.ortZeigen(bekannt);
    beiAuswahl(bekannt);
    if (bekannt === null) {
      offen = null;
      verlaufSchliessen(verlauf, eingabe, seitenleiste);
      mitteZeigen(mitte, gewaehlt, liste !== null);
      return;
    }
    if (offen !== bekannt.name) {
      offen = bekannt.name;
      if (verlauf.element.parentNode !== mitte) {
        mitte.replaceChildren(verlauf.element, eingabe.element);
      }
      beiOeffnen(bekannt.name);
      verlauf.oeffnen(bekannt.name);
    }
  }
  window.addEventListener('hashchange', folgen, { signal });
  // Die Wahl des schon gewählten Channels ändert die Adresse nicht, es kommt kein hashchange (R00007, US-13).
  seitenleiste.nav.addEventListener('click', (ereignis) => {
    const verweis = ereignis.target.closest('a[data-channel]');
    if (verweis !== null && verweis.dataset.channel === offen) {
      verlauf.erneutVersuchen();
    }
  }, { signal });
  // Alt ↓ / Alt ↑ wählt wie ein Klick über die Adresse; der Fokus bleibt, wo er ist (ADR-0027 Regel 4).
  document.addEventListener('keydown', (ereignis) => {
    const richtung = kuerzelRichtung(ereignis);
    const dialogOffen = document.querySelector('dialog[open]') !== null;
    if (richtung === null || dialogOffen || liste === null) {
      return;
    }
    ereignis.preventDefault();
    const namen = liste.map((channel) => channel.name);
    const nachbar = nachbarChannel(namen, offen, richtung);
    if (nachbar !== null) {
      window.location.hash = `#/c/${nachbar}`;
    }
  }, { signal });
  folgen();
  return (neueListe) => {
    liste = neueListe;
    seitenleiste.channelsZeigen(neueListe);
    folgen();
  };
}

// Liegt der Fokus im Verlauf oder in der Eingabe, die jetzt gehen, kommt er auf den Schalter „Channel anlegen“ —
// nie ins Nichts.
function verlaufSchliessen(verlauf, eingabe, seitenleiste) {
  const fokusImVerlauf = verlauf.element.contains(document.activeElement) || eingabe.element.contains(document.activeElement);
  verlauf.schliessen();
  verlauf.element.remove();
  eingabe.element.remove();
  if (fokusImVerlauf) {
    seitenleiste.plus.focus();
  }
}

function mitteZeigen(mitte, gewaehlt, listeBekannt) {
  if (gewaehlt === null) {
    mitte.replaceChildren(element('p', { className: 'lc-mitte-hinweis', textContent: 'Links einen Channel wählen.' }));
    return;
  }
  if (!listeBekannt) {
    mitte.replaceChildren();
    return;
  }
  mitte.replaceChildren(element('p', { className: 'lc-mitte-hinweis', textContent: `# ${gewaehlt.name} gibt es nicht.` }));
}

// Die Meldung steht einige Sekunden und geht dann von selbst; eine neue ersetzt sie.
const STATUS_STEHT_MS = 6000;
let statusAusblenden = null;

function statusZeigen(statuszeile, text) {
  clearTimeout(statusAusblenden);
  statuszeile.textContent = text;
  statuszeile.hidden = false;
  statusAusblenden = setTimeout(() => {
    statuszeile.hidden = true;
    statuszeile.textContent = '';
  }, STATUS_STEHT_MS);
}

// Unter 720 px legt sich die eingeblendete Liste als Fläche über die Mitte (Gestaltungsfrage 1): beim Öffnen
// geht der Fokus in die Liste, Esc und der Schalter schließen sie, der Fokus kehrt auf den Schalter zurück.
const UEBERLAGERT_UNTER_PX = 720;

// Der Wunsch des Menschen gilt bis zum Neuladen; ohne ihn folgt die Liste der Fensterbreite. Liegt der Fokus
// beim Ausblenden in der Liste — auch wenn sie durch Verkleinern einklappt —, geht er auf den Schalter.
function einAusblendenBedienen({ schalter, region, haupt }, signal) {
  let wunsch = null;
  function anwenden() {
    const sichtbar = teilnehmerSichtbar(window.innerWidth, wunsch);
    if (!sichtbar && region.contains(document.activeElement)) {
      schalter.focus();
    }
    region.hidden = !sichtbar;
    haupt.classList.toggle('ohne-teilnehmer', !sichtbar);
    schalter.setAttribute('aria-expanded', String(sichtbar));
  }
  function umschalten() {
    const warSichtbar = !region.hidden;
    const ueberlagert = window.innerWidth < UEBERLAGERT_UNTER_PX;
    wunsch = 'ein';
    if (warSichtbar) {
      wunsch = 'aus';
    }
    anwenden();
    if (warSichtbar && ueberlagert) {
      schalter.focus();
    }
    if (!warSichtbar && ueberlagert) {
      region.focus();
    }
  }
  schalter.addEventListener('click', umschalten, { signal });
  document.addEventListener('keydown', (ereignis) => {
    const strgPunkt = (ereignis.ctrlKey || ereignis.metaKey) && ereignis.key === '.';
    const ueberlagerteListeOffen = !region.hidden && window.innerWidth < UEBERLAGERT_UNTER_PX;
    // Ein offener Dialog liegt über der Liste — sein Esc gehört ihm.
    const dialogOffen = document.querySelector('dialog[open]') !== null;
    const schliessen = ereignis.key === 'Escape' && ueberlagerteListeOffen && !dialogOffen;
    if (strgPunkt || schliessen) {
      ereignis.preventDefault();
      umschalten();
    }
  }, { signal });
  window.matchMedia(`(min-width: ${SICHTBAR_AB_PX}px)`).addEventListener('change', anwenden, { signal });
  anwenden();
}

// Designsystem README: unter 1100 px klappt die Teilnehmerliste ein, damit der Chat neben die IDE passt; hat der
// Mensch sie ein- oder ausgeblendet (wunsch 'ein' | 'aus'), gilt das bis zum Neuladen, sonst die Breite.
// Die Grenze steht nur hier (die Media-Query wird aus ihr gebildet, J06).
const SICHTBAR_AB_PX = 1100;

export function teilnehmerSichtbar(fensterbreite, wunsch) {
  if (wunsch === 'ein') {
    return true;
  }
  if (wunsch === 'aus') {
    return false;
  }
  return fensterbreite >= SICHTBAR_AB_PX;
}

// Der Alarmtext wird nur beim Wechsel gesetzt, damit ein Screenreader ihn nicht bei jedem Versuch neu ansagt.
// Solange die Verbindung fehlt, ist die Liste als „Stand unbekannt“ markiert: gedämpft, mit einem Satz, und
// kein ● gilt mehr als sicher online. Liefert den nun gültigen Zustand.
function verbindungZeigen({ hinweis, region, standUnbekannt }, letzterZustand, zustand) {
  if (zustand === 'unterbrochen' && letzterZustand !== 'unterbrochen') {
    hinweis.textContent = UNTERBROCHEN;
    hinweis.hidden = false;
    standUnbekannt.textContent = STAND_UNBEKANNT;
    standUnbekannt.hidden = false;
    region.classList.add('lc-unbekannt');
    onlineBenennen(region, true);
    return 'unterbrochen';
  }
  if (zustand === 'verbunden' && letzterZustand !== 'verbunden') {
    hinweis.textContent = '';
    hinweis.hidden = true;
    standUnbekannt.hidden = true;
    region.classList.remove('lc-unbekannt');
    onlineBenennen(region, false);
    return 'verbunden';
  }
  return letzterZustand;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
