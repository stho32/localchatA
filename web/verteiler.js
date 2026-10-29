// Verteiler im SharedWorker (B0094, ADR-0015 Regel 2 und 4): alle Tabs eines Browsers teilen höchstens einen
// Strom. Ereignisse und Zustände gehen an jeden Tab; ein neuer Tab bekommt sofort den zusammengeführten Stand.
// Lehnt der Server das Token ab, soll genau ein Tab still beitreten — bleibt sein neues Token aus, übernimmt
// der nächste. Geht der letzte Tab, endet der Strom. Neben den Teilnehmern merkt er sich die Channels
// (ADR-0023 Regel 6) und gibt einem neuen Tab beide Stände.
//
// Tab → Worker: { art: 'token', token } · { art: 'tschuess' }
// Worker → Tab: { art: 'ereignis', ereignis, versatz } · { art: 'zustand', zustand, beitreten, token, naechsterVersuchUm }
// (token: das abgelehnte, nur bei 'abgelehnt'; naechsterVersuchUm nur bei 'unterbrochen', sonst null). Der Versatz (Serverzeit − Uhr des Rechners, B0110) geht mit jedem Ereignis
// und mit dem Stand für einen neuen Tab hinaus — auch der Herzschlag wird dafür weitergereicht.
import { LEER, zusammenfuehren } from './teilnehmerliste.js';
import { LEER as KEINE_CHANNELS, channelsZusammenfuehren } from './channels.js';

const UEBERNAHME_NACH_MS = 5000;

export function verteilerAnlegen({
  halten,
  warten = (ms, rueckruf) => setTimeout(rueckruf, ms),
  abbrechen = (zeitgeber) => clearTimeout(zeitgeber),
}) {
  let ports = [];
  let halter = null;
  let liste = null;
  let channels = null;
  let versatz = 0;
  let zustand = null;
  let naechsterVersuchUm = null;
  let abgelehntesToken = null;
  let uebernahme = null;

  function verbinden(port) {
    ports.push(port);
    port.onmessage = (nachricht) => empfangen(port, nachricht.data);
    if (liste !== null) {
      port.postMessage({ art: 'ereignis', ereignis: standAus(liste), versatz });
    }
    if (channels !== null) {
      port.postMessage({ art: 'ereignis', ereignis: { art: 'channels', stand: channels.stand, liste: channels.channels }, versatz });
    }
    if (zustand === 'verbunden' || zustand === 'unterbrochen') {
      port.postMessage({ art: 'zustand', zustand, beitreten: false, naechsterVersuchUm });
    }
  }

  function empfangen(port, daten) {
    if (daten.art === 'token') {
      tokenUebernehmen(daten.token);
    }
    if (daten.art === 'tschuess') {
      gehen(port);
    }
  }

  function tokenUebernehmen(token) {
    if (!token || token === abgelehntesToken) {
      return;
    }
    uebernahmeBeenden();
    if (halter === null) {
      halter = halten({ token, beiEreignis, beiZustand });
      return;
    }
    halter.tokenSetzen(token);
  }

  function gehen(port) {
    ports = ports.filter((verbunden) => verbunden !== port);
    if (ports.length > 0 || halter === null) {
      return;
    }
    halter.beenden();
    uebernahmeBeenden();
    halter = null;
    liste = null;
    channels = null;
    zustand = null;
  }

  function beiEreignis(ereignis, neuerVersatz) {
    versatz = neuerVersatz;
    liste = zusammenfuehren(liste || LEER, ereignis);
    if (ereignis.art === 'channels' || channels !== null) {
      channels = channelsZusammenfuehren(channels || KEINE_CHANNELS, ereignis);
    }
    allen({ art: 'ereignis', ereignis, versatz });
  }

  // `unterbrochen` trägt den Zeitpunkt des nächsten Versuchs; er wird gemerkt und neuen Tabs nachgereicht.
  function beiZustand(neuerZustand, token, zeitpunkt) {
    zustand = neuerZustand;
    naechsterVersuchUm = null;
    if (neuerZustand === 'abgelehnt') {
      abgelehntesToken = token;
      beitretenLassen();
      return;
    }
    if (neuerZustand === 'unterbrochen') {
      naechsterVersuchUm = zeitpunkt;
    }
    allen({ art: 'zustand', zustand: neuerZustand, beitreten: false, naechsterVersuchUm });
  }

  function beitretenLassen() {
    const zustaendig = ports[0];
    ports.forEach((port) => port.postMessage({
      art: 'zustand', zustand: 'abgelehnt', beitreten: port === zustaendig, token: abgelehntesToken,
    }));
    uebernahme = warten(UEBERNAHME_NACH_MS, () => {
      ports = [...ports.slice(1), ...ports.slice(0, 1)];
      beitretenLassen();
    });
  }

  function uebernahmeBeenden() {
    if (uebernahme !== null) {
      abbrechen(uebernahme);
      uebernahme = null;
    }
  }

  function allen(nachricht) {
    ports.forEach((port) => port.postMessage(nachricht));
  }

  return { verbinden };
}

// Der zusammengeführte Stand als Ereignis „teilnehmer“ mit der höchsten Nummer, die der Worker gesehen hat —
// damit ein älterer Wechsel, der danach noch ankommt, im neuen Tab nichts überschreibt.
function standAus(liste) {
  const nummern = [liste.stand, ...liste.eintraege.map((eintrag) => eintrag.stand), ...Object.values(liste.entfernt)];
  return { art: 'teilnehmer', stand: Math.max(...nummern), liste: liste.eintraege };
}
