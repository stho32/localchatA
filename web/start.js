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
// Der Lauf des Beitreten-Bildschirms (Karte #62): höchstens einer je Tab.
let bildschirmLauf = null;
const takten = sekundentaktAnlegen();
// Die Zustellung gehört dem Tab, nicht der Hauptansicht (R00025 F0088): ungesendete Nachrichten überstehen den
// Beitreten-Bildschirm; ein Neuladen verwirft sie.
const zustellung = { zustand: KEINE_ZUSTELLUNG };

farbschemaFolgen();
const geladen = await sitzungBeimLaden();
zeigen(geladen.zustand, false, geladen.abgelehntesToken);

// Nach einem Beitritt per Eingabe wandert der Fokus auf die eigene Kennung, sonst läge er nach dem Abbau
// des Formulars auf dem Dokument. `abgelehntesToken` ist das Token, dessen Ablehnung den Tab auf den Bildschirm brachte.
function zeigen(zustand, nachEingabe = false, abgelehntesToken = null) {
  bildschirmAbmelden();
  if (zustand.art === 'kopfzeile') {
    hauptansichtZeigen(zustand.teilnehmer, zustand.token, nachEingabe);
    return;
  }
  bildschirmZeigen(zustand, abgelehntesToken);
}

// Der Beitreten-Bildschirm folgt dem Beitritt eines Nachbar-Tabs (Karte #62, ADR-0012 Regeln 1–6): ein Lauscher auf
// `storage` solange er steht, dazu die Nachlese beim Zeichnen. Folgen schreibt und löscht nie im Speicher.
function bildschirmAbmelden() {
  if (bildschirmLauf !== null) {
    bildschirmLauf.abgemeldet = true;
    window.removeEventListener('storage', bildschirmLauf.beimSpeichern);
    bildschirmLauf = null;
  }
}

function bildschirmZeigen(zustand, abgelehntesToken) {
  const lauf = { abgemeldet: false, folgt: false, klickt: false, gemerkt: null, beimSpeichern: null };
  const griffe = beitretenBildschirm.zeichnen(wurzel, zustand, {
    beitreten: async (name) => {
      lauf.klickt = true;
      const neu = await sitzungSperren(() => sitzung.vomBildschirmBeitreten(name, speicher, api, abgelehntesToken));
      zeigen(neu, true, abgelehntesToken);
    },
    wartende: ungesendete(zustellung.zustand),
  });

  // Ein Lauf zugleich (Regel 5): ein Signal während des Folgens wird gemerkt und geprüft, wenn der Lauf scheitert.
  async function nachbarFolgen(gemeldet) {
    if (lauf.klickt || lauf.abgemeldet) {
      return;
    }
    if (lauf.folgt) {
      lauf.gemerkt = gemeldet;
      return;
    }
    lauf.folgt = true;
    griffe.sperren();
    let gefolgt = null;
    try {
      for (let pruefen = gemeldet; pruefen !== null && gefolgt === null;) {
        lauf.gemerkt = null;
        gefolgt = await sitzung.nachbarFolgen(speicher, api, abgelehntesToken, pruefen);
        if (lauf.gemerkt !== pruefen) {
          pruefen = lauf.gemerkt;
        } else {
          pruefen = null;
        }
      }
    } finally {
      lauf.folgt = false;
      if (gefolgt === null && !lauf.abgemeldet) {
        griffe.freigeben();
      }
    }
    if (gefolgt !== null && !lauf.abgemeldet) {
      zeigen(gefolgt);
    }
  }

  lauf.beimSpeichern = (ereignis) => {
    if (ereignis.key === sitzung.SITZUNG && ereignis.newValue && ereignis.newValue !== abgelehntesToken) {
      nachbarFolgen(ereignis.newValue);
    }
  };
  bildschirmLauf = lauf;
  window.addEventListener('storage', lauf.beimSpeichern);
  if (['beitreten', 'vergeben', 'ungueltig'].includes(zustand.art)) {
    nachlesen(abgelehntesToken, nachbarFolgen);
  }
}

// Nachlese (Regel 3): ein Token, das schon im Speicher liegt, gilt als Signal, auch wenn sein Ereignis ohne Lauscher kam.
function nachlesen(abgelehntesToken, nachbarFolgen) {
  const token = speicher.getItem(sitzung.SITZUNG);
  if (token && token !== abgelehntesToken) {
    nachbarFolgen(token);
  }
}

// Die Channels der Seitenleiste folgen demselben Strom (Stand und Wechsel nach `stand`, ADR-0023 Regel 6).
// Die Liste folgt dem Strom — im SharedWorker für alle Tabs, sonst im Tab: Stand und Wechsel werden nach
// `stand` zusammengeführt, geordnet und mit dem Versatz zur Serverzeit gezeichnet, Zeilen „gleich weg“ zählt der
// Sekundentakt herunter; der Herzschlag erneuert nur den Versatz. Lehnt der Strom (beim zuständigen Tab) oder das Senden
// das Token ab (Neustart, Gnadenfrist), führt ein Weg zum Wiederbeitritt (wiederbeitritt.js, R00025 F0088).
// Der Strom beginnt mit dem eben geprüften bzw. vergebenen Token, nie neu aus dem Speicher gelesen: den kann ein
// Nachbar-Tab ohne Web Locks geleert haben, oder er nimmt nichts an (Befund #58).
function hauptansichtZeigen(teilnehmer, token, nachEingabe) {
  const wiederBeitreten = wiederbeitrittAnlegen({
    speicher,
    api,
    sperre: sitzungSperren,
    gelungen: (token) => {
      if (laufenderStrom !== null) {
        laufenderStrom.tokenSetzen(token);
      }
    },
    gescheitert: (zustand, abgelehntesToken) => {
      zustellung.zustand = zustellen(zustellung.zustand, { art: 'beitrittGescheitert' }).zustand;
      ansicht.entwurfSichern();
      stromBeenden();
      zeigen(zustand, false, abgelehntesToken);
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
    token,
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
    return laden({ neuesToken: aufTokenEinesAnderenTabsWarten });
  }
  return navigator.locks.request(sitzung.SITZUNG, () => laden());
}

// Auf dem Bildschirm nach einem Beitritt ohne Token ist das Token, das beim Laden im Speicher lag, das abgelehnte (401).
async function laden(geschwister) {
  const gemerkt = speicher.getItem(sitzung.SITZUNG);
  const zustand = await sitzung.beimLaden(speicher, api, geschwister);
  if (['beitreten', 'vergeben', 'ungueltig'].includes(zustand.art)) {
    return { zustand, abgelehntesToken: gemerkt };
  }
  return { zustand, abgelehntesToken: null };
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
