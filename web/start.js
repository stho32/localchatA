// Oberfläche verdrahten (A00005): beim Laden die Sitzung prüfen, dann Beitreten-Bildschirm oder Hauptansicht
// mit Teilnehmerliste und Live-Strom.
import { rufen, verlaufAbrufen, verlaufNachholen, fristSetzen, nachrichtSenden } from './api.js';
import * as sitzung from './sitzung.js';
import * as beitretenBildschirm from './beitreten.js';
import * as hauptansicht from './hauptansicht.js';
import * as teilnehmerliste from './teilnehmerliste.js';
import { LEER as KEINE_CHANNELS, channelsZusammenfuehren } from './channels.js';
import { verbinden } from './live.js';
import { sekundentaktAnlegen } from './sekundentakt.js';
import { wiederbeitrittAnlegen } from './wiederbeitritt.js';
import { LEER as KEINE_ZUSTELLUNG, zustellen, ungesendete } from './zustellung.js';

const api = {
  ich: (token) => rufen('/api/ich', { token }),
  beitreten: (name, token) => rufen('/api/beitreten', { koerper: { name, art: 'mensch' }, token }),
};
const speicher = window.localStorage;
const wurzel = document.getElementById('app');
let laufenderStrom = null;
const takten = sekundentaktAnlegen();
// Die Zustellung gehört dem Tab, nicht der Hauptansicht (R00025 F0088): ungesendete Nachrichten überstehen den
// Beitreten-Bildschirm; ein Neuladen verwirft sie.
const zustellung = { zustand: KEINE_ZUSTELLUNG };

farbschemaFolgen();
zeigen(await sitzungBeimLaden());

// Nach einem Beitritt per Eingabe wandert der Fokus auf die eigene Kennung, sonst läge er nach dem Abbau
// des Formulars auf dem Dokument.
function zeigen(zustand, nachEingabe = false) {
  if (zustand.art === 'kopfzeile') {
    hauptansichtZeigen(zustand.teilnehmer, nachEingabe);
    return;
  }
  beitretenBildschirm.zeichnen(wurzel, zustand, {
    beitreten: async (name) => zeigen(await sitzung.beitreten(name, speicher, api), true),
    wartende: ungesendete(zustellung.zustand),
  });
}

// Die Channels der Seitenleiste folgen demselben Strom (Stand und Wechsel nach `stand`, ADR-0023 Regel 6).
// Die Liste folgt dem Strom — im SharedWorker für alle Tabs, sonst im Tab: Stand und Wechsel werden nach
// `stand` zusammengeführt, geordnet und mit dem Versatz zur Serverzeit gezeichnet, Zeilen „gleich weg“ zählt der
// Sekundentakt herunter; der Herzschlag erneuert nur den Versatz. Lehnt der Strom (beim zuständigen Tab) oder das Senden
// das Token ab (Neustart, Gnadenfrist), führt ein Weg zum Wiederbeitritt (wiederbeitritt.js, R00025 F0088).
function hauptansichtZeigen(teilnehmer, nachEingabe) {
  const wiederBeitreten = wiederbeitrittAnlegen({
    speicher,
    api,
    sperre: sitzungSperren,
    gelungen: (token) => {
      if (laufenderStrom !== null) {
        laufenderStrom.tokenSetzen(token);
      }
    },
    gescheitert: (zustand) => {
      zustellung.zustand = zustellen(zustellung.zustand, { art: 'beitrittGescheitert' }).zustand;
      ansicht.entwurfSichern();
      stromBeenden();
      zeigen(zustand);
    },
  });
  const ansicht = hauptansicht.zeichnen(wurzel, teilnehmer, {
    kennungFokussieren: nachEingabe,
    channelAnlegen: (name, beschreibung) => rufen('/api/channels', {
      koerper: { name, beschreibung }, token: speicher.getItem(sitzung.SITZUNG),
    }),
    verlaufAbrufen: (name, cursor, signal) => verlaufAbrufen(name, cursor, { token: speicher.getItem(sitzung.SITZUNG), signal }),
    verlaufNachholen: (name, cursor, signal, n) => verlaufNachholen(name, cursor, { token: speicher.getItem(sitzung.SITZUNG), signal, n }),
    fristSetzen: (name, tage, bestaetigt) => fristSetzen(name, tage, bestaetigt, { token: speicher.getItem(sitzung.SITZUNG) }),
    nachrichtSenden: async (channel, text, schluessel) => {
      const token = speicher.getItem(sitzung.SITZUNG);
      return { ...await nachrichtSenden(channel, text, schluessel, { token }), token };
    },
    wiederBeitreten,
    zustellung,
  });
  let liste = teilnehmerliste.LEER;
  let channels = KEINE_CHANNELS;
  let versatz = 0;
  stromBeenden();
  laufenderStrom = verbinden({
    token: speicher.getItem(sitzung.SITZUNG),
    version: document.querySelector('meta[name="lc-version"]').content,
    beiEreignis: (ereignis, neuerVersatz) => {
      versatz = neuerVersatz;
      if (typeof ereignis.jetzt === 'string') {
        ansicht.serverzeitZeigen(ereignis.jetzt);
      }
      if (ereignis.art === 'herzschlag') {
        return;
      }
      if (ereignis.art === 'nachricht') {
        ansicht.nachrichtEmpfangen(ereignis);
        return;
      }
      if (ereignis.art === 'teilnehmer') {
        ansicht.standEmpfangen();
      }
      if (ereignis.art === 'channels' || ereignis.art === 'channel') {
        channels = channelsZusammenfuehren(channels, ereignis);
        ansicht.channelsZeigen(channels.channels);
        return;
      }
      liste = teilnehmerliste.zusammenfuehren(liste, ereignis);
      const gruppen = teilnehmerliste.ordnen(liste, teilnehmer.kennung);
      teilnehmerliste.zeichnen(ansicht.region, gruppen, { versatz });
      takten({
        zeilen: alleZeilen(gruppen),
        uhr: () => ({ versatz, jetzt: Date.now() }),
        zeileNeuZeichnen: (zeile) => teilnehmerliste.zeileNeuZeichnen(ansicht.region, zeile, { versatz }),
      });
      ansicht.anzahlZeigen(liste.eintraege.length);
    },
    beiZustand: (zustand, { beitreten, token, naechsterVersuchUm }) => {
      ansicht.verbindungZeigen(zustand, naechsterVersuchUm);
      if (zustand === 'abgelehnt' && beitreten) {
        wiederBeitreten(token);
      }
    },
  });
}

// Ohne GET /api/ich: dessen 401 stünde als Konsolenfehler (AK F0013). Mit Web Lock tritt je Browser nur ein Tab
// zugleich bei; ohne (LAN-Adresse) wartet ein Tab nach dem 409 auf das Token des Nachbar-Tabs.
function sitzungSperren(arbeit) {
  if (navigator.locks === undefined) {
    return arbeit({ neuesToken: aufTokenEinesAnderenTabsWarten });
  }
  return navigator.locks.request(sitzung.SITZUNG, () => arbeit());
}

function alleZeilen(gruppen) {
  return [...gruppen.menschen, ...gruppen.agenten.flatMap((gruppe) => gruppe.zeilen)];
}

function stromBeenden() {
  takten({ zeilen: [], uhr: () => ({ versatz: 0, jetzt: Date.now() }), zeileNeuZeichnen: () => {} });
  if (laufenderStrom !== null) {
    laufenderStrom.beenden();
    laufenderStrom = null;
  }
}

// Laden mehrere Tabs gleichzeitig, tritt nur einer still bei; die anderen warten und lesen dessen Token.
// Web Locks gibt es nur in sicheren Kontexten (localhost, 127.0.0.1) — über eine LAN-Adresse wartet ein Tab
// nach dem 409 stattdessen auf das storage-Ereignis des Tabs, der zuerst beigetreten ist.
function sitzungBeimLaden() {
  if (navigator.locks === undefined) {
    return sitzung.beimLaden(speicher, api, { neuesToken: aufTokenEinesAnderenTabsWarten });
  }
  return navigator.locks.request(sitzung.SITZUNG, () => sitzung.beimLaden(speicher, api));
}

function aufTokenEinesAnderenTabsWarten() {
  return new Promise((gefunden) => {
    const wartezeitInMs = 2000;
    const aufgeben = setTimeout(() => fertig(null), wartezeitInMs);
    function beimSpeichern(ereignis) {
      if (ereignis.key === sitzung.SITZUNG && ereignis.newValue) {
        fertig(ereignis.newValue);
      }
    }
    function fertig(token) {
      clearTimeout(aufgeben);
      window.removeEventListener('storage', beimSpeichern);
      gefunden(token);
    }
    window.addEventListener('storage', beimSpeichern);
  });
}

function farbschemaFolgen() {
  const dunkel = window.matchMedia('(prefers-color-scheme: dark)');
  const anwenden = () => {
    let thema = 'light';
    if (dunkel.matches) {
      thema = 'dark';
    }
    document.documentElement.dataset.theme = thema;
  };
  anwenden();
  dunkel.addEventListener('change', anwenden);
}
