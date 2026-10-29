// Channel-Einstellungen (R00008 F0037, ADR-0033; Designsystem ChannelEinstellungen): `# name` in der Kopfzeile
// öffnet den modalen Dialog „# name — Einstellungen“ mit dem Feld „Aufbewahrungsfrist“. Speichern sendet ohne
// Bestätigung; würden Tage entfallen (409 `bestaetigen`), folgt ein zweiter Schritt im selben Dialog mit den Zahlen
// und dem Fokus auf „Zurück“, der den Satz als Beschreibung trägt; ein gesperrter Dialog trägt den Sperrhinweis als
// Beschreibung (WCAG 4.1.3, 1.3.1). Fehler bleiben im offenen Dialog, die Eingabe steht noch; Esc und „Abbrechen“
// schließen, der Fokus kehrt zum Auslöser zurück.
import {
  fristPruefen, bestaetigungText, statusText, inzwischenText, fehlerDerAntwort, versuchZaehlen, mitVersuch,
} from './frist.js';

const HINWEIS = 'Ältere Tage entfallen. Heute zählt mit. Standard: 7 Tage.';

// setzen: async (name, tage, bestaetigt) → { serverWeg, status, daten } (api.js); statusZeigen: text → Statuszeile;
// neuOeffnen: name → den Channel neu öffnen wie „Zum Neuesten“.
export function einstellungenEinrichten({ wurzel, setzen, statusZeigen, neuOeffnen }) {
  let offen = null;

  // channel: { name, frist_tage } des gewählten Channels; ausloeser bekommt beim Schließen den Fokus zurück.
  function oeffnen(channel, ausloeser) {
    if (offen !== null) {
      return;
    }
    const teile = dialogZeichnen(channel);
    offen = { name: channel.name, frist: channel.frist_tage, laeuft: false, teile };
    const zustand = offen;
    const abmelden = new AbortController();
    teile.dialog.addEventListener('close', () => {
      abmelden.abort();
      teile.dialog.remove();
      offen = null;
      if (ausloeser.isConnected) {
        ausloeser.focus();
      }
    }, { signal: abmelden.signal });
    bedienen(zustand, { setzen, statusZeigen, neuOeffnen }, abmelden.signal);
    wurzel.append(teile.dialog);
    teile.dialog.showModal();
    if (channel.frist_tage === null) {
      teile.abbrechen.focus();
      return;
    }
    teile.feld.focus();
  }

  // Ein Wechsel aus dem Strom: ändert jemand anders die Frist, bleibt die Eingabe, der Hinweis nennt den neuen Wert.
  function channelGeaendert(channel) {
    const fremdGeaendert = offen !== null && !offen.laeuft && channel.name === offen.name && channel.frist_tage !== offen.frist;
    if (!fremdGeaendert) {
      return;
    }
    offen.frist = channel.frist_tage;
    offen.teile.inzwischen.textContent = ` ${inzwischenText(channel.frist_tage)}`;
  }

  return { oeffnen, channelGeaendert };
}

function bedienen(zustand, { setzen, statusZeigen, neuOeffnen }, signal) {
  const { teile, name } = zustand;
  let schritt = 1;
  let versuch = null;

  // Ein Fehlversuch: gleicher Schlüssel wie der vorige, solange die Zeile stand, zählt weiter (ADR-0033 Ergänzung
  // Karte #27, Regel 2).
  function fehlversuchMelden(angezeigt) {
    let vorher = versuch;
    if (teile.fehler.hidden) {
      vorher = null;
    }
    versuch = versuchZaehlen(vorher, angezeigt.schluessel);
    fehlerZeigen(teile, { ...angezeigt, text: mitVersuch(angezeigt.text, versuch.versuch) });
  }

  async function senden(tage, bestaetigt) {
    zustand.laeuft = true;
    schalterSperren(teile, true);
    const antwort = await setzen(name, tage, bestaetigt);
    const eintreffenMs = Date.now();
    zustand.laeuft = false;
    schalterSperren(teile, false);
    if (antwort.status === 200 && !antwort.serverWeg) {
      teile.dialog.close();
      statusZeigen(statusText(name, tage, antwort.daten.entfallen));
      if (antwort.daten.entfallen.tage > 0) {
        neuOeffnen(name);
      }
      return;
    }
    if (antwort.status === 409 && antwort.daten.regel === 'bestaetigen' && !bestaetigt) {
      schritt = 2;
      zweiterSchritt(teile, bestaetigungText(tage, antwort.daten.entfaellt));
      return;
    }
    schritt = 1;
    ersterSchritt(teile);
    fehlversuchMelden(fehlerDerAntwort(antwort, eintreffenMs));
    teile.feld.focus();
  }

  teile.formular.addEventListener('submit', (ereignis) => {
    ereignis.preventDefault();
    if (schritt !== 1 || zustand.laeuft) {
      return;
    }
    const geprueft = fristPruefen(teile.feld.value);
    if (!geprueft.gueltig) {
      fehlversuchMelden(pruefFehler(geprueft));
      teile.feld.focus();
      return;
    }
    senden(geprueft.tage, false);
  }, { signal });
  teile.feld.addEventListener('input', () => {
    const geprueft = fristPruefen(teile.feld.value);
    if (geprueft.gueltig) {
      fehlerZeigen(teile, null);
      return;
    }
    fehlerZeigen(teile, pruefFehler(geprueft));
  }, { signal });
  teile.abbrechen.addEventListener('click', () => teile.dialog.close(), { signal });
  teile.zurueck.addEventListener('click', () => {
    schritt = 1;
    ersterSchritt(teile);
    teile.feld.focus();
  }, { signal });
  teile.verkuerzen.addEventListener('click', () => {
    const geprueft = fristPruefen(teile.feld.value);
    if (zustand.laeuft || !geprueft.gueltig) {
      return;
    }
    senden(geprueft.tage, true);
  }, { signal });
}

function dialogZeichnen(channel) {
  const gesperrt = channel.frist_tage === null;
  const feld = element('input', {
    id: 'lc-frist', className: 'lc-feld lc-feld-zahl', type: 'number', min: 1, max: 3650, step: 1, disabled: gesperrt,
  });
  feld.setAttribute('inputmode', 'numeric');
  feld.setAttribute('aria-describedby', 'lc-frist-hinweis lc-frist-fehler');
  if (!gesperrt) {
    feld.value = String(channel.frist_tage);
  }
  const inzwischen = element('span');
  const hinweis = element('p', { id: 'lc-frist-hinweis', className: 'lc-hinweis' }, [HINWEIS, inzwischen]);
  hinweis.setAttribute('aria-live', 'polite');
  const fehlertext = element('span');
  const fehler = element('div', { id: 'lc-frist-fehler', className: 'lc-meldung lc-fehler', hidden: true }, [fehlertext]);
  fehler.setAttribute('role', 'alert');
  const bestaetigung = element('p', { id: 'lc-frist-bestaetigung', className: 'lc-bestaetigung', hidden: true });
  const abbrechen = element('button', { type: 'button', className: 'lc-btn', textContent: 'Abbrechen' });
  const speichern = element('button', { type: 'submit', className: 'lc-btn primary', textContent: 'Speichern', disabled: gesperrt });
  const zurueck = element('button', { type: 'button', className: 'lc-btn', textContent: 'Zurück', hidden: true });
  zurueck.setAttribute('aria-describedby', 'lc-frist-bestaetigung');
  const verkuerzen = element('button', { type: 'button', className: 'lc-btn primary', textContent: 'Frist verkürzen', hidden: true });
  const formular = element('form', { className: 'lc-dialog-formular', noValidate: true }, [
    element('h2', { id: 'lc-einstellungen-titel', className: 'lc-dialog-titel', textContent: `# ${channel.name} — Einstellungen` }),
    element('label', { htmlFor: 'lc-frist', className: 'lc-label', textContent: 'Aufbewahrungsfrist' }),
    element('div', { className: 'lc-frist-zeile' }, [feld, element('span', { textContent: 'Tage' })]),
    hinweis,
  ]);
  if (gesperrt) {
    formular.append(gesperrtZeichnen(channel.name));
  }
  formular.append(fehler, bestaetigung, element('div', { className: 'lc-dialog-schalter' }, [abbrechen, speichern, zurueck, verkuerzen]));
  const dialog = element('dialog', { className: 'lc-dialog' }, [formular]);
  dialog.setAttribute('aria-labelledby', 'lc-einstellungen-titel');
  if (gesperrt) {
    dialog.setAttribute('aria-describedby', 'lc-frist-gesperrt');
  }
  return { dialog, formular, feld, inzwischen, fehler, fehlertext, bestaetigung, abbrechen, speichern, zurueck, verkuerzen };
}

// ADR-0033 Regel 4: der Pfad der defekten Datei in Mono, „in diesem Channel entfällt nichts“.
function gesperrtZeichnen(name) {
  return element('p', { id: 'lc-frist-gesperrt', className: 'lc-meldung lc-fehler lc-gesperrt' }, [
    element('span', {}, [
      '⚠ Die Einstellungsdatei ',
      element('code', { textContent: `channels/${name}/einstellungen.toml` }),
      ' ist defekt — in diesem Channel entfällt nichts. Von Hand reparieren und localchatA neu starten.',
    ]),
  ]);
}

function zweiterSchritt(teile, satz) {
  fehlerZeigen(teile, null);
  teile.feld.readOnly = true;
  teile.bestaetigung.textContent = satz;
  teile.bestaetigung.hidden = false;
  teile.abbrechen.hidden = true;
  teile.speichern.hidden = true;
  teile.zurueck.hidden = false;
  teile.verkuerzen.hidden = false;
  teile.zurueck.focus();
}

function ersterSchritt(teile) {
  teile.feld.readOnly = false;
  teile.bestaetigung.hidden = true;
  teile.abbrechen.hidden = false;
  teile.speichern.hidden = false;
  teile.zurueck.hidden = true;
  teile.verkuerzen.hidden = true;
}

function schalterSperren(teile, gesperrt) {
  teile.speichern.disabled = gesperrt;
  teile.verkuerzen.disabled = gesperrt;
}

function pruefFehler(geprueft) {
  return { text: geprueft.fehler, schluessel: geprueft.fehler, eingabeFalsch: true };
}

// fehler null räumt die Fehlerzeile weg; sonst ⚠ und Text, das Feld nur bei einem Prüffehler mit aria-invalid.
function fehlerZeigen({ feld, fehler, fehlertext }, angezeigt) {
  if (angezeigt === null) {
    feld.removeAttribute('aria-invalid');
    fehler.hidden = true;
    return;
  }
  fehlertext.textContent = `⚠ ${angezeigt.text}`;
  fehler.hidden = false;
  if (angezeigt.eingabeFalsch) {
    feld.setAttribute('aria-invalid', 'true');
    return;
  }
  feld.removeAttribute('aria-invalid');
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
