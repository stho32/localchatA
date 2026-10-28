// Oberfläche verdrahten (A00005): beim Laden die Sitzung prüfen, dann Beitreten-Bildschirm oder Kopfzeile.
import { rufen } from './api.js';
import * as sitzung from './sitzung.js';
import * as beitretenBildschirm from './beitreten.js';
import * as kopfzeile from './kopfzeile.js';

const api = {
  ich: (token) => rufen('/api/ich', { token }),
  beitreten: (name) => rufen('/api/beitreten', { koerper: { name, art: 'mensch' } }),
};
const speicher = window.localStorage;
const wurzel = document.getElementById('app');

farbschemaFolgen();
zeigen(await sitzungBeimLaden());

// Nach einem Beitritt per Eingabe wandert der Fokus auf die eigene Kennung, sonst läge er nach dem Abbau
// des Formulars auf dem Dokument.
function zeigen(zustand, nachEingabe = false) {
  if (zustand.art === 'kopfzeile') {
    kopfzeile.zeichnen(wurzel, zustand.teilnehmer, { kennungFokussieren: nachEingabe });
    return;
  }
  beitretenBildschirm.zeichnen(wurzel, zustand, {
    beitreten: async (name) => zeigen(await sitzung.beitreten(name, speicher, api), true),
  });
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
