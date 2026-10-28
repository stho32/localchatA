// Channel im Browser anlegen (F0025, ADR-0024 Regeln 4, 5, 7): `+` oder `Strg Alt N` öffnet den Dialog.

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
