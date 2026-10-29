// Hauptansicht (Designsystem: Bildschirm 4.2, Baustein Hauptansicht): Kopfzeile, links die Seitenleiste
// „Channels“, Mitte und rechts die Region „Teilnehmer“. Tab-Reihenfolge: Kopfzeile → Seitenleiste → Mitte →
// Teilnehmer (ADR-0024 Regel 1). Eine getrennte Verbindung zeigt genau eine Warnung: die Leiste oben über der Mitte
// (R00025 F0084, ADR-0038 Ergänzung Karte #36). `Strg .` und der Schalter
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
import { wiederVerbundenText, leisteInhalt, geltenderZeitpunkt, leisteFuellen } from './verbindungsleiste.js';
import { ANSAGE_LEER, ansage, ansageSchreiberAnlegen } from './ansage.js';

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
  wurzel.append(haupt, statuszeile, ansagen.region);
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
    fokusRueckgaengig: (schluessel) => senden.rueckgaengigFokussiert(schluessel),
    fokusVerloren: () => eingabe.fokussieren(),
    schalter: (aktion, schluessel) => {
      if (aktion === 'rueckgaengig') {
        verlauf.nachRueckgaengigFokussieren(schluessel);
        senden.schalter(aktion, schluessel);
        return;
      }
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
    zustellung: optionen.zustellung,
    netzAus: () => navigator.onLine === false,
    ansagen: ansagen.melden,
  });
  const eingabe = eingabeAnlegen({
    senden: senden.senden,
    entwurf: entwurfAnlegen({ speicher: window.sessionStorage }),
    ansagen: ansagen.melden,
  });
  const wieder = wiederverbindenEinrichten({
    mitte, verlauf, senden, abrufen: optionen.verlaufNachholen, ansagen: ansagen.melden,
  }, vorigeAnsichtAbmelden.signal);
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
    seitenleiste: channels, kopf, mitte, verlauf, eingabe, beiAuswahl, beiOeffnen, erneutWaehlen: wieder.erneutWaehlen,
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
    verbindungZeigen: (zustand, naechsterVersuchUm) => {
      letzterZustand = verbindungZeigen({ region, standUnbekannt }, letzterZustand, zustand);
      wieder.melden({ art: 'strom', zustand, naechsterVersuchUm });
    },
    // Ein vollständiger Stand der Teilnehmer: am Anfang jedes Stroms und nach NeuerStand (ADR-0028 Regel 6).
    standEmpfangen: () => wieder.melden({ art: 'stand' }),
    // Vor dem Wechsel zum Beitreten-Bildschirm.
    entwurfSichern: () => eingabe.entwurfSichern(),
  };
}

// Getrennt heißt Strom unterbrochen oder Netz des Tabs aus (wiederverbinden.js): die Leiste oben im Verlauf sagt es,
// die Warteschlange ruht. Wieder verbunden → Ereignisse wieder annehmen, für den offenen Channel nachholen — oder,
// war er nie geladen, ihn laden (nachhollauf.js) —, erst nach dem Gelingen 4 s melden und weitersenden — so stehen fremde Nachrichten aus der Trennung
// vor den eigenen gepufferten (ADR-0037 Regel 7). Scheitert das Nachholen, bleibt die Leiste „Verbindung verloren“.
// Die Leiste liegt über der Mitte, nicht im Verlauf — mit und ohne offenen Channel, einmal je Hauptansicht; Ein- und
// Ausblenden verschiebt nichts (ADR-0038 Ergänzung Karte #36).
function wiederverbindenEinrichten({ mitte, verlauf, senden, abrufen, ansagen }, signal) {
  const leiste = element('div', { className: 'lc-leiste', hidden: true });
  mitte.prepend(leiste);
  let zustand = VERBINDUNG_ANFANG;
  let anzeige = { getrennt: false, meldung: null, meldungSeitMs: 0 };
  let ausblenden = null;
  // Zeitpunkte des nächsten Versuchs (ms seit der Epoche): vom Strom bzw. vom gescheiterten Nachholen.
  const naechsterVersuch = { strom: null, nachholen: null };
  let countdown = null;
  // Läuft gerade ein Versuch des Laden- oder Nachhol-Laufs? Die Leiste sagt es nur bei stehendem Strom (R00037 Regel 2).
  let versuchLaeuft = false;

  function leisteZeigen() {
    const naechsterVersuchUm = geltenderZeitpunkt({ stromUnterbrochen: !zustand.strom, ...naechsterVersuch });
    const inhalt = leisteInhalt({ ...anzeige, naechsterVersuchUm, versuchLaeuft: versuchLaeuft && zustand.strom }, Date.now());
    leiste.hidden = inhalt === null;
    leiste.textContent = '';
    if (inhalt !== null) {
      leisteFuellen(leiste, inhalt.text);
      leiste.classList.toggle('lc-leiste-fehler', inhalt.fehler);
    }
    countdownStellen();
  }

  // Der Countdown zählt im Sekundentakt nur, solange die Warnung steht — ein Zeitgeber für die eine Leiste, solange die
  // Hauptansicht steht.
  function countdownStellen() {
    const SEKUNDE_MS = 1000;
    const zaehlen = anzeige.getrennt && !signal.aborted;
    if (zaehlen && countdown === null) {
      countdown = setInterval(leisteZeigen, SEKUNDE_MS);
    }
    if (!zaehlen && countdown !== null) {
      clearInterval(countdown);
      countdown = null;
    }
  }
  signal.addEventListener('abort', countdownStellen);

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
    annehmen: (nachrichten) => verlauf.nachgeholtAnnehmen(nachrichten),
    neuOeffnen: (channel) => verlauf.oeffnen(channel),
    laden: (channel, signal) => verlauf.oeffnen(channel, signal),
    versuch: (laeuft) => {
      versuchLaeuft = laeuft;
      verlauf.hinweisNachziehen();
      leisteZeigen();
    },
    gescheitert: (naechsterVersuchUm) => {
      naechsterVersuch.nachholen = naechsterVersuchUm;
      leisteZeigen();
    },
    fertig: (ergebnis, mitLeiste) => {
      if (!mitLeiste) {
        return;
      }
      anzeige = { ...anzeige, getrennt: false };
      senden.verbunden();
      requestAnimationFrame(() => meldungNachDemEinfuegen(wiederVerbundenText(ergebnis)));
    },
  });

  // Lädt der Tab von selbst? Strom getrennt (das Wiederverbinden startet den Laden-Lauf) oder ein Laden-Lauf läuft: dann
  // sagt der Hinweis „wird geladen“, sonst hilft nur die neue Wahl des Channels (R00037 Regel 1).
  verlauf.selbstversuchendSetzen(() => !zustand.strom || !zustand.netz || lauf.laedt());

  // Das Nachgeholte fügt der Verlauf im nächsten Bild ein (ADR-0037 Regel 6); die Meldung kommt erst danach, damit
  // „✓ Wieder verbunden“ nie über einer Lücke steht — außer der Tab ist inzwischen wieder getrennt.
  function meldungNachDemEinfuegen(text) {
    if (anzeige.getrennt) {
      return;
    }
    meldungSetzen(text);
    ansagen({ art: 'wiederverbunden', text: text.replace(/^✓ /, '') });
  }

  function melden(ereignis) {
    const ergebnis = verbindung(zustand, ereignis);
    zustand = ergebnis.zustand;
    if (ereignis.art === 'strom') {
      naechsterVersuch.strom = zeitpunktDesStroms(ereignis);
    }
    if (ergebnis.aktion === 'trennen') {
      lauf.getrennt();
      naechsterVersuch.nachholen = null;
      anzeige = { ...anzeige, getrennt: true };
      senden.getrennt();
      ansagen({ art: 'getrennt' });
    }
    if (anzeige.getrennt) {
      leisteZeigen();
    }
    if (ergebnis.aktion === 'wiederverbinden') {
      senden.stromWieder();
      lauf.wiederverbunden();
    }
    if (ergebnis.aktion === 'nachholen') {
      lauf.neuerStand();
    }
    verlauf.hinweisNachziehen();
  }

  window.addEventListener('offline', () => melden({ art: 'netz', online: false }), { signal });
  window.addEventListener('online', () => melden({ art: 'netz', online: true }), { signal });
  // Eine neue Wahl des offenen Channels: läuft ein Laden-Lauf, zieht sie dessen Versuch vor — kein zweites Öffnen
  // daneben (ADR-0037 7i); sonst versucht der Verlauf es nach „Verlauf nicht geladen“ erneut (R00007).
  function erneutWaehlen() {
    if (lauf.laedt()) {
      lauf.jetztVersuchen();
      return;
    }
    verlauf.erneutVersuchen();
  }

  return { melden, channelGewechselt: lauf.channelGewechselt, erneutWaehlen };
}

function zeitpunktDesStroms({ zustand, naechsterVersuchUm }) {
  if (zustand === 'unterbrochen' && typeof naechsterVersuchUm === 'number') {
    return naechsterVersuchUm;
  }
  return null;
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
function auswahlEinrichten({ seitenleiste, kopf, mitte, verlauf, eingabe, beiAuswahl, beiOeffnen, erneutWaehlen }, signal) {
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
        mitteFuellen(mitte, verlauf.element, eingabe.element);
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
      erneutWaehlen();
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
    mitteFuellen(mitte, element('p', { className: 'lc-mitte-hinweis', textContent: 'Links einen Channel wählen.' }));
    return;
  }
  if (!listeBekannt) {
    mitteFuellen(mitte);
    return;
  }
  mitteFuellen(mitte, element('p', { className: 'lc-mitte-hinweis', textContent: `# ${gewaehlt.name} gibt es nicht.` }));
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

// Der Alarmtext wird nur beim Wechsel gesetzt, damit ein Bildschirmleser ihn nicht bei jedem Versuch neu ansagt.
// Solange die Verbindung fehlt, ist die Liste als „Stand unbekannt“ markiert: gedämpft, mit einem Satz, und
// kein ● gilt mehr als sicher online. Liefert den nun gültigen Zustand.
function verbindungZeigen({ region, standUnbekannt }, letzterZustand, zustand) {
  if (zustand === 'unterbrochen' && letzterZustand !== 'unterbrochen') {
    standUnbekannt.textContent = STAND_UNBEKANNT;
    standUnbekannt.hidden = false;
    region.classList.add('lc-unbekannt');
    onlineBenennen(region, true);
    return 'unterbrochen';
  }
  if (zustand === 'verbunden' && letzterZustand !== 'verbunden') {
    standUnbekannt.hidden = true;
    region.classList.remove('lc-unbekannt');
    onlineBenennen(region, false);
    return 'verbunden';
  }
  return letzterZustand;
}

// Ersetzt den Inhalt der Mitte; die Leiste der Verbindung bleibt (sie gehört der Hauptansicht, nicht dem Channel).
function mitteFuellen(mitte, ...kinder) {
  const leiste = [...mitte.children].filter((kind) => kind.classList.contains('lc-leiste'));
  mitte.replaceChildren(...leiste, ...kinder);
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
