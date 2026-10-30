// Bearbeiten an Ort und Stelle (R00041 F0135, PO 121): ein mehrzeiliges Feld in der eigenen Nachricht mit ihrem
// aktuellen Text, darunter „Abbrechen“ · „Speichern“. Tasten wie in der Eingabe (ADR-0038 Ergänzung Karte #41), Textregeln
// wie die Eingabe; leer nur mit Anhang. Höchstens ein Feld je Tab.
import { textPruefen } from './textregel.js';
import { hinweis } from './hinweis.js';

const IME_TASTENCODE = 229;

// Was eine Taste im Feld bewirkt, rein: 'speichern' | 'zeile' | 'abbrechen' | null. Mit grobem Zeiger macht Enter eine
// Zeile, Strg und Meta Enter speichern überall; Esc bricht ab; während einer Komposition gehört jede Taste ihr.
export function feldTaste(ereignis, zeiger) {
  const komponiert = ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
  if (komponiert) {
    return null;
  }
  if (ereignis.key === 'Escape') {
    return 'abbrechen';
  }
  if (ereignis.key !== 'Enter' || ereignis.altKey) {
    return null;
  }
  if (ereignis.ctrlKey || ereignis.metaKey) {
    return 'speichern';
  }
  if (ereignis.shiftKey || zeiger === 'grob') {
    return 'zeile';
  }
  return 'speichern';
}

// Warum „Speichern“ nicht geht, rein (UX S4): leer ohne Anhang, oder der Hinweis der Eingabe zu Länge und Zeichen;
// null, wenn gespeichert werden darf.
export function speicherHinweis(text, anhangZahl) {
  const ergebnis = textPruefen(text);
  if (ergebnis.art === 'leer' && anhangZahl > 0) {
    return null;
  }
  if (ergebnis.art === 'leer') {
    return 'Leer geht nicht — zum Entfernen „Löschen“ wählen.';
  }
  if (ergebnis.art === 'ok') {
    return null;
  }
  return hinweis(ergebnis).text;
}

// Das eine Feld des Tabs. zeiger: () → 'fein' | 'grob' · fokusAufNachricht(zeit): Fokus auf die Nachricht im Verlauf.
// beginnen(channel, artikel, nachricht) öffnet das Feld in `artikel`; istOffen(zeit) sagt, ob es an dieser Nachricht steht.
export function bearbeitenAnlegen({ zeiger, fokusAufNachricht }) {
  let offen = null;

  function beginnen(channel, artikel, nachricht) {
    if (offen !== null) {
      return;
    }
    offen = feldZeichnen(nachricht, zeiger());
    offen.channel = channel;
    offen.zeit = nachricht.zeit;
    offen.feld.addEventListener('input', bereitZeigen);
    offen.feld.addEventListener('keydown', tasteImFeld);
    offen.abbrechenKnopf.addEventListener('click', abbrechen);
    artikel.classList.add('lc-im-bearbeiten');
    artikel.querySelector('.lc-msg-inhalt').append(offen.rahmen);
    hoeheAnpassen(offen.feld);
    offen.feld.focus();
    offen.feld.setSelectionRange(offen.feld.value.length, offen.feld.value.length);
    bereitZeigen();
  }

  // Nicht verfügbar heißt aria-disabled, nie disabled: der Knopf bleibt fokussierbar und wird vorgelesen.
  function bereitZeigen() {
    hoeheAnpassen(offen.feld);
    const grund = speicherHinweis(offen.feld.value, offen.anhangZahl);
    offen.hinweis.textContent = '';
    offen.speichernKnopf.removeAttribute('aria-disabled');
    if (grund !== null) {
      offen.hinweis.textContent = grund;
      offen.speichernKnopf.setAttribute('aria-disabled', 'true');
    }
  }

  function tasteImFeld(ereignis) {
    const wirkung = feldTaste(ereignis, zeiger());
    if (wirkung === 'abbrechen') {
      ereignis.preventDefault();
      abbrechen();
    }
    if (wirkung === 'speichern') {
      ereignis.preventDefault();
    }
  }

  function abbrechen() {
    const zeit = offen.zeit;
    schliessen();
    fokusAufNachricht(zeit);
  }

  function schliessen() {
    const artikel = offen.rahmen.closest('.lc-msg');
    if (artikel !== null) {
      artikel.classList.remove('lc-im-bearbeiten');
    }
    offen.rahmen.remove();
    offen = null;
  }

  return {
    beginnen,
    istOffen: (zeit) => offen !== null && offen.zeit === zeit,
  };
}

// Feld, Hinweis und Knöpfe „Abbrechen“ · „Speichern“ (UX S7). Bei feinem Zeiger beschreibt das Feld seine Tasten.
function feldZeichnen(nachricht, zeigerJetzt) {
  const feld = element('textarea', { className: 'lc-bearbeiten-feld', value: nachricht.text, rows: 1 });
  feld.setAttribute('aria-label', 'Nachricht bearbeiten');
  const hinweisZeile = element('p', { id: 'lc-bearbeiten-hinweis', className: 'lc-bearbeiten-hinweis' });
  const beschreibung = [hinweisZeile.id];
  const tasten = element('span', { id: 'lc-bearbeiten-tasten', hidden: true, textContent: 'Enter speichert, Umschalt+Enter macht eine neue Zeile, Esc bricht ab.' });
  if (zeigerJetzt === 'fein') {
    beschreibung.push(tasten.id);
  }
  feld.setAttribute('aria-describedby', beschreibung.join(' '));
  const abbrechenKnopf = element('button', { type: 'button', textContent: 'Abbrechen' });
  const speichernKnopf = element('button', { type: 'button', className: 'lc-bearbeiten-speichern', textContent: 'Speichern' });
  speichernKnopf.setAttribute('aria-describedby', hinweisZeile.id);
  const knoepfe = element('div', { className: 'lc-send-schalter lc-bearbeiten-knoepfe' }, [abbrechenKnopf, speichernKnopf]);
  const rahmen = element('div', { className: 'lc-bearbeiten' }, [feld, hinweisZeile, tasten, knoepfe]);
  const anhangZahl = vorhandeneAnhaenge(nachricht).length;
  return { rahmen, feld, hinweis: hinweisZeile, abbrechenKnopf, speichernKnopf, anhangZahl };
}

// Anhänge im Zustand `vorhanden` (ein Satz ohne das Feld hat keine).
function vorhandeneAnhaenge({ anhaenge = [] }) {
  return anhaenge.filter((anhang) => anhang.zustand !== 'geloescht');
}

// Das Feld wächst mit dem Inhalt bis zur Höhe aus dem CSS (10 Zeilen) und scrollt dann in sich.
function hoeheAnpassen(feld) {
  feld.style.height = '';
  feld.style.height = `${feld.scrollHeight}px`;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
