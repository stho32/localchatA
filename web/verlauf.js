// Der Verlauf eines Channels in der Mitte (ADR-0026 Regeln 5, 8, 9): Öffnen holt die neuesten 100 mit einem Abruf,
// zeichnet sie älteste oben und steht am Ende; solange der Blick dort steht, hält ein ResizeObserver ihn dort, auch
// wenn Höhen sich nachträglich ändern (Schrift, Fenster). Ein Wechsel bricht den laufenden Abruf ab, eine Antwort
// für einen verlassenen Channel wird verworfen, Daten des vorigen Channels fallen weg. Der Verlauf ist die einzige
// scrollende Fläche der Mitte: role="log", aria-live="off", mit der Tastatur fokussierbar.
import { nachrichtZeichnen, kopfSetzen, trennerZeichnen, langeEinklappen, langeUmschalten } from './nachricht.js';
import { gliederung, datumBenennen } from './gliederung.js';

// abrufen: async (name, cursor, signal) → { art: 'block' | 'fehler' | 'abgebrochen', … } (api.js verlaufAbrufen).
export function verlaufAnlegen({ abrufen }) {
  const inhalt = element('div', { className: 'lc-log-inhalt' });
  const log = element('div', { className: 'lc-log', tabIndex: 0 }, [inhalt]);
  log.setAttribute('role', 'log');
  log.setAttribute('aria-live', 'off');
  let laufend = new AbortController();
  let amEnde = true;
  let heute = null;

  function ansEnde() {
    log.scrollTop = log.scrollHeight;
    amEnde = true;
  }

  log.addEventListener('scroll', () => {
    amEnde = log.scrollHeight - log.scrollTop - log.clientHeight <= 1;
  }, { passive: true });
  const hoehen = new ResizeObserver(() => {
    if (amEnde) {
      ansEnde();
    }
  });
  hoehen.observe(inhalt);
  hoehen.observe(log);

  // Aufklappen erweitert an Ort und Stelle; der Kopf der Nachricht bleibt an seiner Bildschirmstelle.
  inhalt.addEventListener('click', (ereignis) => {
    const schalter = ereignis.target.closest('.lc-foldbar button');
    if (schalter === null) {
      return;
    }
    const anker = ankerVon(schalter.closest('.lc-msg'));
    langeUmschalten(schalter);
    ankerHalten(log, anker);
    amEnde = log.scrollHeight - log.scrollTop - log.clientHeight <= 1;
  });

  async function oeffnen(name) {
    laufend.abort();
    laufend = new AbortController();
    const dieser = laufend;
    log.setAttribute('aria-label', `Verlauf # ${name}`);
    inhalt.replaceChildren();
    const ergebnis = await abrufen(name, {}, dieser.signal);
    if (dieser !== laufend || ergebnis.art === 'abgebrochen') {
      return;
    }
    if (ergebnis.art === 'fehler') {
      const fehler = hinweis('⚠ Verlauf nicht geladen — Verbindung prüfen.', 'lc-mitte-hinweis lc-fehler');
      fehler.setAttribute('role', 'alert');
      inhalt.replaceChildren(fehler);
      return;
    }
    if (ergebnis.block.nachrichten.length === 0) {
      inhalt.replaceChildren(hinweis(`Noch keine Nachrichten in # ${name}.`, 'lc-mitte-hinweis'));
      return;
    }
    const knoten = gegliedertZeichnen(ergebnis.block.nachrichten, null, heute);
    inhalt.replaceChildren(...knoten);
    langeEinklappen(nurNachrichten(knoten));
    ansEnde();
  }

  // Das heutige Datum des Servers ('JJJJ-MM-TT') benennt die Tagestrenner neu — gleiche Höhe, nichts springt.
  function heuteSetzen(datum) {
    if (datum === heute) {
      return;
    }
    heute = datum;
    for (const trenner of inhalt.querySelectorAll('.lc-day')) {
      trenner.textContent = datumBenennen(trenner.dataset.datum, heute);
    }
  }

  function schliessen() {
    laufend.abort();
    inhalt.replaceChildren();
  }

  return { element: log, oeffnen, schliessen, heuteSetzen };
}

// Anker (ADR-0026 Regel 3): die oberste sichtbare Nachricht und ihr Abstand zum oberen Rand vor einer Änderung;
// danach wird scrollTop um ihre Verschiebung korrigiert. Gesucht wird binär über die Nachrichten im DOM.
function ankerMerken(log, inhalt) {
  const nachrichten = inhalt.querySelectorAll('.lc-msg');
  const rand = log.getBoundingClientRect().top;
  let unten = 0;
  let oben = nachrichten.length;
  while (unten < oben) {
    const mitte = Math.floor((unten + oben) / 2);
    if (nachrichten[mitte].getBoundingClientRect().bottom > rand) {
      oben = mitte;
    } else {
      unten = mitte + 1;
    }
  }
  if (unten === nachrichten.length) {
    return null;
  }
  return ankerVon(nachrichten[unten]);
}

function ankerVon(nachricht) {
  return { nachricht, top: nachricht.getBoundingClientRect().top };
}

function ankerHalten(log, anker) {
  if (anker === null || !anker.nachricht.isConnected) {
    return;
  }
  log.scrollTop += anker.nachricht.getBoundingClientRect().top - anker.top;
}

function nurNachrichten(knoten) {
  return knoten.filter((einzeln) => einzeln.classList.contains('lc-msg'));
}

// Nachrichten nach der vorigen (oder null) gegliedert: Tagestrenner vor dem ersten des Tages, Kopf nur bei neuer
// Gruppe (ADR-0026 Regel 4).
function gegliedertZeichnen(nachrichten, vorige, heute) {
  const knoten = [];
  let davor = vorige;
  for (const nachricht of nachrichten) {
    const { kopf, tagestrenner } = gliederung(davor, nachricht);
    if (tagestrenner !== null) {
      knoten.push(trennerZeichnen(tagestrenner, heute));
    }
    const artikel = nachrichtZeichnen(nachricht);
    kopfSetzen(artikel, kopf);
    knoten.push(artikel);
    davor = nachricht;
  }
  return knoten;
}

function hinweis(text, klasse) {
  return element('p', { className: klasse, textContent: text });
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
