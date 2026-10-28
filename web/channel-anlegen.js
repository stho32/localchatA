// Channel im Browser anlegen (F0025, ADR-0024 Regeln 4, 5, 7): `+` oder `Strg Alt N` öffnet den modalen Dialog
// „Channel anlegen“; Enter legt an. 201 → Dialog zu, Statuszeile, Fokus zurück zum Auslöser; jeder Fehler bleibt
// im offenen Dialog (⚠ in kd-danger, aria-invalid), die Eingabe bleibt stehen, der Fokus im Feld „Name“. Esc und „Abbrechen“ schließen.
import { pruefen } from './namensregel.js';

// anlegen: async name → { serverWeg, status, daten } (api.js); statusZeigen: text → Statuszeile;
// signal: endet mit der Ansicht.
export function dialogEinrichten({ wurzel, plus, anlegen, statusZeigen, signal }) {
  let offen = null;

  // Der Dialog entsteht beim Öffnen und verschwindet beim Schließen — geschlossen gibt es kein zweites Feld „Name“.
  function oeffnen() {
    if (offen !== null) {
      return;
    }
    const ausloeser = document.activeElement;
    offen = dialogZeichnen();
    const teile = offen;
    const abmelden = new AbortController();
    teile.dialog.addEventListener('close', () => {
      abmelden.abort();
      teile.dialog.remove();
      offen = null;
      fokusZurueck(ausloeser, plus);
    }, { signal: abmelden.signal });
    bedienen(teile, { anlegen, statusZeigen }, abmelden.signal);
    wurzel.append(teile.dialog);
    hinweisZeigen(teile);
    teile.dialog.showModal();
    teile.feld.focus();
  }

  plus.addEventListener('click', oeffnen, { signal });
  document.addEventListener('keydown', (ereignis) => {
    if (kuerzelOeffnet(ereignis)) {
      ereignis.preventDefault();
      oeffnen();
    }
  }, { signal });
}

function bedienen(teile, { anlegen, statusZeigen }, signal) {
  let laeuft = false;
  async function absenden() {
    if (laeuft) {
      return;
    }
    laeuft = true;
    teile.anlegenSchalter.disabled = true;
    const gedeutet = anlegeAntwort(await anlegen(teile.feld.value));
    laeuft = false;
    teile.anlegenSchalter.disabled = false;
    if (gedeutet.art === 'angelegt') {
      teile.dialog.close();
      statusZeigen(gedeutet.text);
      return;
    }
    fehlerZeigen(teile, gedeutet);
    // Der Schalter war während der Anfrage gesperrt und hat den Fokus verloren — der Fehler betrifft den Namen.
    teile.feld.focus();
  }

  teile.formular.addEventListener('submit', (ereignis) => {
    ereignis.preventDefault();
    absenden();
  }, { signal });
  teile.abbrechen.addEventListener('click', () => teile.dialog.close(), { signal });
  teile.feld.addEventListener('input', () => {
    fehlerZeigen(teile, null);
    hinweisZeigen(teile);
  }, { signal });
  teile.uebernehmen.addEventListener('click', () => {
    teile.feld.value = teile.uebernehmen.dataset.vorschlag;
    fehlerZeigen(teile, null);
    hinweisZeigen(teile);
    teile.feld.focus();
  }, { signal });
}

// Der Auslöser bekommt den Fokus zurück; ist er nicht mehr im Dokument oder war es keiner, der Schalter +.
function fokusZurueck(ausloeser, plus) {
  const ausloeserDa = ausloeser !== null && ausloeser !== document.body && ausloeser.isConnected;
  if (ausloeserDa) {
    ausloeser.focus();
    return;
  }
  plus.focus();
}

const REGEL = 'Erlaubt sind 1 bis 32 Zeichen aus a–z, 0–9 und -, nicht mit - am Anfang oder Ende.';

function dialogZeichnen() {
  const feld = element('input', {
    id: 'lc-channel-name', className: 'lc-feld', type: 'text', autocomplete: 'off', spellcheck: false,
  });
  feld.setAttribute('autocapitalize', 'none');
  feld.setAttribute('aria-describedby', 'lc-channel-hinweis lc-channel-fehler');
  const hinweis = element('p', { id: 'lc-channel-hinweis', className: 'lc-hinweis' });
  hinweis.setAttribute('aria-live', 'polite');
  const fehlertext = element('span');
  const uebernehmen = element('button', { type: 'button', className: 'lc-btn', hidden: true });
  const fehler = element('div', { id: 'lc-channel-fehler', className: 'lc-meldung lc-fehler', hidden: true }, [fehlertext, uebernehmen]);
  fehler.setAttribute('role', 'alert');
  const abbrechen = element('button', { type: 'button', className: 'lc-btn', textContent: 'Abbrechen' });
  const anlegenSchalter = element('button', { type: 'submit', className: 'lc-btn primary', textContent: 'Channel anlegen' });
  const formular = element('form', { className: 'lc-dialog-formular', noValidate: true }, [
    element('h2', { id: 'lc-anlegen-titel', className: 'lc-dialog-titel', textContent: 'Channel anlegen' }),
    element('label', { htmlFor: 'lc-channel-name', className: 'lc-label', textContent: 'Name' }),
    feld,
    hinweis,
    fehler,
    element('div', { className: 'lc-dialog-schalter' }, [abbrechen, anlegenSchalter]),
  ]);
  const dialog = element('dialog', { className: 'lc-dialog' }, [formular]);
  dialog.setAttribute('aria-labelledby', 'lc-anlegen-titel');
  return { dialog, formular, feld, hinweis, fehler, fehlertext, uebernehmen, abbrechen, anlegenSchalter };
}

// Live-Prüfung wie beim Beitreten: die Regel, bei ungültiger Eingabe mit Vorschlag — nur als Hinweis, der
// Server entscheidet.
function hinweisZeigen({ feld, hinweis }) {
  const geprueft = pruefen(feld.value, 'channel');
  if (feld.value === '' || geprueft.gueltig) {
    hinweis.textContent = REGEL;
    return;
  }
  let text = geprueft.hinweis;
  if (geprueft.vorschlag !== '') {
    text = `${geprueft.hinweis} Vorschlag: ${geprueft.vorschlag}`;
  }
  hinweis.textContent = text;
}

// gedeutet null räumt die Fehlerzeile weg; sonst ⚠ und Text, bei einem Vorschlag der Schalter „<vorschlag> übernehmen“.
function fehlerZeigen({ feld, fehler, fehlertext, uebernehmen }, gedeutet) {
  if (gedeutet === null) {
    fehler.hidden = true;
    feld.removeAttribute('aria-invalid');
    return;
  }
  fehlertext.textContent = `⚠ ${gedeutet.text}`;
  uebernehmen.hidden = gedeutet.vorschlag === '';
  uebernehmen.textContent = `${gedeutet.vorschlag} übernehmen`;
  uebernehmen.dataset.vorschlag = gedeutet.vorschlag;
  fehler.hidden = false;
  feld.setAttribute('aria-invalid', 'true');
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}

// `Strg Alt N` überall — außer mit AltGraph: auf deutschen Tastaturen ist AltGr gleich Strg Alt, und AltGr N
// tippt in Textfeldern ein Zeichen (Regel 7).
export function kuerzelOeffnet(ereignis) {
  const strgAlt = ereignis.ctrlKey && ereignis.altKey;
  const altGraph = ereignis.getModifierState('AltGraph');
  return strgAlt && !altGraph && ereignis.key.toLowerCase() === 'n';
}

// Antwort auf POST /api/channels ({ serverWeg, status, daten } aus api.js) → { art, text, vorschlag } für Dialog
// und Statuszeile (Regel 5); vorschlag ist '' ohne Vorschlag.
export function anlegeAntwort({ serverWeg, status, daten }) {
  if (serverWeg) {
    return gedeutet('netzfehler', 'Nicht angelegt — Verbindung prüfen.');
  }
  if (status === 201) {
    return gedeutet('angelegt', `Channel # ${daten.channel.name} angelegt.`);
  }
  if (status === 409 && daten.channel !== undefined) {
    return gedeutet('vorhanden', `# ${daten.channel.name} gibt es schon.`);
  }
  if (status === 409) {
    return gedeutet('vorhanden', daten.fehler);
  }
  if (status === 422) {
    return gedeutet('ungueltig', daten.fehler, daten.vorschlag || '');
  }
  if (status === 429) {
    return gedeutet('grenze', grenzeText(daten));
  }
  return gedeutet('fehler', fehlertext(status, daten));
}

function gedeutet(art, text, vorschlag = '') {
  return { art, text, vorschlag };
}

function grenzeText({ grenze, hoechstens }) {
  let wessen = 'insgesamt';
  if (grenze === 'herkunft') {
    wessen = 'je Rechner';
  }
  return `Zu viele Channels: höchstens ${hoechstens} ${wessen} — einen vorhandenen nutzen.`;
}

function fehlertext(status, daten) {
  if (daten !== null && typeof daten.fehler === 'string') {
    return daten.fehler;
  }
  return `Nicht angelegt (Status ${status}).`;
}
