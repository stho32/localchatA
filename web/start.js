// Oberfläche verdrahten (A00005): beim Laden die Sitzung prüfen, dann Beitreten-Bildschirm oder Hauptansicht
// mit Teilnehmerliste und Live-Strom.
import { rufen } from './api.js';
import * as sitzung from './sitzung.js';
import * as beitretenBildschirm from './beitreten.js';
import * as hauptansicht from './hauptansicht.js';
import * as teilnehmerliste from './teilnehmerliste.js';
import { verbinden } from './live.js';

const api = {
  ich: (token) => rufen('/api/ich', { token }),
  beitreten: (name, token) => rufen('/api/beitreten', { koerper: { name, art: 'mensch' }, token }),
};
const speicher = window.localStorage;
const wurzel = document.getElementById('app');
let laufenderStrom = null;
let tokenDesStroms = null;

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
  });
}

// Die Liste folgt dem Strom — im SharedWorker für alle Tabs, sonst im Tab: Stand und Wechsel werden nach
// `stand` zusammengeführt, geordnet und gezeichnet. Lehnt der Strom das Token ab (Neustart, Gnadenfrist),
// tritt der zuständige Tab still wieder bei wie beim Laden und gibt das neue Token weiter.
function hauptansichtZeigen(teilnehmer, nachEingabe) {
  const ansicht = hauptansicht.zeichnen(wurzel, teilnehmer, { kennungFokussieren: nachEingabe });
  let liste = teilnehmerliste.LEER;
  stromBeenden();
  tokenDesStroms = speicher.getItem(sitzung.SITZUNG);
  laufenderStrom = verbinden({
    token: tokenDesStroms,
    version: document.querySelector('meta[name="lc-version"]').content,
    beiEreignis: (ereignis) => {
      liste = teilnehmerliste.zusammenfuehren(liste, ereignis);
      teilnehmerliste.zeichnen(ansicht.region, teilnehmerliste.ordnen(liste, teilnehmer.kennung));
      ansicht.anzahlZeigen(liste.eintraege.length);
    },
    beiZustand: (zustand, { beitreten }) => {
      ansicht.verbindungZeigen(zustand);
      if (zustand === 'abgelehnt' && beitreten) {
        stillWiederBeitreten();
      }
    },
  });
}

// Ohne GET /api/ich: dessen 401 stünde als Konsolenfehler (AK F0013); mit Web Lock tritt auch im Rückfall
// je Tab nur einer bei, die anderen lesen dessen Token.
async function stillWiederBeitreten() {
  const abgelehntesToken = tokenDesStroms;
  const wiederBeitreten = (geschwister) => sitzung.nachAblehnung(speicher, api, abgelehntesToken, geschwister);
  let zustand;
  if (navigator.locks === undefined) {
    zustand = await wiederBeitreten({ neuesToken: aufTokenEinesAnderenTabsWarten });
  } else {
    zustand = await navigator.locks.request(sitzung.SITZUNG, () => wiederBeitreten());
  }
  if (zustand.art === 'kopfzeile') {
    tokenDesStroms = speicher.getItem(sitzung.SITZUNG);
    laufenderStrom.tokenSetzen(tokenDesStroms);
    return;
  }
  stromBeenden();
  zeigen(zustand);
}

function stromBeenden() {
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
