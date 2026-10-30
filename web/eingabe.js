// Eingabe unter dem Verlauf (ADR-0038 Regeln 1–4, 7, Designsystem Eingabe).
import { textPruefen, nurLeerraum, steuerzeichenEntfernen } from './textregel.js';
import { hinweis, hinweisAnsagen } from './hinweis.js';
import { zuEigenerLetzten } from './fokus.js';
import { bezugAusNachricht, zitatText, bezugNachfuehren } from './zitat.js';
import { uhrzeit } from './gliederung.js';

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
// Entwurf (entwurf.js) folgt dem Channel (Regel 7). `eigeneLetzteFokussieren()` holt bei ↑ in der leeren Eingabe den
// Fokus auf die eigene letzte Nachricht im Verlauf (R00041 F0141, 1-tastatur.md); im leeren Feld hat ↑ sonst keine
// Wirkung, darum wird die Taste dort immer verbraucht. `tippen` (tippen.js) erfährt jede Eingabe, das Senden und den
// Wechsel des Channels — nie das Wiederherstellen des Entwurfs (R00045 F0161). `tippzeile` (aktivitaetsanzeige.js)
// steht in der Hinweisreihe hinter der Hinweiszeile, nie in ihr und nie in aria-describedby. Über dem Feld steht die
// Antwortzeile (antwortzeileAnlegen).
export function eingabeAnlegen({ senden, entwurf, ansagen, tippen, tippzeile, eigeneLetzteFokussieren = () => {} }) {
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
  const hinweisReihe = element('div', { className: 'lc-comp-hinweisreihe' }, [zeile, tippzeile, bereinigenGruppe]);
  const antwort = antwortzeileAnlegen(feld, ansagen);
  const rahmen = element('div', { className: 'lc-comp' }, [antwort.zeile, kasten, hinweisReihe]);
  let channel = null;
  let gezeigt = hinweis(textPruefen(''));
  const MELDUNG_STEHT_MS = 6000;
  let meldungEnde = null;

  function textBewerten() {
    hinweisZeigen();
    bereitZeigen();
  }

  function hinweisZeigen() {
    clearTimeout(meldungEnde);
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

  // Eine Meldung ohne Fehler steht in der Hinweiszeile statt über Feld und „Senden“ (UX S1, Karte #79) und wird einmal
  // angesagt; nach einigen Sekunden oder mit der nächsten Eingabe zeigt die Zeile wieder den Hinweis zum Text.
  function meldungZeigen(text) {
    clearTimeout(meldungEnde);
    zeile.textContent = text;
    zeile.classList.remove('lc-fehler');
    ansagen({ art: 'sofort', text });
    meldungEnde = setTimeout(hinweisZeigen, MELDUNG_STEHT_MS);
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
    tippen.eingabe(channel, feld.value);
  });
  // Die Zeigerart wird je Tastendruck gefragt: steckt jemand eine Maus ans Tablet, gilt sofort die neue Regel (K5).
  // Bei 'zeile' und während einer Komposition schreibt der Browser bzw. die Bildschirmtastatur die Zeile selbst.
  feld.addEventListener('keydown', (ereignis) => {
    if (ereignis.key === 'Escape' && antwort.steht()) {
      ereignis.preventDefault();
      antwort.abbrechen();
      return;
    }
    if (zuEigenerLetzten(ereignis, feld.value)) {
      ereignis.preventDefault();
      eigeneLetzteFokussieren(() => document.activeElement === feld && feld.value === '');
      return;
    }
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
    tippen.gesendet(channel);
    senden(text, antwort.verwerfen());
    feld.focus();
  }

  // Die Instanz hat den Bezug abgewiesen (R00047 B0715, ADR-0053 Regel 7): der Text kommt zurück — ins Feld, wenn er zum
  // offenen Channel gehört, sonst in den Entwurf seines Channels —, der Text der Instanz steht am Feld; nichts Getipptes
  // geht verloren: was inzwischen im Feld steht, bleibt hinter dem zurückgegebenen Text.
  function bezugAbgewiesen({ channel: ihrChannel, text, grund }) {
    if (ihrChannel !== channel) {
      entwurf.weggehen(ihrChannel, zusammen(text, entwurf.oeffnen(ihrChannel)));
      return;
    }
    feld.value = zusammen(text, feld.value);
    hoeheAnpassen(feld);
    textBewerten();
    entwurf.eingabe(channel, feld.value);
    zeile.textContent = `⚠ ${grund}`;
    zeile.classList.add('lc-fehler');
    ansagen({ art: 'sofort', text: grund });
  }
  function entwurfSichern() {
    if (channel !== null) {
      entwurf.weggehen(channel, feld.value);
    }
  }
  window.addEventListener('pagehide', entwurfSichern);
  return {
    element: rahmen,
    // Beim Wechsel bleibt der Entwurf des alten Channels, der des neuen kommt ins Feld; der Fokus bleibt, wo er ist.
    channelSetzen: (name) => {
      if (channel !== null) {
        entwurf.weggehen(channel, feld.value);
        tippen.gewechselt(channel);
      }
      const verworfen = antwort.verwerfen();
      channel = name;
      const beschriftung = `Nachricht an # ${name}`;
      feld.setAttribute('aria-label', beschriftung);
      feld.placeholder = beschriftung;
      feld.value = entwurf.oeffnen(name);
      hoeheAnpassen(feld);
      textBewerten();
      if (verworfen !== null) {
        meldungZeigen(`Die Antwort an ${verworfen.von} wurde beim Wechsel verworfen.`);
      }
    },
    fokussieren: () => feld.focus(),
    bezugAbgewiesen,
    // Ein Ereignis `aenderung` führt die offene Zeile des Channels nach (R00047 B0722, ADR-0053 Regel 5): bearbeitet zeigt
    // den neuen Textanfang, gelöscht „Nachricht gelöscht“; der Text im Feld bleibt.
    zitatNachfuehren: (ereignis) => {
      if (ereignis.channel === channel) {
        antwort.nachfuehren(ereignis);
      }
    },
    // „Antworten“ an einer Nachricht des Fensters (R00047 B0714): die Zeile über dem Feld, der Fokus im Feld — synchron in
    // der Geste, damit auf Touch die Bildschirmtastatur aufgeht.
    antwortBeginnen: (nachricht) => {
      antwort.beginnen(bezugAusNachricht(nachricht));
      feld.focus();
    },
    // Die Hauptansicht weicht dem Beitreten-Bildschirm: der Entwurf wird sofort geschrieben (R00025 F0088).
    entwurfSichern,
  };
}

// Die Antwortzeile über dem Feld (R00047 B0714, ADR-0053 Regel 7, Vorprüfung `ux-reviewer`): role="group" mit dem
// Namen „Antwort an <Kennung>“, darin „Antwort an <Kennung>:“, der Textanfang einzeilig und „Abbrechen“. Höchstens ein
// Bezug, gehalten hier im Zustand der Eingabe, nie im Entwurf; „Abbrechen“ und `Esc` verwerfen ihn, der Fokus geht ins
// Feld, der Text bleibt. Kein aria-describedby auf das Feld. Angesagt werden Beginnen und Abbrechen je genau einmal (B0719):
// „Antwort an <Kennung>, <HH:MM>.“ mit der Uhrzeit des Originals — nicht noch einmal für dieselbe Nachricht, solange die
// Zeile steht — und „Antwort abgebrochen.“; Senden und Channelwechsel verwerfen still.
function antwortzeileAnlegen(feld, ansagen) {
  const kennung = element('span', { className: 'lc-antwort-kennung' });
  const text = element('span', { className: 'lc-antwort-text' });
  const abbrechenKnopf = element('button', { type: 'button', className: 'lc-antwort-abbrechen', textContent: 'Abbrechen' });
  const zeile = element('div', { className: 'lc-antwortzeile', hidden: true }, [
    element('span', { className: 'lc-antwort-an' }, ['Antwort an ', kennung, ':']), text, abbrechenKnopf,
  ]);
  zeile.setAttribute('role', 'group');
  let bezug = null;

  function zeigen(neuerBezug) {
    bezug = neuerBezug;
    zeile.setAttribute('aria-label', `Antwort an ${bezug.von}`);
    kennung.textContent = bezug.von;
    text.textContent = zitatText(bezug);
    zeile.hidden = false;
  }

  function beginnen(neuerBezug) {
    const schonDieselbe = bezug !== null && Date.parse(bezug.zeit) === Date.parse(neuerBezug.zeit);
    zeigen(neuerBezug);
    if (!schonDieselbe) {
      ansagen({ art: 'sofort', text: `Antwort an ${neuerBezug.von}, ${uhrzeit(neuerBezug.zeit)}.` });
    }
  }

  // → der verworfene Bezug oder null, wenn keiner stand
  function verwerfen() {
    const verworfen = bezug;
    bezug = null;
    zeile.hidden = true;
    return verworfen;
  }

  function abbrechen() {
    if (verwerfen() !== null) {
      ansagen({ art: 'sofort', text: 'Antwort abgebrochen.' });
    }
    feld.focus();
  }

  // Ein Tippen auf „Abbrechen“ nimmt dem Feld den Fokus nicht: die Bildschirmtastatur bleibt offen.
  abbrechenKnopf.addEventListener('pointerdown', (ereignis) => {
    if (document.activeElement === feld) {
      ereignis.preventDefault();
    }
  });
  abbrechenKnopf.addEventListener('click', abbrechen);
  zeile.addEventListener('keydown', (ereignis) => {
    if (ereignis.key === 'Escape' && bezug !== null) {
      ereignis.preventDefault();
      abbrechen();
    }
  });
  // Wird das Original gelöscht, während die Zeile steht, sagt sie es einmal an (UX S2, Karte #79); gelöscht bleibt gelöscht.
  function nachfuehren(ereignis) {
    const neu = bezugNachfuehren(bezug, ereignis);
    if (neu === bezug) {
      return;
    }
    zeigen(neu);
    if (neu.zustand === 'geloescht') {
      ansagen({ art: 'sofort', text: `Antwort an ${neu.von}: Nachricht gelöscht.` });
    }
  }
  return { zeile, beginnen, verwerfen, abbrechen, nachfuehren, steht: () => bezug !== null };
}

function zusammen(zurueck, inzwischen) {
  if (inzwischen === '') {
    return zurueck;
  }
  return `${zurueck}\n${inzwischen}`;
}

function entferntText(anzahl) {
  if (anzahl === 1) {
    return '1 Steuerzeichen entfernt';
  }
  return `${anzahl} Steuerzeichen entfernt`;
}

// Die Zeigerart des Geräts: 'grob' auf Touch-Geräten, sonst 'fein' — auch das Bearbeiten-Feld folgt ihr.
export function zeigerart() {
  if (window.matchMedia('(pointer: coarse)').matches) {
    return 'grob';
  }
  return 'fein';
}

function komponiert(ereignis) {
  return ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
}

// Höhe aus dem Inhalt; die Obergrenze setzt das CSS (max-height: lc-composer-max), darüber scrollt das Feld.
// Das Messen setzt die Höhe kurz zurück und liest `scrollHeight` (erzwingt das Layout). Damit der Verlauf dabei nicht
// kurz höher wird — der Browser klemmte sonst `scrollTop` und schöbe die Lesestelle, R00048 —, hält der Kasten um das
// Feld währenddessen seine bisherige Höhe.
function hoeheAnpassen(feld) {
  const kasten = feld.parentElement;
  if (kasten !== null) {
    kasten.style.minHeight = `${kasten.offsetHeight}px`;
  }
  feld.style.height = '';
  if (feld.value !== '') {
    feld.style.height = `${feld.scrollHeight}px`;
  }
  if (kasten !== null) {
    kasten.style.minHeight = '';
  }
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
