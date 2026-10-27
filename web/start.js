// Oberfläche verdrahten (A00005): beim Laden die Sitzung prüfen, dann Beitreten-Bildschirm oder Kopfzeile.
import { rufen } from './api.js';
import * as sitzung from './sitzung.js';
import * as beitretenBildschirm from './beitreten.js';

const api = {
  ich: (token) => rufen('/api/ich', { token }),
  beitreten: (name) => rufen('/api/beitreten', { koerper: { name, art: 'mensch' } }),
};
const speicher = window.localStorage;
const wurzel = document.getElementById('app');

farbschemaFolgen();
zeigen(await sitzung.beimLaden(speicher, api));

function zeigen(zustand) {
  if (zustand.art === 'kopfzeile') {
    return;
  }
  beitretenBildschirm.zeichnen(wurzel, zustand, {
    beitreten: async (name) => zeigen(await sitzung.beitreten(name, speicher, api)),
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
