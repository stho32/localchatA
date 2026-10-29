// Eingabe unter dem Verlauf (ADR-0038 Regeln 1–4, 7, Designsystem Eingabe).
import { textPruefen, nurLeerraum, steuerzeichenEntfernen } from './textregel.js';
import { hinweis, hinweisAnsagen } from './hinweis.js';

const IME_TASTENCODE = 229;

// Die eine Bedingung, unter der Taste und Knopf senden (ADR-0038 Ergänzung Karte #41 K2): nicht nur Leerraum,
// höchstens 16.384 Byte, kein verbotenes Zeichen.
export function sendbar(text) {
  return textPruefen(text).art === 'ok';
}

// Was eine Taste im Feld bewirkt (Regel 2, Ergänzung Karte #41 K5): 'senden' | 'zeile' | null. Während einer
// IME-Komposition und bei Text nur aus Leerraum nie senden; Alt Enter tut nichts. Mit grobem Zeiger (Touch-Gerät,
// `zeiger` 'grob') macht Enter eine Zeile, Strg und Meta Enter senden; sonst sendet Enter.
export function taste(ereignis, text, zeiger) {
  const komponiert = ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
  if (ereignis.key !== 'Enter' || komponiert || ereignis.altKey) {
    return null;
  }
  const mitSendetaste = ereignis.ctrlKey || ereignis.metaKey;
  if (!mitSendetaste && (ereignis.shiftKey || zeiger === 'grob')) {
    return 'zeile';
  }
  if (nurLeerraum(text)) {
    return null;
  }
  return 'senden';
}

// Das Feld unter dem Verlauf (Regel 1): <textarea rows="1"> mit zugänglichem Namen und Platzhalter „Nachricht an
// # name“; es wächst mit dem Inhalt bis lc-composer-max (im CSS) und scrollt dann in sich. `senden(text)` bekommt
// den Text; danach ist das Feld leer und wieder eine Zeile hoch, der Fokus bleibt (Regel 3). Gesperrt wird nie.
// Rechts vom Feld die Gruppe der Schalter mit dem Knopf „Senden“ als letztem (Ergänzung Karte #41 K3, K7).
// Darunter die Hinweiszeile (Regel 4): Länge und Steuerzeichen mit derselben Regel wie der Server — bei einem Fehler
// sendet Enter nicht; was die Zeile ansagt, geht ohne ⚠ an die eine Ansage-Region (`ansagen`, R00025 F0084). Der
// Entwurf (entwurf.js) folgt dem Channel (Regel 7).
export function eingabeAnlegen({ senden, entwurf, ansagen }) {
  const feld = document.createElement('textarea');
  feld.rows = 1;
  feld.className = 'lc-comp-feld';
  const zeile = element('p', { id: 'lc-comp-hinweis', className: 'lc-comp-hinweis' });
  feld.setAttribute('aria-describedby', zeile.id);
  const knopf = element('button', { type: 'button', className: 'lc-comp-senden', textContent: 'Senden' });
  knopf.setAttribute('aria-describedby', zeile.id);
  const aktionen = element('div', { className: 'lc-comp-aktionen' }, [knopf]);
  const kasten = element('div', { className: 'lc-comp-box' }, [feld, aktionen]);
  // Neben, nicht in der Hinweiszeile: deren Text ist die Beschreibung von Feld und Knopf (R00025 F0086, V17).
  const bereinigen = element('button', { type: 'button', textContent: 'Steuerzeichen entfernen' });
  const bereinigenGruppe = element('div', { className: 'lc-send-schalter lc-comp-bereinigen', hidden: true }, [bereinigen]);
  const hinweisReihe = element('div', { className: 'lc-comp-hinweisreihe' }, [zeile, bereinigenGruppe]);
  const rahmen = element('div', { className: 'lc-comp' }, [kasten, hinweisReihe]);
  let channel = null;
  let gezeigt = hinweis(textPruefen(''));

  function textBewerten() {
    hinweisZeigen();
    bereitZeigen();
  }

  function hinweisZeigen() {
    const ergebnis = textPruefen(feld.value);
    const neu = hinweis(ergebnis);
    bereinigenGruppe.hidden = ergebnis.art !== 'zeichen';
    zeile.textContent = neu.text;
    zeile.classList.toggle('lc-fehler', neu.stufe === 'fehler');
    if (neu.stufe === 'fehler') {
      feld.setAttribute('aria-invalid', 'true');
    } else {
      feld.removeAttribute('aria-invalid');
    }
    if (hinweisAnsagen(gezeigt, neu)) {
      ansagen({ art: 'sofort', text: neu.text.replace(/^⚠ /, '') });
    }
    gezeigt = neu;
  }

  // Nicht verfügbar heißt aria-disabled, nie disabled: der Knopf bleibt fokussierbar und wird vorgelesen (K2, K3).
  function bereitZeigen() {
    if (sendbar(feld.value)) {
      knopf.removeAttribute('aria-disabled');
    } else {
      knopf.setAttribute('aria-disabled', 'true');
    }
  }

  feld.addEventListener('input', () => {
    hoeheAnpassen(feld);
    textBewerten();
    entwurf.eingabe(channel, feld.value);
  });
  // Die Zeigerart wird je Tastendruck gefragt: steckt jemand eine Maus ans Tablet, gilt sofort die neue Regel (K5).
  // Bei 'zeile' und während einer Komposition schreibt der Browser bzw. die Bildschirmtastatur die Zeile selbst.
  feld.addEventListener('keydown', (ereignis) => {
    const wirkung = taste(ereignis, feld.value, zeigerart());
    const enterOhneZeile = ereignis.key === 'Enter' && wirkung !== 'zeile' && !komponiert(ereignis);
    if (enterOhneZeile) {
      ereignis.preventDefault();
    }
    if (wirkung === 'senden') {
      absenden();
    }
  });
  // Ein Tippen auf den Knopf nimmt dem Feld den Fokus nicht: die Bildschirmtastatur bleibt offen (K4).
  knopf.addEventListener('pointerdown', (ereignis) => {
    if (document.activeElement === feld) {
      ereignis.preventDefault();
    }
  });
  knopf.addEventListener('click', absenden);
  // Erst der Klick nimmt die verbotenen Zeichen aus dem Feld — ohne ihn wird nichts umgeformt (R00025 F0086).
  bereinigen.addEventListener('pointerdown', (ereignis) => {
    if (document.activeElement === feld) {
      ereignis.preventDefault();
    }
  });
  bereinigen.addEventListener('click', () => {
    const bereinigt = steuerzeichenEntfernen(feld.value);
    feld.value = bereinigt.text;
    hoeheAnpassen(feld);
    textBewerten();
    entwurf.eingabe(channel, feld.value);
    feld.focus();
    feld.setSelectionRange(feld.value.length, feld.value.length);
    ansagen({ art: 'sofort', text: entferntText(bereinigt.entfernt) });
  });
  let kompositionLaeuft = false;
  feld.addEventListener('compositionstart', () => {
    kompositionLaeuft = true;
  });
  feld.addEventListener('compositionend', () => {
    kompositionLaeuft = false;
  });

  // Der eine Absendeweg für Taste und Knopf (K1). Ein offener Wortvorschlag wird vor dem Leeren beendet — gesendet
  // wird der Text, den das Feld zeigt, und keine Tastatur schreibt das Wort danach ins leere Feld zurück (K4).
  function absenden() {
    if (!sendbar(feld.value)) {
      return;
    }
    if (kompositionLaeuft) {
      feld.blur();
      feld.focus();
    }
    const text = feld.value;
    feld.value = '';
    hoeheAnpassen(feld);
    textBewerten();
    entwurf.gesendet(channel);
    senden(text);
    feld.focus();
  }
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
      textBewerten();
    },
    fokussieren: () => feld.focus(),
  };
}

function entferntText(anzahl) {
  if (anzahl === 1) {
    return '1 Steuerzeichen entfernt';
  }
  return `${anzahl} Steuerzeichen entfernt`;
}

function zeigerart() {
  if (window.matchMedia('(pointer: coarse)').matches) {
    return 'grob';
  }
  return 'fein';
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
