// Hochladen im Browser (R00030 F0101, ADR-0046 Regel 13): eine Nachricht mit Dateien wartet außerhalb der
// Sendeschlange als Eintrag „wird hochgeladen …“ am Ende des eigenen Verlaufs, bis jede Datei angemeldet und oben ist;
// Text, der in der Zwischenzeit gesendet wird, geht sofort hinaus (N1). Die Einträge gehören dem Tab — ein
// Channelwechsel unterbricht nichts, gezeigt werden die des offenen Channels.
import { groesseInWorten } from './loeschen.js';
import { typkuerzel } from './dateiliste.js';
import { nachrichtZeichnen } from './nachricht.js';
import { nurLeerraum } from './textregel.js';

const ZU_VIELE_ANFRAGEN = 429;
const KEINE_ANTWORT = 'Keine Antwort vom Server.';
const KEIN_PLATZ = 507;

// Wie eine Anmeldung ausging (B0450), rein: angenommen (201 mit ID), warten (429 oder 507 `hochladungen` mit
// Retry-After), abgewiesen (jede andere Antwort, mit dem Text der Instanz) oder ohne Antwort.
// antwort: { art: 'antwort', status, daten, wartenS } | { art: 'netzfehler' }
// → { art: 'angemeldet', id } | { art: 'warten', sekunden } | { art: 'abgewiesen', grund } | { art: 'ohneAntwort' }
export function anmeldungBewerten(antwort) {
  if (antwort.art !== 'antwort') {
    return { art: 'ohneAntwort' };
  }
  const { status, daten } = antwort;
  if (status === 201 && daten !== null && daten.anhang !== undefined) {
    return { art: 'angemeldet', id: daten.anhang.id };
  }
  const voruebergehend = status === ZU_VIELE_ANFRAGEN || (status === KEIN_PLATZ && daten !== null && daten.grund === 'hochladungen');
  if (voruebergehend) {
    return { art: 'warten', sekunden: antwort.wartenS };
  }
  if (daten !== null && typeof daten.fehler === 'string') {
    return { art: 'abgewiesen', grund: daten.fehler };
  }
  return { art: 'abgewiesen', grund: `Nicht hochgeladen (Status ${status}).` };
}

// Der Fortschritt in ganzen Prozent (B0451), rein: abgerundet, 0 ohne bekannte Länge, höchstens 100.
export function prozentAus(geladen, gesamt) {
  if (!(gesamt > 0)) {
    return 0;
  }
  return Math.min(100, Math.max(0, Math.floor((geladen * 100) / gesamt)));
}

// Wie ein Hochladen ausging (B0451), rein: 200 fertig; 422 `metadaten` (nach dem letzten Byte, F4) eine Abweisung mit dem
// Text der Instanz; jeder andere Status und „ohne Antwort“ ein Fehlschlag mit Grund, der „Erneut hochladen“ anbietet.
// 404 `abgelaufen` oder `unbekannt`: die Anmeldung ist weg, der Browser meldet neu an (B0452).
// → { art: 'fertig' } | { art: 'abgewiesen' | 'fehlgeschlagen' | 'neuAnmelden', grund } | { art: 'abgebrochen' }
export function hochladenBewerten(antwort) {
  if (antwort.art === 'abgebrochen') {
    return { art: 'abgebrochen' };
  }
  if (antwort.art !== 'antwort') {
    return { art: 'fehlgeschlagen', grund: KEINE_ANTWORT };
  }
  const { status, daten } = antwort;
  if (status === 200) {
    return { art: 'fertig' };
  }
  const mitText = daten !== null && typeof daten.fehler === 'string';
  const anmeldungWeg = status === 404 && daten !== null && (daten.grund === 'abgelaufen' || daten.grund === 'unbekannt');
  if (anmeldungWeg) {
    return { art: 'neuAnmelden', grund: daten.fehler };
  }
  if (status === 422 && mitText && daten.regel === 'metadaten') {
    return { art: 'abgewiesen', grund: daten.fehler };
  }
  if (mitText) {
    return { art: 'fehlgeschlagen', grund: daten.fehler };
  }
  return { art: 'fehlgeschlagen', grund: `Nicht hochgeladen (Status ${status}).` };
}

// Die Ansage beim Einreihen (UX S8): einmal je Nachricht, nicht je Datei.
export function hochgeladenAnsage(anzahl) {
  if (anzahl === 1) {
    return '1 Datei hochgeladen, Nachricht wird gesendet.';
  }
  return `${anzahl} Dateien hochgeladen, Nachricht wird gesendet.`;
}

// Die Ansage einer Abweisung (UX-Antwort auf Frage 4): „Nicht hochgeladen: <name> — <Grund>“; beginnt der Text der
// Instanz selbst mit „Nicht hochgeladen: “, steht das nur einmal.
export function abweisungAnsage(name, grund) {
  const PRAEFIX = 'Nicht hochgeladen: ';
  let ohnePraefix = grund;
  if (grund.startsWith(PRAEFIX)) {
    ohnePraefix = grund.slice(PRAEFIX.length);
  }
  return `${PRAEFIX}${name} — ${ohnePraefix}`;
}

// Die Einträge des Tabs. kennung: die eigene; anmelden(name, bytes) → Antwort (api.js anhangAnmelden); hochladen(id, datei,
// fortschritt) → { abbrechen, ergebnis } (api.js anhangHochladen); plaetze: höchstens 2 Hochladungen zugleich je Browser
// (Web Locks, ohne sie je Tab); verwerfen(id): die Anmeldung freigeben (api.js anhangVerwerfen); zurueckgeben({ text, bezug }):
// „Verwerfen“ legt Text und Bezug zurück in die Eingabe; senden(text,
// bezug, { channel, anhaenge }): die Sendeschlange (senden.js); ansagen: die eine Ansage-Region; fokusVerloren(): der
// Fokus geht ins Feld; zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }.
export function hochladungenAnlegen({
  kennung, anmelden, hochladen, verwerfen, zurueckgeben, senden, ansagen, fokusVerloren, zeitgeber = ECHTE_ZEITGEBER,
  plaetze = plaetzeImBrowser(),
}) {
  const bereich = element('div', { className: 'lc-hochladungen' });
  const eintraege = [];
  let offenerChannel = null;

  function zeigen() {
    bereich.replaceChildren(...eintraege.filter((eintrag) => eintrag.channel === offenerChannel).map((eintrag) => eintrag.artikel));
  }

  // Ein Absenden mit Dateien: Leerraum allein wird zum leeren Text (B5).
  function beginnen({ text, bezug, dateien }) {
    let textFuerSenden = text;
    if (nurLeerraum(text)) {
      textFuerSenden = '';
    }
    const eintrag = { channel: offenerChannel, text: textFuerSenden, bezug, dateien: [], artikel: null, liste: null };
    eintrag.artikel = eintragZeichnen(eintrag);
    eintrag.dateien = dateien.map((datei) => dateiAnlegen(eintrag, datei));
    eintraege.push(eintrag);
    zeigen();
    eintrag.dateien.forEach((datei) => dateiAnmelden(eintrag, datei));
  }

  function eintragZeichnen(eintrag) {
    const artikel = nachrichtZeichnen({ von: kennung, art: 'mensch', text: eintrag.text, antwort_auf: eintrag.bezug });
    artikel.classList.add('lc-vorlaeufig', 'lc-hochladung');
    const zeile = element('div', { className: 'lc-send' }, [element('span', { textContent: '◌', ariaHidden: 'true' }), ' wird hochgeladen …']);
    eintrag.liste = element('ul', { className: 'lc-hochladung-dateien' });
    const verwerfenSchalter = element('button', { type: 'button', textContent: 'Verwerfen' });
    verwerfenSchalter.addEventListener('click', () => eintragVerwerfen(eintrag));
    artikel.querySelector('.lc-msg-inhalt').append(zeile, eintrag.liste, element('div', { className: 'lc-send-schalter' }, [verwerfenSchalter]));
    return artikel;
  }

  function dateiAnlegen(eintrag, datei) {
    const eintragDatei = {
      datei, name: datei.name, bytes: datei.size, zustand: 'anmelden', id: null, grund: '', sekunden: 0, zeitgeber: null,
      prozent: 0, laufend: null, neuAngemeldet: false, schalterArt: null, knoten: null, zustandText: null, grundText: null,
      schalter: null, balken: null,
    };
    const kuerzel = element('span', { className: 'lc-datei-kuerzel', textContent: typkuerzel(datei.name) });
    kuerzel.setAttribute('aria-hidden', 'true');
    eintragDatei.zustandText = element('span', { className: 'lc-hochladung-zustand' });
    eintragDatei.grundText = element('p', { className: 'lc-hochladung-grund', hidden: true });
    eintragDatei.schalter = element('div', { className: 'lc-send-schalter lc-hochladung-schalter' });
    eintragDatei.balken = balkenAnlegen(datei.name);
    eintragDatei.knoten = element('li', { className: 'lc-hochladung-datei' }, [
      element('div', { className: 'lc-hochladung-kopf' }, [
        kuerzel,
        element('span', { className: 'lc-datei-name', textContent: datei.name }),
        element('span', { className: 'lc-datei-groesse', textContent: groesseInWorten(datei.size) }),
        eintragDatei.zustandText,
      ]),
      eintragDatei.balken.element,
      eintragDatei.grundText,
      eintragDatei.schalter,
    ]);
    eintrag.liste.append(eintragDatei.knoten);
    return eintragDatei;
  }

  async function dateiAnmelden(eintrag, datei) {
    datei.zustand = 'anmelden';
    dateiZeigen(eintrag, datei);
    const ergebnis = anmeldungBewerten(await anmelden(datei.name, datei.bytes));
    if (!eintrag.dateien.includes(datei)) {
      return;
    }
    if (ergebnis.art === 'angemeldet') {
      datei.id = ergebnis.id;
      dateiHochladen(eintrag, datei);
      return;
    }
    if (ergebnis.art === 'warten') {
      datei.zustand = 'wartet';
      datei.sekunden = ergebnis.sekunden;
      datei.zeitgeber = zeitgeber.stellen(ergebnis.sekunden * 1000, () => dateiAnmelden(eintrag, datei));
    } else if (ergebnis.art === 'abgewiesen') {
      dateiScheitert(eintrag, datei, 'abgewiesen', ergebnis.grund);
      return;
    } else {
      dateiScheitert(eintrag, datei, 'fehlgeschlagen', KEINE_ANTWORT);
      return;
    }
    dateiZeigen(eintrag, datei);
  }

  // Unter einem Platz (höchstens 2 je Browser) der PUT mit Balken; bis zum Platz „wartet“.
  function dateiHochladen(eintrag, datei) {
    datei.zustand = 'wartetAufPlatz';
    datei.prozent = 0;
    dateiZeigen(eintrag, datei);
    plaetze(async () => {
      if (!eintrag.dateien.includes(datei)) {
        return;
      }
      datei.zustand = 'laedt';
      dateiZeigen(eintrag, datei);
      datei.laufend = hochladen(datei.id, datei.datei, (geladen, gesamt) => {
        datei.prozent = prozentAus(geladen, gesamt);
        balkenZeigen(datei);
      });
      const ergebnis = hochladenBewerten(await datei.laufend.ergebnis);
      datei.laufend = null;
      if (!eintrag.dateien.includes(datei) || ergebnis.art === 'abgebrochen') {
        return;
      }
      if (ergebnis.art === 'fertig') {
        datei.zustand = 'fertig';
        datei.neuAngemeldet = false;
        dateiZeigen(eintrag, datei);
        sendenWennFertig(eintrag);
        return;
      }
      if (ergebnis.art === 'neuAnmelden' && !datei.neuAngemeldet) {
        datei.neuAngemeldet = true;
        datei.id = null;
        dateiAnmelden(eintrag, datei);
        return;
      }
      if (ergebnis.art === 'neuAnmelden') {
        dateiScheitert(eintrag, datei, 'fehlgeschlagen', ergebnis.grund);
        return;
      }
      dateiScheitert(eintrag, datei, ergebnis.art, ergebnis.grund);
    });
  }

  // „Erneut hochladen“: dieselbe Anmeldung neu, der Fokus auf „Abbrechen“ derselben Datei.
  function erneutHochladen(eintrag, datei) {
    datei.neuAngemeldet = false;
    dateiHochladen(eintrag, datei);
    datei.schalter.querySelector('button').focus();
  }

  // „Abbrechen“: der PUT bricht ab, die Anmeldung wird freigegeben, die Datei verlässt den Eintrag.
  function dateiAbbrechen(eintrag, datei) {
    anmeldungFreigeben(datei);
    dateiEntfernen(eintrag, datei);
  }

  function anmeldungFreigeben(datei) {
    zeitgeber.loeschen(datei.zeitgeber);
    if (datei.laufend !== null) {
      datei.laufend.abbrechen();
    }
    if (datei.id !== null) {
      verwerfen(datei.id);
    }
  }

  // „Verwerfen“ (U2): alle Hochladungen brechen ab, Text und Bezug gehen zurück in die Eingabe.
  function eintragVerwerfen(eintrag) {
    const dateien = eintrag.dateien;
    eintrag.dateien = [];
    dateien.forEach(anmeldungFreigeben);
    eintragBeenden(eintrag);
    zurueckgeben({ text: eintrag.text, bezug: eintrag.bezug });
  }

  // Abgewiesen (nur „Entfernen“) oder fehlgeschlagen („Erneut hochladen“): ⚠ mit dem Grund an der Datei, sofort angesagt.
  function dateiScheitert(eintrag, datei, zustand, grund) {
    datei.zustand = zustand;
    datei.grund = grund;
    dateiZeigen(eintrag, datei);
    ansagen({ art: 'sofort', text: abweisungAnsage(datei.name, grund) });
  }

  function dateiZeigen(eintrag, datei) {
    datei.zustandText.textContent = zustandBeschriftung(datei);
    balkenZeigen(datei);
    const mitGrund = datei.zustand === 'abgewiesen' || datei.zustand === 'fehlgeschlagen';
    datei.grundText.hidden = !mitGrund;
    datei.grundText.textContent = '';
    if (mitGrund) {
      datei.grundText.textContent = `⚠ ${datei.grund}`;
    }
    datei.knoten.classList.toggle('lc-fehlgeschlagen', mitGrund);
    schalterZeigen(eintrag, datei);
  }

  // Der Schalter der Datei wird nur neu gebaut, wenn seine Art wechselt — ein Fokus auf „Abbrechen“ übersteht den
  // Wechsel von „wartet“ zu „lädt“.
  function schalterZeigen(eintrag, datei) {
    let art = 'abbrechen';
    if (datei.zustand === 'abgewiesen' || datei.zustand === 'fehlgeschlagen') {
      art = datei.zustand;
    }
    if (art === datei.schalterArt) {
      return;
    }
    datei.schalterArt = art;
    if (art === 'abgewiesen') {
      datei.schalter.replaceChildren(schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => dateiEntfernen(eintrag, datei)));
      return;
    }
    if (art === 'fehlgeschlagen') {
      datei.schalter.replaceChildren(
        schalterZeichnen('Erneut hochladen', `${datei.name} erneut hochladen`, () => erneutHochladen(eintrag, datei)),
        schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => dateiAbbrechen(eintrag, datei)),
      );
      return;
    }
    datei.schalter.replaceChildren(schalterZeichnen('Abbrechen', `${datei.name}: Hochladen abbrechen`, () => dateiAbbrechen(eintrag, datei)));
  }

  function balkenZeigen(datei) {
    datei.balken.zeigen(datei.zustand === 'laedt', datei.prozent);
  }

  // Eine Datei verlässt den Eintrag; der Fokus geht auf einen Schalter des Eintrags, sonst ins Feld (nie auf body).
  // War es die letzte, verschwindet der Eintrag — mit Text wird der Text gesendet.
  function dateiEntfernen(eintrag, datei) {
    zeitgeber.loeschen(datei.zeitgeber);
    const stelle = eintrag.dateien.indexOf(datei);
    eintrag.dateien = eintrag.dateien.filter((andere) => andere !== datei);
    datei.knoten.remove();
    if (eintrag.dateien.length === 0) {
      eintragBeenden(eintrag);
      if (eintrag.text !== '') {
        senden(eintrag.text, eintrag.bezug, { channel: eintrag.channel, anhaenge: [] });
      }
      fokusVerloren();
      return;
    }
    const schalterDort = eintrag.liste.querySelectorAll('button');
    if (schalterDort.length === 0) {
      fokusVerloren();
    } else {
      schalterDort[Math.min(stelle, schalterDort.length - 1)].focus();
    }
    sendenWennFertig(eintrag);
  }

  // Sind alle Dateien oben, tritt die Nachricht mit ihren IDs in die Sendeschlange (B0453, N1) — einmal angesagt. Lag
  // der Fokus im Eintrag, geht er ins Feld.
  function sendenWennFertig(eintrag) {
    const alleOben = eintrag.dateien.length > 0 && eintrag.dateien.every((datei) => datei.zustand === 'fertig');
    if (!alleOben || !eintraege.includes(eintrag)) {
      return;
    }
    const fokusImEintrag = eintrag.artikel.contains(document.activeElement);
    eintragBeenden(eintrag);
    ansagen({ art: 'sofort', text: hochgeladenAnsage(eintrag.dateien.length) });
    senden(eintrag.text, eintrag.bezug, { channel: eintrag.channel, anhaenge: eintrag.dateien.map((datei) => datei.id) });
    if (fokusImEintrag) {
      fokusVerloren();
    }
  }

  function eintragBeenden(eintrag) {
    eintraege.splice(eintraege.indexOf(eintrag), 1);
    eintrag.artikel.remove();
  }

  return {
    element: bereich,
    beginnen,
    channelGeoeffnet: (name) => {
      offenerChannel = name;
      zeigen();
    },
  };
}

function zustandBeschriftung(datei) {
  if (datei.zustand === 'wartet') {
    return `wartet · neuer Versuch in ${datei.sekunden} s`;
  }
  if (datei.zustand === 'wartetAufPlatz') {
    return 'wartet';
  }
  if (datei.zustand === 'fertig') {
    return 'hochgeladen';
  }
  return '';
}

// Der Balken einer Datei (Baustein Anhang): 4 px in kd-accent auf seiner Spur, progressbar „<name> wird hochgeladen“ mit
// aria-valuenow 0–100, daneben das Prozent; nur solange die Datei lädt.
function balkenAnlegen(name) {
  const fuellung = element('span', { className: 'lc-balken-fuellung' });
  const balken = element('span', { className: 'lc-balken' }, [fuellung]);
  balken.setAttribute('role', 'progressbar');
  balken.setAttribute('aria-label', `${name} wird hochgeladen`);
  balken.setAttribute('aria-valuemin', '0');
  balken.setAttribute('aria-valuemax', '100');
  const prozent = element('span', { className: 'lc-balken-prozent' });
  const zeile = element('div', { className: 'lc-hochladung-fortschritt', hidden: true }, [balken, prozent]);
  return {
    element: zeile,
    zeigen: (sichtbar, wert) => {
      zeile.hidden = !sichtbar;
      if (!sichtbar) {
        balken.removeAttribute('aria-valuenow');
        return;
      }
      balken.setAttribute('aria-valuenow', String(wert));
      fuellung.style.width = `${wert}%`;
      prozent.textContent = `${wert} %`;
    },
  };
}

// Höchstens 2 Hochladungen zugleich je Browser über alle Tabs (B0451): zwei Web Locks als Plätze, wer zuerst einen
// bekommt, lädt, die andere Anfrage wird zurückgezogen. Ohne Web Locks (kein sicherer Kontext, etwa über eine
// LAN-Adresse per http) gilt die Grenze je Tab.
function plaetzeImBrowser() {
  const PLAETZE = ['localchata-hochladen-1', 'localchata-hochladen-2'];
  const sperren = globalThis.navigator === undefined ? undefined : globalThis.navigator.locks;
  if (sperren === undefined) {
    return plaetzeImTab(PLAETZE.length);
  }
  return (arbeit) => new Promise((fertig) => {
    const zurueckziehen = new AbortController();
    let begonnen = false;
    for (const platz of PLAETZE) {
      sperren.request(platz, { signal: zurueckziehen.signal }, async () => {
        if (begonnen) {
          return;
        }
        begonnen = true;
        zurueckziehen.abort();
        try {
          await arbeit();
        } finally {
          fertig();
        }
      }).catch(() => {});
    }
  });
}

function plaetzeImTab(anzahl) {
  let frei = anzahl;
  const wartende = [];
  async function laufen(arbeit) {
    try {
      await arbeit();
    } finally {
      frei += 1;
      const naechste = wartende.shift();
      if (naechste !== undefined) {
        naechste();
      }
    }
  }
  return (arbeit) => new Promise((fertig) => {
    const starten = () => {
      frei -= 1;
      laufen(arbeit).then(fertig);
    };
    if (frei > 0) {
      starten();
      return;
    }
    wartende.push(starten);
  });
}

function schalterZeichnen(beschriftung, name, aktion) {
  const schalter = element('button', { type: 'button', textContent: beschriftung });
  schalter.setAttribute('aria-label', name);
  schalter.addEventListener('click', aktion);
  return schalter;
}

const ECHTE_ZEITGEBER = {
  stellen: (ms, rueckruf) => setTimeout(rueckruf, ms),
  loeschen: (id) => clearTimeout(id),
};

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
