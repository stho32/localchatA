'use strict';
// Gemeinsames der R00045-Tippanzeige-Specs (Karte #77, #83): das Wort der Anzeige, die Prüfung dafür im Browser und
// der trennbare Strom und zwei Browser in `# allgemein`.
const { sendenStarten, imBrowserBeitreten, eingabe } = require('./senden-hilfen');
const { browserOeffnen } = require('./r00041-hilfen');

const ANNA_TIPPT = 'anna tippt …';

/// Prüfung für waitForFunction: die Tippzeile an der Eingabe zeigt genau `text`.
function zeigtTipp(text) {
  const zeile = document.querySelector('.lc-comp .lc-comp-tipp');
  return zeile !== null && zeile.textContent === text;
}

/// Ohne SharedWorker hält der Tab den Strom selbst; die Hülle um fetch macht ihn per `window.stromTrennen()` trennbar
/// (page.route kann einen offenen Strom nicht abbrechen).
function stromTrennbarMachen(kontext) {
  return kontext.addInitScript(() => {
    const echt = window.fetch.bind(window);
    window.fetch = async (pfad, optionen) => {
      const antwort = await echt(pfad, optionen);
      if (!String(pfad).endsWith('/api/ereignisse') || antwort.body === null) {
        return antwort;
      }
      const leser = antwort.body.getReader();
      let steuerung;
      const koerper = new ReadableStream({
        start: (zugriff) => { steuerung = zugriff; },
        pull: async (zugriff) => {
          const { value: stueck, done: zuEnde } = await leser.read();
          if (zuEnde) {
            zugriff.close();
            return;
          }
          zugriff.enqueue(stueck);
        },
        cancel: () => leser.cancel(),
      });
      window.stromTrennen = () => {
        steuerung.error(new TypeError('Strom im Test getrennt'));
        leser.cancel();
      };
      return new Response(koerper, { status: antwort.status, headers: antwort.headers });
    };
  });
}

/// Startet eine Instanz und öffnet `anna` und `ben` in `# allgemein`; anna hat den Fokus im Feld. Ohne Worker hält der
/// Tab den Strom selbst und ist trennbar. → { instanz, anna, ben } (der Aufrufer stoppt die Instanz).
async function zweiBrowserStarten(browser, { annaOhneWorker = false, benOhneWorker = false } = {}) {
  const instanz = await sendenStarten();
  const anna = await browserOeffnen(browser, { ohneWorker: annaOhneWorker });
  const ben = await browserOeffnen(browser, { ohneWorker: benOhneWorker });
  for (const [mensch, ohneWorker] of [[anna, annaOhneWorker], [ben, benOhneWorker]]) {
    if (ohneWorker) {
      await stromTrennbarMachen(mensch.kontext);
    }
  }
  await imBrowserBeitreten(anna.page, instanz.adresse, 'anna');
  await imBrowserBeitreten(ben.page, instanz.adresse, 'ben');
  await eingabe(anna.page).click();
  return { instanz, anna, ben };
}

module.exports = { ANNA_TIPPT, zeigtTipp, stromTrennbarMachen, zweiBrowserStarten };
