// Der Verlauf eines Channels in der Mitte (ADR-0026): Öffnen holt die neuesten 100 mit einem Abruf, zeichnet sie
// älteste oben und steht am Ende; solange der Blick dort steht, hält ein ResizeObserver ihn dort, auch wenn Höhen
// sich nachträglich ändern (Schrift, Fenster). Beim Scrollen lädt das Fenster an beiden Enden je 100 nach und hält
// höchstens fünf Blöcke; die oberste sichtbare Nachricht bleibt dabei an ihrer Stelle (Anker von Hand, in einem
// Zug). Ein Wechsel bricht laufende Abrufe ab, eine Antwort für einen verlassenen Channel wird verworfen, Daten des
// vorigen Channels fallen weg. Der Verlauf ist die einzige scrollende Fläche der Mitte: role="log",
// aria-live="off", mit der Tastatur fokussierbar; Nachladen wird nie angesagt.
import { nachrichtZeichnen, kopfSetzen, trennerZeichnen, langeEinklappen, langeUmschalten } from './nachricht.js';
import { gliederung, datumBenennen } from './gliederung.js';
import { nachladen, einfuegen } from './fenster.js';

// abrufen: async (name, cursor, signal) → { art: 'block' | 'fehler' | 'abgebrochen', … } (api.js verlaufAbrufen).
export function verlaufAnlegen({ abrufen }) {
  const inhalt = element('div', { className: 'lc-log-inhalt' });
  const log = element('div', { className: 'lc-log', tabIndex: 0 }, [inhalt]);
  log.setAttribute('role', 'log');
  log.setAttribute('aria-live', 'off');
  const ladezeile = element('div', { className: 'lc-loadmore', textContent: 'Ältere Nachrichten werden geladen …', hidden: true });
  const pille = element('button', { type: 'button', className: 'lc-pill', hidden: true }, [
    '↓ Zum Neuesten ', element('kbd', { textContent: 'Ende' }),
  ]);
  pille.setAttribute('aria-keyshortcuts', 'End');
  const rahmen = element('div', { className: 'lc-verlauf' }, [ladezeile, log, element('div', { className: 'lc-pillen' }, [pille])]);
  let laufend = new AbortController();
  let offenerName = null;
  let amEnde = true;
  let heute = null;
  let fenster = null;
  let bloeckeKnoten = [];

  function abstandUnten() {
    return log.scrollHeight - log.scrollTop - log.clientHeight;
  }

  function ansEnde() {
    log.scrollTop = log.scrollHeight;
    amEnde = true;
  }

  function lageMerken() {
    amEnde = abstandUnten() <= 1 && fenster !== null && !fenster.neuer;
    const weitOben = abstandUnten() > log.clientHeight;
    pille.hidden = fenster === null || !(weitOben || fenster.neuer);
  }

  log.addEventListener('scroll', () => {
    lageMerken();
    pruefen();
  }, { passive: true });
  const hoehen = new ResizeObserver(() => {
    if (amEnde) {
      ansEnde();
    }
    lageMerken();
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
    lageMerken();
  });

  // „Zum Neuesten“: Klick oder Ende im Verlauf öffnet den Channel neu wie beim Wählen; der Fokus geht in den Verlauf.
  function zumNeuesten() {
    log.focus();
    oeffnen(offenerName);
  }
  pille.addEventListener('click', zumNeuesten);
  log.addEventListener('keydown', (ereignis) => {
    const nurEnde = ereignis.key === 'End' && !ereignis.altKey && !ereignis.ctrlKey && !ereignis.metaKey && !ereignis.shiftKey;
    if (nurEnde && offenerName !== null) {
      ereignis.preventDefault();
      zumNeuesten();
    }
  });

  async function oeffnen(name) {
    laufend.abort();
    laufend = new AbortController();
    const dieser = laufend;
    offenerName = name;
    fenster = null;
    bloeckeKnoten = [];
    ladenBeendet();
    log.setAttribute('aria-label', `Verlauf # ${name}`);
    inhalt.replaceChildren();
    lageMerken();
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
    fenster = { bloecke: [ergebnis.block.nachrichten], aelter: ergebnis.block.aelter, neuer: false };
    const knoten = gegliedertZeichnen(ergebnis.block.nachrichten, null, heute);
    bloeckeKnoten = [knoten];
    inhalt.replaceChildren(...knoten);
    langeEinklappen(nurNachrichten(knoten));
    ansEnde();
    lageMerken();
    pruefen();
  }

  // Höchstens ein Abruf zugleich; danach wird erneut geprüft — so holt auch ein Sprung an den Rand des Scrollbalkens
  // Block für Block nach, ohne Kaskade (ADR-0026 Regel 2).
  async function pruefen() {
    if (fenster === null) {
      return;
    }
    const richtung = nachladen({
      bloecke: fenster.bloecke.length,
      aelter: fenster.aelter,
      neuer: fenster.neuer,
      abstandOben: log.scrollTop,
      abstandUnten: abstandUnten(),
      sicht: log.clientHeight,
      laeuft: log.dataset.laedt !== undefined,
    });
    if (richtung === null) {
      return;
    }
    const dieser = laufend;
    log.dataset.laedt = richtung;
    ladezeile.hidden = richtung !== 'aelter';
    const ergebnis = await abrufen(offenerName, cursorFuer(fenster, richtung), dieser.signal);
    if (dieser !== laufend) {
      return;
    }
    ladenBeendet();
    if (ergebnis.art !== 'block') {
      return;
    }
    blockEinfuegen(ergebnis.block, richtung);
    pruefen();
  }

  function ladenBeendet() {
    delete log.dataset.laedt;
    ladezeile.hidden = true;
  }

  // Einfügen, am anderen Ende abwerfen, Kopf und Trenner an den Grenzen neu — dann den Anker korrigieren, alles im
  // selben Task vor dem nächsten Bild (ADR-0026 Regel 3).
  function blockEinfuegen(block, richtung) {
    const anker = ankerMerken(log, inhalt);
    const ergebnis = einfuegen(fenster, block, richtung);
    if (block.nachrichten.length > 0) {
      knotenEinfuegen(block.nachrichten, richtung);
    }
    if (ergebnis.abgeworfen !== null) {
      knotenAbwerfen(richtung);
    }
    fenster = ergebnis.fenster;
    ankerHalten(log, anker);
    lageMerken();
  }

  function knotenEinfuegen(nachrichten, richtung) {
    if (richtung === 'aelter') {
      const knoten = gegliedertZeichnen(nachrichten, null, heute);
      inhalt.prepend(...knoten);
      bloeckeKnoten.unshift(knoten);
      grenzeGliedern(nachrichten[nachrichten.length - 1], 1);
      langeEinklappen(nurNachrichten(knoten));
      return;
    }
    const letzte = fenster.bloecke[fenster.bloecke.length - 1];
    const knoten = gegliedertZeichnen(nachrichten, letzte[letzte.length - 1], heute);
    inhalt.append(...knoten);
    bloeckeKnoten.push(knoten);
    langeEinklappen(nurNachrichten(knoten));
  }

  function knotenAbwerfen(richtung) {
    if (richtung === 'aelter') {
      bloeckeKnoten.pop().forEach((einzeln) => einzeln.remove());
      return;
    }
    bloeckeKnoten.shift().forEach((einzeln) => einzeln.remove());
    grenzeGliedern(null, 0);
  }

  // Die erste Nachricht des Blocks `index` steht jetzt nach `vorige` (oder ganz oben): Kopf und Tagestrenner neu.
  function grenzeGliedern(vorige, index) {
    const knoten = bloeckeKnoten[index];
    const artikel = knoten.find((einzeln) => einzeln.classList.contains('lc-msg'));
    const { kopf, tagestrenner } = gliederung(vorige, { zeit: artikel.dataset.zeit, von: artikel.dataset.von });
    kopfSetzen(artikel, kopf);
    const trenner = artikel.previousElementSibling;
    const hatTrenner = trenner !== null && trenner.classList.contains('lc-day');
    if (tagestrenner === null && hatTrenner) {
      trenner.remove();
      knoten.splice(knoten.indexOf(trenner), 1);
    }
    if (tagestrenner !== null && !hatTrenner) {
      const neuerTrenner = trennerZeichnen(tagestrenner, heute);
      artikel.before(neuerTrenner);
      knoten.splice(knoten.indexOf(artikel), 0, neuerTrenner);
    }
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
    offenerName = null;
    fenster = null;
    bloeckeKnoten = [];
    ladenBeendet();
    inhalt.replaceChildren();
    lageMerken();
  }

  return { element: rahmen, oeffnen, schliessen, heuteSetzen };
}

function cursorFuer(fenster, richtung) {
  if (richtung === 'aelter') {
    return { vor: fenster.bloecke[0][0].zeit };
  }
  const letzte = fenster.bloecke[fenster.bloecke.length - 1];
  return { nach: letzte[letzte.length - 1].zeit };
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
