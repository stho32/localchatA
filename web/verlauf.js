// Der Verlauf eines Channels in der Mitte (ADR-0026 Regeln 5, 8, 9): Öffnen holt die neuesten 100 mit einem Abruf,
// zeichnet sie älteste oben und steht am Ende; solange der Blick dort steht, hält ein ResizeObserver ihn dort, auch
// wenn Höhen sich nachträglich ändern (Schrift, Fenster). Ein Wechsel bricht den laufenden Abruf ab, eine Antwort
// für einen verlassenen Channel wird verworfen, Daten des vorigen Channels fallen weg. Der Verlauf ist die einzige
// scrollende Fläche der Mitte: role="log", aria-live="off", mit der Tastatur fokussierbar.
import { nachrichtZeichnen } from './nachricht.js';

// abrufen: async (name, cursor, signal) → { art: 'block' | 'fehler' | 'abgebrochen', … } (api.js verlaufAbrufen).
export function verlaufAnlegen({ abrufen }) {
  const inhalt = element('div', { className: 'lc-log-inhalt' });
  const log = element('div', { className: 'lc-log', tabIndex: 0 }, [inhalt]);
  log.setAttribute('role', 'log');
  log.setAttribute('aria-live', 'off');
  let laufend = new AbortController();
  let amEnde = true;

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
    inhalt.replaceChildren(...ergebnis.block.nachrichten.map(nachrichtZeichnen));
    ansEnde();
  }

  function schliessen() {
    laufend.abort();
    inhalt.replaceChildren();
  }

  return { element: log, oeffnen, schliessen };
}

function hinweis(text, klasse) {
  return element('p', { className: klasse, textContent: text });
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
