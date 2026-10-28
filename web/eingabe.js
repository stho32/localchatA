// Eingabe unter dem Verlauf (ADR-0038 Regeln 1–4, 7, Designsystem Eingabe).
import { textPruefen } from './textregel.js';
import { hinweis, hinweisAnsagen } from './hinweis.js';

const IME_TASTENCODE = 229;

// Was eine Taste im Feld bewirkt (Regel 2): 'senden' | 'zeile' | null. Während einer IME-Komposition und bei Text
// nur aus Leerraum nie senden; Alt Enter tut nichts.
export function taste(ereignis, text) {
  const komponiert = ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
  if (ereignis.key !== 'Enter' || komponiert || ereignis.altKey) {
    return null;
  }
  if (ereignis.shiftKey) {
    return 'zeile';
  }
  if (text.trim() === '') {
    return null;
  }
  return 'senden';
}

// Das Feld unter dem Verlauf (Regel 1): <textarea rows="1"> mit zugänglichem Namen und Platzhalter „Nachricht an
// # name“; es wächst mit dem Inhalt bis lc-composer-max (im CSS) und scrollt dann in sich. `senden(text)` bekommt
// den Text; danach ist das Feld leer und wieder eine Zeile hoch, der Fokus bleibt (Regel 3). Gesperrt wird nie.
// Darunter die Hinweiszeile (Regel 4): Länge und Steuerzeichen mit derselben Regel wie der Server — bei einem Fehler
// sendet Enter nicht. Der Entwurf (entwurf.js) folgt dem Channel (Regel 7).
export function eingabeAnlegen({ senden, entwurf }) {
  const feld = document.createElement('textarea');
  feld.rows = 1;
  feld.className = 'lc-comp-feld';
  const zeile = element('p', { id: 'lc-comp-hinweis', className: 'lc-comp-hinweis' });
  feld.setAttribute('aria-describedby', zeile.id);
  const angesagt = element('span', { className: 'lc-unsichtbar' });
  angesagt.setAttribute('aria-live', 'polite');
  const kasten = element('div', { className: 'lc-comp-box' }, [feld]);
  const rahmen = element('div', { className: 'lc-comp' }, [kasten, zeile, angesagt]);
  let channel = null;
  let gezeigt = hinweis(textPruefen(''));

  function hinweisZeigen() {
    const neu = hinweis(textPruefen(feld.value));
    zeile.textContent = neu.text;
    zeile.classList.toggle('lc-fehler', neu.stufe === 'fehler');
    if (neu.stufe === 'fehler') {
      feld.setAttribute('aria-invalid', 'true');
    } else {
      feld.removeAttribute('aria-invalid');
    }
    if (hinweisAnsagen(gezeigt, neu)) {
      angesagt.textContent = neu.text;
    }
    gezeigt = neu;
  }

  feld.addEventListener('input', () => {
    hoeheAnpassen(feld);
    hinweisZeigen();
    entwurf.eingabe(channel, feld.value);
  });
  feld.addEventListener('keydown', (ereignis) => {
    const wirkung = taste(ereignis, feld.value);
    const enterOhneZeile = ereignis.key === 'Enter' && wirkung !== 'zeile' && !komponiert(ereignis);
    if (enterOhneZeile) {
      ereignis.preventDefault();
    }
    const regelVerletzt = gezeigt.stufe === 'fehler';
    if (wirkung === 'senden' && !regelVerletzt) {
      const text = feld.value;
      feld.value = '';
      hoeheAnpassen(feld);
      hinweisZeigen();
      entwurf.gesendet(channel);
      senden(text);
    }
  });
  window.addEventListener('pagehide', () => {
    if (channel !== null) {
      entwurf.weggehen(channel, feld.value);
    }
  });
  return {
    element: rahmen,
    // Beim Wechsel bleibt der Entwurf des alten Channels, der des neuen kommt ins Feld; der Fokus bleibt, wo er ist.
    channelSetzen: (name) => {
      if (channel !== null) {
        entwurf.weggehen(channel, feld.value);
      }
      channel = name;
      const beschriftung = `Nachricht an # ${name}`;
      feld.setAttribute('aria-label', beschriftung);
      feld.placeholder = beschriftung;
      feld.value = entwurf.oeffnen(name);
      hoeheAnpassen(feld);
      hinweisZeigen();
    },
    fokussieren: () => feld.focus(),
  };
}

function komponiert(ereignis) {
  return ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
}

// Höhe aus dem Inhalt; die Obergrenze setzt das CSS (max-height: lc-composer-max), darüber scrollt das Feld.
function hoeheAnpassen(feld) {
  feld.style.height = '';
  if (feld.value === '') {
    return;
  }
  feld.style.height = `${feld.scrollHeight}px`;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
