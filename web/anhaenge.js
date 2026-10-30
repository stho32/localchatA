// Hochladen im Browser (R00030 F0101, ADR-0046 Regel 13): eine Nachricht mit Dateien wartet außerhalb der
// Sendeschlange als Eintrag „wird hochgeladen …“ am Ende des eigenen Verlaufs, bis jede Datei angemeldet und oben ist;
// Text, der in der Zwischenzeit gesendet wird, geht sofort hinaus (N1). Die Einträge gehören dem Tab — ein
// Channelwechsel unterbricht nichts, gezeigt werden die des offenen Channels.
import { groesseInWorten } from './loeschen.js';
import { typkuerzel } from './dateiliste.js';
import { nachrichtZeichnen } from './nachricht.js';
import { nurLeerraum } from './textregel.js';

const ZU_VIELE_ANFRAGEN = 429;
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

// Die Einträge des Tabs. kennung: die eigene; anmelden(name, bytes) → Antwort (api.js anhangAnmelden); senden(text,
// bezug, { channel, anhaenge }): die Sendeschlange (senden.js); ansagen: die eine Ansage-Region; fokusVerloren(): der
// Fokus geht ins Feld; zeitgeber: { stellen(ms, rueckruf) → id, loeschen(id) }.
export function hochladungenAnlegen({ kennung, anmelden, senden, ansagen, fokusVerloren, zeitgeber = ECHTE_ZEITGEBER }) {
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
    artikel.querySelector('.lc-msg-inhalt').append(zeile, eintrag.liste);
    return artikel;
  }

  function dateiAnlegen(eintrag, datei) {
    const eintragDatei = {
      datei, name: datei.name, bytes: datei.size, zustand: 'anmelden', id: null, grund: '', sekunden: 0, zeitgeber: null,
      knoten: null, zustandText: null, grundText: null, schalter: null,
    };
    const kuerzel = element('span', { className: 'lc-datei-kuerzel', textContent: typkuerzel(datei.name) });
    kuerzel.setAttribute('aria-hidden', 'true');
    eintragDatei.zustandText = element('span', { className: 'lc-hochladung-zustand' });
    eintragDatei.grundText = element('p', { className: 'lc-hochladung-grund', hidden: true });
    eintragDatei.schalter = element('div', { className: 'lc-send-schalter lc-hochladung-schalter' });
    eintragDatei.knoten = element('li', { className: 'lc-hochladung-datei' }, [
      element('div', { className: 'lc-hochladung-kopf' }, [
        kuerzel,
        element('span', { className: 'lc-datei-name', textContent: datei.name }),
        element('span', { className: 'lc-datei-groesse', textContent: groesseInWorten(datei.size) }),
        eintragDatei.zustandText,
      ]),
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
      datei.zustand = 'angemeldet';
    } else if (ergebnis.art === 'warten') {
      datei.zustand = 'wartet';
      datei.sekunden = ergebnis.sekunden;
      datei.zeitgeber = zeitgeber.stellen(ergebnis.sekunden * 1000, () => dateiAnmelden(eintrag, datei));
    } else if (ergebnis.art === 'abgewiesen') {
      datei.zustand = 'abgewiesen';
      datei.grund = ergebnis.grund;
      ansagen({ art: 'sofort', text: abweisungAnsage(datei.name, ergebnis.grund) });
    } else {
      datei.zustand = 'abgewiesen';
      datei.grund = 'Keine Antwort vom Server.';
      ansagen({ art: 'sofort', text: abweisungAnsage(datei.name, datei.grund) });
    }
    dateiZeigen(eintrag, datei);
  }

  function dateiZeigen(eintrag, datei) {
    datei.zustandText.textContent = zustandBeschriftung(datei);
    const mitGrund = datei.zustand === 'abgewiesen';
    datei.grundText.hidden = !mitGrund;
    datei.grundText.textContent = '';
    if (mitGrund) {
      datei.grundText.textContent = `⚠ ${datei.grund}`;
    }
    datei.knoten.classList.toggle('lc-fehlgeschlagen', datei.zustand === 'abgewiesen');
    if (datei.zustand === 'abgewiesen') {
      datei.schalter.replaceChildren(schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => dateiEntfernen(eintrag, datei)));
    } else {
      datei.schalter.replaceChildren();
    }
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
      return;
    }
    schalterDort[Math.min(stelle, schalterDort.length - 1)].focus();
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
  return '';
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
