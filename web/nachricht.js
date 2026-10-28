// Eine Nachricht im Verlauf zeichnen (Designsystem components/Nachricht, ADR-0026): Raster mit linker Spalte
// lc-gutter (Kachel bzw. Uhrzeit) und Inhalt; Kopf mit Kennung name@herkunft in lc-id und Uhrzeit HH:MM in lc-time,
// Datum mit Sekunden im title. Der Text steht nur als Text (textContent), Zeilenumbrüche bleiben (CSS pre-wrap);
// Markdown kommt mit I0017, die Art des Absenders mit I0009 — bis dahin trägt die Kachel nur das Kürzel.
import { kuerzel } from './kopfzeile.js';
import { uhrzeit, zeitTitel } from './gliederung.js';

// nachricht: { zeit, von, text } aus der API.
export function nachrichtZeichnen(nachricht) {
  const artikel = element('article', { className: 'lc-msg' });
  artikel.dataset.zeit = nachricht.zeit;
  artikel.dataset.von = nachricht.von;
  const inhalt = element('div', { className: 'lc-msg-inhalt' }, [
    element('div', { className: 'lc-body', textContent: nachricht.text }),
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
  const kachel = element('span', { className: 'lc-tile', textContent: kuerzel(name) });
  kachel.setAttribute('aria-hidden', 'true');
  gutter.replaceChildren(kachel);
  const kennung = element('span', { className: 'lc-id' }, [name, element('i', { textContent: `@${herkunft}` })]);
  inhalt.prepend(element('div', { className: 'lc-meta' }, [kennung, zeitElement(zeit)]));
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
