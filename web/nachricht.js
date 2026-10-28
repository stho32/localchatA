// Eine Nachricht im Verlauf zeichnen (Designsystem components/Nachricht, ADR-0026): Raster mit linker Spalte
// lc-gutter (Kachel bzw. Uhrzeit) und Inhalt; Kopf mit Kennung name@herkunft in lc-id und Uhrzeit HH:MM in lc-time,
// Datum mit Sekunden im title. Der Text steht nur als Text (textContent), Zeilenumbrüche bleiben (CSS pre-wrap);
// Markdown kommt mit I0017. Die Art des Absenders (Baustein Kennung): Kachel und Name in kd-agent mit der Marke
// „Agent“ für Agenten, in kd-mensch ohne Marke für Menschen und Sätze ohne Art.
import { kuerzel } from './kopfzeile.js';
import { uhrzeit, zeitTitel, datumBenennen } from './gliederung.js';

// nachricht: { zeit, von, art, text } aus der API.
export function nachrichtZeichnen(nachricht) {
  const artikel = element('article', { className: 'lc-msg' });
  artikel.dataset.zeit = nachricht.zeit;
  artikel.dataset.von = nachricht.von;
  artikel.dataset.art = darstellungsart(nachricht.art);
  const inhalt = element('div', { className: 'lc-msg-inhalt' }, [
    element('div', { className: 'lc-body lc-fold', id: `lc-text-${nachricht.zeit}`, textContent: nachricht.text }),
  ]);
  artikel.append(element('div', { className: 'g' }), inhalt);
  kopfSetzen(artikel, true);
  return artikel;
}

// Mit Kopf: Kachel links, darüber Kennung und Uhrzeit; ohne Kopf: die Uhrzeit links (Gruppe).
export function kopfSetzen(artikel, mitKopf) {
  const gutter = artikel.querySelector('.g');
  const inhalt = artikel.querySelector('.lc-msg-inhalt');
  const vorhandenerKopf = inhalt.querySelector('.lc-meta');
  if (vorhandenerKopf !== null) {
    vorhandenerKopf.remove();
  }
  const zeit = artikel.dataset.zeit;
  const von = artikel.dataset.von;
  artikel.classList.toggle('first', mitKopf);
  if (!mitKopf) {
    gutter.replaceChildren(zeitElement(zeit));
    return;
  }
  const [name, herkunft] = von.split('@');
  const art = artikel.dataset.art;
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(name) });
  kachel.setAttribute('aria-hidden', 'true');
  gutter.replaceChildren(kachel);
  const kennung = element('span', { className: `lc-id ${art}` }, [name, element('i', { textContent: `@${herkunft}` })]);
  const kopf = [kennung];
  if (art === 'agent') {
    kopf.push(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  if (zeit !== undefined) {
    kopf.push(zeitElement(zeit));
  }
  inhalt.prepend(element('div', { className: 'lc-meta' }, kopf));
}

// Ein Satz ohne Art (von Hand, ältere Sätze) wird wie ein Mensch gezeigt.
function darstellungsart(art) {
  if (art === 'agent') {
    return 'agent';
  }
  return 'mensch';
}

// Lange Nachricht (Designsystem LangeNachricht): jeder Inhalt ist per CSS auf lc-collapse begrenzt; läuft er über,
// steht darunter die Leiste „Ganze Nachricht anzeigen · n Zeilen“. Gemessen wird erst, wenn alle Nachrichten eines
// Blocks im DOM stehen — einmal lesen, dann schreiben (ADR-0026 Regel 4).
export function langeEinklappen(artikel) {
  const ueberlang = artikel.filter((einzeln) => {
    const text = einzeln.querySelector('.lc-body');
    return text.scrollHeight > text.clientHeight;
  });
  for (const einzeln of ueberlang) {
    const text = einzeln.querySelector('.lc-body');
    const schalter = element('button', { type: 'button' });
    schalter.setAttribute('aria-controls', text.id);
    schalter.dataset.zeilen = String(text.textContent.split('\n').length);
    leisteBeschriften(schalter, false);
    einzeln.querySelector('.lc-msg-inhalt').append(element('div', { className: 'lc-foldbar' }, [schalter]));
  }
}

// Auf- oder zuklappen an Ort und Stelle; den Kopf hält der Aufrufer an seiner Bildschirmstelle.
export function langeUmschalten(schalter) {
  const offen = schalter.getAttribute('aria-expanded') !== 'true';
  document.getElementById(schalter.getAttribute('aria-controls')).classList.toggle('lc-offen', offen);
  leisteBeschriften(schalter, offen);
}

function leisteBeschriften(schalter, offen) {
  schalter.setAttribute('aria-expanded', String(offen));
  if (offen) {
    schalter.textContent = 'Einklappen';
    return;
  }
  schalter.textContent = leistenBeschriftung(Number(schalter.dataset.zeilen));
}

export function leistenBeschriftung(zeilen) {
  if (zeilen === 1) {
    return 'Ganze Nachricht anzeigen · 1 Zeile';
  }
  return `Ganze Nachricht anzeigen · ${zeilen} Zeilen`;
}

// Tagestrenner (Designsystem VerlaufsMarken): Datum zentriert zwischen zwei Haarlinien, klebt oben.
export function trennerZeichnen(datum, heute) {
  const trenner = element('div', { className: 'lc-day', textContent: datumBenennen(datum, heute) });
  trenner.setAttribute('role', 'separator');
  trenner.dataset.datum = datum;
  return trenner;
}

function zeitElement(zeit) {
  const zeitpunkt = element('time', { className: 'lc-time', dateTime: zeit, textContent: uhrzeit(zeit) });
  zeitpunkt.title = zeitTitel(zeit);
  return zeitpunkt;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
