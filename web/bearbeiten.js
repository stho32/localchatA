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
    return 'Leer geht nicht — zum Entfernen ‚Löschen‘ wählen.';
  }
  if (ergebnis.art === 'ok') {
    return null;
  }
  return hinweis(ergebnis).text;
}

// Was die Antwort des Speicherns bedeutet (R00041 F0135, UX S5, PO 122), rein. bekanntGeaendert: `geaendert` der
// Fassung, die das Feld geöffnet hat. → { art: 'gespeichert' | 'unveraendert', nachricht } | { art: 'geloescht' }
// | { art: 'abgewiesen', text, pruefFehler }
export function speichernAuswerten(antwort, bekanntGeaendert) {
  if (antwort.art !== 'antwort') {
    return { art: 'abgewiesen', text: '⚠ Nicht gespeichert: keine Antwort vom Server.', pruefFehler: false };
  }
  if (antwort.status === 200 && antwort.daten.nachricht.geaendert === bekanntGeaendert) {
    return { art: 'unveraendert', nachricht: antwort.daten.nachricht };
  }
  if (antwort.status === 200) {
    return { art: 'gespeichert', nachricht: antwort.daten.nachricht };
  }
  if (antwort.status === 409) {
    return { art: 'geloescht' };
  }
  return { art: 'abgewiesen', text: `⚠ ${fehlerText(antwort)}`, pruefFehler: antwort.status === 422 };
}

function fehlerText({ status, daten }) {
  const mitText = daten !== null && typeof daten === 'object' && typeof daten.fehler === 'string';
  if (mitText) {
    return daten.fehler;
  }
  return `Nicht gespeichert (Status ${status}).`;
}

// Das eine Feld des Tabs. zeiger: () → 'fein' | 'grob' · fokusAufNachricht(zeit): Fokus auf die Nachricht im Verlauf ·
// speichern: async (channel, zeit, text) → Antwort wie api.js nachrichtBearbeiten · uebernehmen(nachricht): die
// gespeicherte Fassung in den Verlauf · alsGeloeschtZeigen(zeit) · hinweisen(text): die Hinweiszeile der Seite ·
// ansagen(text): genau eine Ansage über die Region der Seite (UX S2); beim Öffnen keine (S3) · inSicht(element): das
// Element ganz in den sichtbaren Teil des Verlaufs (UX M7, auch bei offener Bildschirmtastatur) · geschlossen(zeit): das
// Feld ist zu, die Nachricht soll ihre bekannte Fassung zeigen · feldHolen(): der Verlauf holt die Nachricht des offenen
// Felds wieder in den Blick und setzt es mit Fokus ein · signal: Abbau der Ansicht, beendet den Listener am Fenster.
// beginnen(channel, artikel, nachricht) öffnet das Feld in `artikel`; offeneZeitIn(channel) sagt, an welcher Nachricht
// des Channels es steht.
// Das Feld gehört dem Tab, nicht dem Knoten (F0142, PO 124): zeichnet der Verlauf die Nachricht neu oder wechselt der
// Channel, bleibt es samt Text und Schreibmarke im Speicher; einsetzen(artikel, mitFokus) hängt es wieder ein.
export function bearbeitenAnlegen({
  zeiger, fokusAufNachricht, inSicht, feldHolen, speichern, uebernehmen, alsGeloeschtZeigen, hinweisen, ansagen, geschlossen, signal,
}) {
  let offen = null;

  function beginnen(channel, artikel, nachricht) {
    if (offen !== null) {
      zweitesBeginnen(channel);
      return;
    }
    offen = feldZeichnen(nachricht, zeiger());
    offen.channel = channel;
    offen.zeit = nachricht.zeit;
    offen.geaendert = nachricht.geaendert;
    offen.laeuft = false;
    offen.auswahl = [nachricht.text.length, nachricht.text.length];
    for (const art of ['select', 'keyup', 'mouseup', 'input', 'blur']) {
      offen.feld.addEventListener(art, auswahlMerken);
    }
    offen.feld.addEventListener('input', bereitZeigen);
    offen.feld.addEventListener('keydown', tasteImFeld);
    offen.rahmen.addEventListener('keydown', escImFeldBereich);
    offen.abbrechenKnopf.addEventListener('click', abbrechen);
    offen.speichernKnopf.addEventListener('click', speichernVersuchen);
    artikel.classList.add('lc-im-bearbeiten');
    artikel.querySelector('.lc-msg-inhalt').append(offen.rahmen);
    offen.feld.focus();
    offen.feld.setSelectionRange(offen.feld.value.length, offen.feld.value.length);
    bereitZeigen();
  }

  // Höchstens ein Feld je Tab (PO 124): ein zweites „Bearbeiten“ öffnet keines; im selben Channel geht der Fokus ins
  // offene — steht es nicht im Verlauf, holt der Verlauf es zurück —, in einem anderen bleibt der Channel.
  function zweitesBeginnen(channel) {
    ansagen('Erst speichern oder abbrechen.');
    if (offen.channel !== channel) {
      return;
    }
    if (offen.rahmen.isConnected) {
      fokusInsFeld();
      return;
    }
    feldHolen();
  }

  function auswahlMerken() {
    offen.auswahl = [offen.feld.selectionStart, offen.feld.selectionEnd];
  }

  function fokusInsFeld() {
    offen.feld.focus({ preventScroll: true });
    offen.feld.setSelectionRange(offen.auswahl[0], offen.auswahl[1]);
    imBlickHalten();
  }

  function einsetzen(artikel, mitFokus) {
    artikel.classList.add('lc-im-bearbeiten');
    artikel.querySelector('.lc-msg-inhalt').append(offen.rahmen);
    if (mitFokus) {
      fokusInsFeld();
    }
  }

  // Die Nachricht ist weg (gelöscht, entfallen): das Feld schließt mit einem Hinweis, der Text geht verloren (PO 122).
  function weilWegSchliessen() {
    schliessen();
    hinweisen('Die Nachricht wurde inzwischen gelöscht.');
  }

  // Die eigene Nachricht ist gelöscht (UX S3): das Feld schließt ohne Hinweis — das Löschen sagt es selbst an.
  function weilSelbstGeloeschtSchliessen() {
    schliessen();
  }

  // Wächst das Feld oder schrumpft das Fenster (Bildschirmtastatur), bleiben Feld und Knöpfe im Blick.
  function imBlickHalten() {
    if (offen !== null && offen.rahmen.isConnected) {
      inSicht(offen.rahmen);
    }
  }
  window.addEventListener('resize', imBlickHalten, { signal });

  // Nicht verfügbar heißt aria-disabled, nie disabled: der Knopf bleibt fokussierbar und wird vorgelesen.
  function bereitZeigen() {
    const hoeheVorher = offen.feld.style.height;
    hoeheAnpassen(offen.feld);
    if (offen.feld.style.height !== hoeheVorher) {
      imBlickHalten();
    }
    const grund = speicherHinweis(offen.feld.value, offen.anhangZahl);
    offen.feld.removeAttribute('aria-invalid');
    offen.speichernKnopf.removeAttribute('aria-disabled');
    if (grund !== null) {
      offen.speichernKnopf.setAttribute('aria-disabled', 'true');
      hinweisSetzen(grund, false);
      return;
    }
    hinweisSetzen('', false);
  }

  // Der Hinweis ist selbst die Live-Region (B0802): ein neuer Text wird einmal angesagt, derselbe Text nicht noch einmal
  // bei jeder Eingabe; ein Hinweis, der nicht mehr zutrifft, wird leer.
  function hinweisSetzen(text, fehler) {
    if (offen.hinweis.textContent !== text) {
      offen.hinweis.textContent = text;
    }
    offen.hinweis.classList.toggle('lc-fehler', fehler);
  }

  function tasteImFeld(ereignis) {
    const wirkung = feldTaste(ereignis, zeiger());
    if (wirkung === 'abbrechen') {
      ereignis.preventDefault();
      abbrechen();
    }
    if (wirkung === 'speichern') {
      ereignis.preventDefault();
      speichernVersuchen();
    }
  }

  // Esc gilt im ganzen Bereich des Felds, auch bei Fokus auf „Abbrechen“ oder „Speichern“ (B0805); das Feld selbst
  // behandelt es in tasteImFeld.
  function escImFeldBereich(ereignis) {
    const komponiert = ereignis.isComposing || ereignis.keyCode === IME_TASTENCODE;
    const schonBehandelt = offen === null || ereignis.defaultPrevented || komponiert;
    if (ereignis.key !== 'Escape' || schonBehandelt) {
      return;
    }
    ereignis.preventDefault();
    abbrechen();
  }

  // Höchstens ein Aufruf zugleich (UX S7): ein Doppelklick sendet genau einen; jeder Versuch hat seinen Schlüssel.
  async function speichernVersuchen() {
    if (offen.laeuft || speicherHinweis(offen.feld.value, offen.anhangZahl) !== null) {
      return;
    }
    const versuch = offen;
    versuch.laeuft = true;
    hinweisSetzen('', false);
    const antwort = await speichern(versuch.channel, versuch.zeit, versuch.feld.value);
    versuch.laeuft = false;
    if (offen !== versuch) {
      return;
    }
    beantwortet(speichernAuswerten(antwort, versuch.geaendert));
  }

  function beantwortet(ergebnis) {
    const zeit = offen.zeit;
    if (ergebnis.art === 'abgewiesen') {
      fehlerZeigen(ergebnis);
      return;
    }
    schliessen();
    if (ergebnis.art === 'geloescht') {
      alsGeloeschtZeigen(zeit);
      hinweisen('Die Nachricht wurde inzwischen gelöscht.');
      return;
    }
    uebernehmen(ergebnis.nachricht);
    fokusAufNachricht(zeit);
    if (ergebnis.art === 'unveraendert') {
      ansagen('Keine Änderung.');
      return;
    }
    ansagen('Änderung gespeichert.');
  }

  // Die Abweisung steht am Feld und beschreibt es (UX S5); aria-invalid nur bei Prüffehlern; das Getippte bleibt.
  function fehlerZeigen({ text, pruefFehler }) {
    hinweisSetzen(text, true);
    offen.feld.removeAttribute('aria-invalid');
    if (pruefFehler) {
      offen.feld.setAttribute('aria-invalid', 'true');
    }
  }

  function abbrechen() {
    const zeit = offen.zeit;
    schliessen();
    fokusAufNachricht(zeit);
    ansagen('Bearbeiten abgebrochen.');
  }

  function schliessen() {
    const artikel = offen.rahmen.closest('.lc-msg');
    if (artikel !== null) {
      artikel.classList.remove('lc-im-bearbeiten');
    }
    offen.rahmen.remove();
    const zeit = offen.zeit;
    offen = null;
    geschlossen(zeit);
  }

  return {
    beginnen,
    einsetzen,
    weilWegSchliessen,
    weilSelbstGeloeschtSchliessen,
    // Die Zeit der Nachricht mit dem offenen Feld, wenn es zu `channel` gehört, sonst null.
    offeneZeitIn: (channel) => {
      if (offen === null || offen.channel !== channel) {
        return null;
      }
      return offen.zeit;
    },
    eingesetzt: () => offen !== null && offen.rahmen.isConnected,
  };
}

// Feld, Hinweis und Knöpfe „Abbrechen“ · „Speichern“ (UX S7). Bei feinem Zeiger beschreibt das Feld seine Tasten.
function feldZeichnen(nachricht, zeigerJetzt) {
  const feld = element('textarea', { className: 'lc-bearbeiten-feld', value: nachricht.text, rows: 1 });
  feld.setAttribute('aria-label', 'Nachricht bearbeiten');
  const hinweisZeile = element('p', { id: 'lc-bearbeiten-hinweis', className: 'lc-bearbeiten-hinweis' });
  hinweisZeile.setAttribute('aria-live', 'polite');
  hinweisZeile.setAttribute('aria-atomic', 'true');
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

// Das Feld wächst mit dem Inhalt bis zur Höhe aus dem CSS (10 Zeilen) und scrollt dann in sich; der Rand zählt zur
// Höhe (border-box). Zum Messen fällt das Feld kurz auf seine eigene Höhe zusammen: der Verlauf darunter klemmt dabei
// seine Rollstellung, und das Feld am Ende des Verlaufs rutschte hinter die Eingabe (B0804) — die Stellung bleibt darum
// erhalten.
function hoeheAnpassen(feld) {
  const verlauf = feld.closest('.lc-log');
  let rollstellung = null;
  if (verlauf !== null) {
    rollstellung = verlauf.scrollTop;
  }
  feld.style.height = '';
  const rand = feld.offsetHeight - feld.clientHeight;
  feld.style.height = `${feld.scrollHeight + rand}px`;
  if (verlauf !== null) {
    verlauf.scrollTop = rollstellung;
  }
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
