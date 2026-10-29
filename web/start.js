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

const api = {
  ich: (token) => rufen('/api/ich', { token }),
  beitreten: (name, token) => rufen('/api/beitreten', { koerper: { name, art: 'mensch' }, token }),
};
const speicher = window.localStorage;
const wurzel = document.getElementById('app');
let laufenderStrom = null;
const takten = sekundentaktAnlegen();

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

// Die Channels der Seitenleiste folgen demselben Strom (Stand und Wechsel nach `stand`, ADR-0023 Regel 6).
// Die Liste folgt dem Strom — im SharedWorker für alle Tabs, sonst im Tab: Stand und Wechsel werden nach
// `stand` zusammengeführt, geordnet und mit dem Versatz zur Serverzeit gezeichnet, Zeilen „gleich weg“ zählt der
// Sekundentakt herunter; der Herzschlag erneuert nur den Versatz. Lehnt der Strom das Token ab (Neustart, Gnadenfrist), tritt der zuständige Tab still wieder bei
// wie beim Laden und gibt das neue Token weiter.
function hauptansichtZeigen(teilnehmer, nachEingabe) {
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
    wiederBeitreten: nachSendeAblehnungBeitreten,
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
        stillWiederBeitreten(token);
      }
    },
  });
}

// Ohne GET /api/ich: dessen 401 stünde als Konsolenfehler (AK F0013); mit Web Lock tritt auch im Rückfall
// je Tab nur einer bei, die anderen lesen dessen Token.
// Das abgelehnte Token kommt vom Strom (im Worker auch für Tabs, deren eigenes Token längst veraltet ist).
async function stillWiederBeitreten(abgelehntesToken) {
  const wiederBeitreten = (geschwister) => sitzung.nachAblehnung(speicher, api, abgelehntesToken, geschwister);
  let zustand;
  if (navigator.locks === undefined) {
    zustand = await wiederBeitreten({ neuesToken: aufTokenEinesAnderenTabsWarten });
  } else {
    zustand = await navigator.locks.request(sitzung.SITZUNG, () => wiederBeitreten());
  }
  if (zustand.art === 'kopfzeile') {
    laufenderStrom.tokenSetzen(speicher.getItem(sitzung.SITZUNG));
    return;
  }
  stromBeenden();
  zeigen(zustand);
}

// Eine 401 auf das Senden (Neustart, Gnadenfrist; ADR-0012, ADR-0037 Regel 4): hat ein anderer Weg schon ein neues
// Token gemerkt, gilt es; sonst mit dem gemerkten Namen und dem alten Token beitreten — lebt die Sitzung noch, ist das
// eine Wiederkehr (200), sonst ein neuer Beitritt. Der Strom bekommt das neue Token.
async function nachSendeAblehnungBeitreten(abgelehntesToken) {
  const beitreten = async () => {
    const token = speicher.getItem(sitzung.SITZUNG);
    if (token && token !== abgelehntesToken) {
      return;
    }
    await sitzung.beitreten(speicher.getItem(sitzung.NAME) || '', speicher, api);
  };
  if (navigator.locks === undefined) {
    await beitreten();
  } else {
    await navigator.locks.request(sitzung.SITZUNG, beitreten);
  }
  if (laufenderStrom !== null) {
    laufenderStrom.tokenSetzen(speicher.getItem(sitzung.SITZUNG));
  }
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
