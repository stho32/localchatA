// Eingabe unter dem Verlauf (ADR-0038 Regeln 1–3, Designsystem Eingabe).

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
export function eingabeAnlegen({ senden }) {
  const feld = document.createElement('textarea');
  feld.rows = 1;
  feld.className = 'lc-comp-feld';
  const kasten = element('div', { className: 'lc-comp-box' }, [feld]);
  const rahmen = element('div', { className: 'lc-comp' }, [kasten]);
  feld.addEventListener('input', () => hoeheAnpassen(feld));
  feld.addEventListener('keydown', (ereignis) => {
    const wirkung = taste(ereignis, feld.value);
    const enterOhneZeile = ereignis.key === 'Enter' && wirkung !== 'zeile' && !komponiert(ereignis);
    if (enterOhneZeile) {
      ereignis.preventDefault();
    }
    if (wirkung === 'senden') {
      const text = feld.value;
      feld.value = '';
      hoeheAnpassen(feld);
      senden(text);
    }
  });
  return {
    element: rahmen,
    channelSetzen: (name) => {
      const beschriftung = `Nachricht an # ${name}`;
      feld.setAttribute('aria-label', beschriftung);
      feld.placeholder = beschriftung;
    },
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
