// Verteiler im SharedWorker (B0094, ADR-0015 Regel 2 und 4): alle Tabs eines Browsers teilen höchstens einen
// Strom. Ereignisse und Zustände gehen an jeden Tab; ein neuer Tab bekommt sofort den zusammengeführten Stand.
// Lehnt der Server das Token ab, soll genau ein Tab still beitreten — bleibt sein neues Token aus, übernimmt
// der nächste. Geht der letzte Tab, endet der Strom.
//
// Tab → Worker: { art: 'token', token } · { art: 'tschuess' }
// Worker → Tab: { art: 'ereignis', ereignis } · { art: 'zustand', zustand, beitreten, token } (token: das
// abgelehnte, nur bei 'abgelehnt')
import { LEER, zusammenfuehren } from './teilnehmerliste.js';

const UEBERNAHME_NACH_MS = 5000;

export function verteilerAnlegen({
  halten,
  warten = (ms, rueckruf) => setTimeout(rueckruf, ms),
  abbrechen = (zeitgeber) => clearTimeout(zeitgeber),
}) {
  let ports = [];
  let halter = null;
  let liste = null;
  let zustand = null;
  let abgelehntesToken = null;
  let uebernahme = null;

  function verbinden(port) {
    ports.push(port);
    port.onmessage = (nachricht) => empfangen(port, nachricht.data);
    if (liste !== null) {
      port.postMessage({ art: 'ereignis', ereignis: standAus(liste) });
    }
    if (zustand === 'verbunden' || zustand === 'unterbrochen') {
      port.postMessage({ art: 'zustand', zustand, beitreten: false });
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
    zustand = null;
  }

  function beiEreignis(ereignis) {
    liste = zusammenfuehren(liste || LEER, ereignis);
    allen({ art: 'ereignis', ereignis });
  }

  function beiZustand(neuerZustand, token) {
    zustand = neuerZustand;
    if (neuerZustand === 'abgelehnt') {
      abgelehntesToken = token;
      beitretenLassen();
      return;
    }
    allen({ art: 'zustand', zustand: neuerZustand, beitreten: false });
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
