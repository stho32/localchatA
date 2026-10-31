// Hochladen im Browser (R00030 F0101, ADR-0046 Regel 13): eine Nachricht mit Dateien wartet außerhalb der
// Sendeschlange als Eintrag „wird hochgeladen …“ am Ende des eigenen Verlaufs, bis jede Datei angemeldet und oben ist;
// Text, der in der Zwischenzeit gesendet wird, geht sofort hinaus (N1). Die Einträge gehören dem Tab — ein
// Channelwechsel unterbricht nichts, gezeigt werden die des offenen Channels.
import { groesseInWorten } from './groesse.js';
import { typkuerzel } from './dateiliste.js';
import { nachrichtZeichnen } from './nachricht.js';
import { nurLeerraum } from './textregel.js';

const ZU_VIELE_ANFRAGEN = 429;
const KEINE_ANTWORT = 'Keine Antwort vom Server.';
const KEIN_PLATZ = 507;
const SEKUNDE_MS = 1000;
const MINDESTMASS = 64 * 1024;

// Wie eine Anmeldung ausging (B0450), rein: angenommen (201 mit ID), warten (429 oder 507 `hochladungen` mit
// Retry-After; beim 507 mit dem Stand des Platzes), abgewiesen (jede andere Antwort, mit dem Text der Instanz) oder
// ohne Antwort.
// antwort: { art: 'antwort', status, daten, wartenS } | { art: 'netzfehler' }
// → { art: 'angemeldet', id } | { art: 'warten', sekunden, platz: { hoechstens, gebunden, braucht } | null }
//   | { art: 'abgewiesen', grund } | { art: 'ohneAntwort' }
export function anmeldungBewerten(antwort) {
  if (antwort.art !== 'antwort') {
    return { art: 'ohneAntwort' };
  }
  const { status, daten } = antwort;
  if (status === 201 && daten !== null && daten.anhang !== undefined) {
    return { art: 'angemeldet', id: daten.anhang.id };
  }
  if (status === ZU_VIELE_ANFRAGEN) {
    return { art: 'warten', sekunden: antwort.wartenS, platz: null };
  }
  const platzBelegt = status === KEIN_PLATZ && daten !== null && daten.grund === 'hochladungen';
  if (platzBelegt) {
    const { hoechstens, gebunden, braucht } = daten;
    return { art: 'warten', sekunden: antwort.wartenS, platz: { hoechstens, gebunden, braucht } };
  }
  if (daten !== null && typeof daten.fehler === 'string') {
    return { art: 'abgewiesen', grund: daten.fehler };
  }
  return { art: 'abgewiesen', grund: `Nicht hochgeladen (Status ${status}).` };
}

// Der Wartezustand (UX-Review #43 F0101 S2), rein: zählt die Sekunden bis zum nächsten Versuch; belegen laufende
// Hochladungen den Platz (507 `hochladungen`), sagt er es.
export function wartetText(sekunden, platzBelegt) {
  const text = `wartet · neuer Versuch in ${sekunden} s`;
  if (platzBelegt) {
    return `${text} · Platz für Anhänge belegt`;
  }
  return text;
}

// Hilft Warten nicht? (S2), rein: die Datei passt nie, solange die eigenen Anmeldungen des Tabs gehalten werden —
// braucht > höchstens − gebunden − eigene. Dann wird abgewiesen statt endlos gewartet.
export function selbstBlockiert(platz, eigeneBytes) {
  return platz.braucht > daneben(platz, eigeneBytes);
}

export function selbstBlockiertGrund(platz, eigeneBytes) {
  return `Nicht hochgeladen: die übrigen Dateien dieser Nachricht belegen den Platz für Anhänge — daneben bleiben ${groesseInWorten(daneben(platz, eigeneBytes))}, die Datei braucht ${groesseInWorten(platz.braucht)}. Jede Datei belegt mindestens ${groesseInWorten(MINDESTMASS)}. „Entfernen“ sendet die übrigen; diese Datei danach einzeln senden.`;
}

// Wie die Instanz einen Anhang an der Anhang-Quote zählt (src/gespraeche/anhangquote.rs): mindestens 64 KiB, sonst auf
// 4 KiB aufgerundet. Die Rechnung der Selbstblockade läuft in diesem Maß.
export function zaehlmass(bytes) {
  const BLOCK = 4 * 1024;
  return Math.max(MINDESTMASS, Math.ceil(bytes / BLOCK) * BLOCK);
}

function daneben(platz, eigeneBytes) {
  return Math.max(0, platz.hoechstens - platz.gebunden - eigeneBytes);
}

// Die Zeile unter dem Text des Eintrags (S3), rein: hängt die Nachricht an abgewiesenen Dateien, sagt sie den Weg.
export function eintragZustandText(abgewiesene) {
  if (abgewiesene === 0) {
    return ' wird hochgeladen …';
  }
  let dateien = 'Datei';
  if (abgewiesene > 1) {
    dateien = 'Dateien';
  }
  return ` angehalten — nach „Entfernen“ der abgewiesenen ${dateien} wird gesendet`;
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
// 409 nach einem abgerissenen PUT (Code-Review #43 F0119 B1): `fertig` — der Server hat die Datei schon — ist geschafft,
// `laeuft` — er hält den abgebrochenen PUT noch fest — heißt später noch einmal.
// → { art: 'fertig' | 'nochEinmal' } | { art: 'abgewiesen' | 'fehlgeschlagen' | 'neuAnmelden', grund } | { art: 'abgebrochen' }
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
  if (status === 409 && daten !== null && daten.regel === 'fertig') {
    return { art: 'fertig' };
  }
  if (status === 409 && daten !== null && daten.regel === 'laeuft') {
    return { art: 'nochEinmal' };
  }
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

// Die Einträge des Tabs, in fünf Teilen (Schnitt B0799, Verhalten unverändert): die Ansicht zeichnet und zeigt, die
// Anmeldung holt die Annahme der Instanz, die Hochladung schickt den PUT, die Verbindung hält an und setzt fort, die
// Einträge führen Nachricht und Datei zu Ende. Die Teile rufen einander über `teile` auf, erst zur Laufzeit gebunden;
// `stand` ist der gemeinsame Zustand: { eintraege, offenerChannel, verbunden }.
// kennung: die eigene; anmelden(name, bytes) → Antwort (api.js anhangAnmelden); hochladen(id, datei, fortschritt) →
// { abbrechen, ergebnis } (api.js anhangHochladen); plaetze: höchstens 2 Hochladungen zugleich je Browser (Web Locks, ohne
// sie je Tab); verwerfen(id): die Anmeldung freigeben (api.js anhangVerwerfen); zurueckgeben({ text, bezug }): „Verwerfen“
// legt Text und Bezug zurück in die Eingabe; senden(text, bezug, { channel, anhaenge }): die Sendeschlange (senden.js);
// ansagen: die eine Ansage-Region; fokusVerloren(): der Fokus geht ins Feld; zeitgeber: { stellen(ms, rueckruf) → id,
// loeschen(id) }.
export function hochladungenAnlegen({
  kennung, anmelden, hochladen, verwerfen, zurueckgeben, senden, ansagen, fokusVerloren, zeitgeber = ECHTE_ZEITGEBER,
  plaetze = plaetzeImBrowser(),
}) {
  const bereich = element('div', { className: 'lc-hochladungen' });
  // Ohne Verbindung (R00030 F0119 B0520, Zustand aus dem Wiederverbinden der Hauptansicht) wartet jede Datei, statt
  // zu scheitern.
  const stand = { eintraege: [], offenerChannel: null, verbunden: true };
  const teile = {};
  teile.ansicht = ansichtAnlegen({ kennung, ansagen, teile });
  teile.anmeldung = anmeldungAnlegen({ stand, teile, anmelden, verwerfen, zeitgeber });
  teile.hochladung = hochladungAnlegen({ stand, teile, hochladen, plaetze, zeitgeber });
  teile.verbindung = verbindungAnlegen({ stand, teile });
  teile.eintraege = eintraegeAnlegen({ stand, teile, bereich, senden, zurueckgeben, ansagen, fokusVerloren, zeitgeber });
  return {
    element: bereich,
    beginnen: teile.eintraege.beginnen,
    channelGeoeffnet: (name) => {
      stand.offenerChannel = name;
      teile.eintraege.zeigen();
    },
    // Was der Tab sich beim Verlassen merkt (B0519): je offenem Eintrag nur Channel, Text und Dateinamen.
    offene: () => stand.eintraege.map((eintrag) => ({
      channel: eintrag.channel, text: eintrag.text, namen: eintrag.dateien.map((datei) => datei.name),
    })),
    alleAnhalten: teile.eintraege.alleAnhalten,
    verbindungGetrennt: teile.verbindung.getrennt,
    wiederVerbunden: teile.verbindung.wiederVerbunden,
  };
}

// Zeichnen und Zeigen: der Eintrag „wird hochgeladen …“ mit seiner Liste, je Datei Kopf, Balken, Grund und Schalter.
function ansichtAnlegen({ kennung, ansagen, teile }) {
  function eintragZeichnen(eintrag) {
    const artikel = nachrichtZeichnen({ von: kennung, art: 'mensch', text: eintrag.text, antwort_auf: eintrag.bezug });
    artikel.classList.add('lc-vorlaeufig', 'lc-hochladung');
    eintrag.zustandText = document.createTextNode(eintragZustandText(0));
    const zeile = element('div', { className: 'lc-send' }, [element('span', { textContent: '◌', ariaHidden: 'true' }), eintrag.zustandText]);
    eintrag.liste = element('ul', { className: 'lc-hochladung-dateien' });
    const verwerfenSchalter = element('button', { type: 'button', textContent: 'Verwerfen' });
    verwerfenSchalter.addEventListener('click', () => teile.eintraege.verwerfen(eintrag));
    artikel.querySelector('.lc-msg-inhalt').append(zeile, eintrag.liste, element('div', { className: 'lc-send-schalter lc-hochladung-verwerfen' }, [verwerfenSchalter]));
    return artikel;
  }

  function dateiAnlegen(eintrag, datei) {
    const eintragDatei = {
      datei, name: datei.name, bytes: datei.size, zustand: 'anmelden', id: null, grund: '', sekunden: 0, platzBelegt: false, zeitgeber: null,
      prozent: 0, laufend: null, neuAngemeldet: false, nochEinmalVersuche: 0, schalterArt: null, knoten: null, zustandText: null, grundText: null,
      schalter: null, balken: null, freigebenNachAbbruch: true,
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

  function kopfZeigen(eintrag) {
    eintrag.zustandText.textContent = eintragZustandText(eintrag.dateien.filter((datei) => datei.zustand === 'abgewiesen').length);
  }

  function dateiZeigen(eintrag, datei) {
    kopfZeigen(eintrag);
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

  // Abgewiesen (nur „Entfernen“) oder fehlgeschlagen („Erneut hochladen“): ⚠ mit dem Grund an der Datei, sofort angesagt.
  function dateiScheitert(eintrag, datei, zustand, grund) {
    datei.zustand = zustand;
    datei.grund = grund;
    dateiZeigen(eintrag, datei);
    ansagen({ art: 'sofort', text: abweisungAnsage(datei.name, grund) });
  }

  // Der Schalter der Datei wird nur neu gebaut, wenn seine Art wechselt — ein Fokus auf „Abbrechen“ übersteht den
  // Wechsel von „wartet“ zu „lädt“. Eine fertige Datei trägt „Entfernen“: sie gibt die Anmeldung frei und verlässt die
  // Nachricht (Code-Review #43 F0101 A7, UX O1).
  function schalterZeigen(eintrag, datei) {
    let art = 'abbrechen';
    if (['abgewiesen', 'fehlgeschlagen', 'fertig'].includes(datei.zustand)) {
      art = datei.zustand;
    }
    if (art === datei.schalterArt) {
      return;
    }
    datei.schalterArt = art;
    if (art === 'abgewiesen') {
      datei.schalter.replaceChildren(schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => teile.eintraege.entfernen(eintrag, datei)));
      return;
    }
    if (art === 'fehlgeschlagen') {
      datei.schalter.replaceChildren(
        schalterZeichnen('Erneut hochladen', `${datei.name} erneut hochladen`, () => teile.hochladung.erneut(eintrag, datei)),
        schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => teile.eintraege.abbrechen(eintrag, datei)),
      );
      return;
    }
    if (art === 'fertig') {
      datei.schalter.replaceChildren(schalterZeichnen('Entfernen', `${datei.name} entfernen`, () => teile.eintraege.abbrechen(eintrag, datei)));
      return;
    }
    datei.schalter.replaceChildren(schalterZeichnen('Abbrechen', `${datei.name}: Hochladen abbrechen`, () => teile.eintraege.abbrechen(eintrag, datei)));
  }

  function balkenZeigen(datei) {
    datei.balken.zeigen(datei.zustand === 'laedt', datei.prozent);
  }

  return { eintragZeichnen, dateiAnlegen, kopfZeigen, dateiZeigen, dateiScheitert, balkenZeigen };
}

// Die Anmeldung einer Datei bei der Instanz: Annahme, Warten (429, 507), Abweisung, Selbstblockade, Freigabe.
function anmeldungAnlegen({ stand, teile, anmelden, verwerfen, zeitgeber }) {
  async function dateiAnmelden(eintrag, datei) {
    if (!stand.verbunden) {
      teile.verbindung.aufVerbindungWarten(eintrag, datei);
      return;
    }
    datei.zustand = 'anmelden';
    teile.ansicht.dateiZeigen(eintrag, datei);
    const ergebnis = anmeldungBewerten(await anmelden(datei.name, datei.bytes));
    if (!eintrag.dateien.includes(datei)) {
      anmeldungNachAbbruchFreigeben(datei, ergebnis);
      return;
    }
    if (ergebnis.art === 'angemeldet') {
      datei.id = ergebnis.id;
      teile.hochladung.dateiHochladen(eintrag, datei);
      return;
    }
    if (ergebnis.art === 'warten') {
      const eigeneBytes = eigeneAngemeldeteBytes();
      if (ergebnis.platz !== null && selbstBlockiert(ergebnis.platz, eigeneBytes)) {
        teile.ansicht.dateiScheitert(eintrag, datei, 'abgewiesen', selbstBlockiertGrund(ergebnis.platz, eigeneBytes));
        return;
      }
      datei.zustand = 'wartet';
      datei.sekunden = ergebnis.sekunden;
      datei.platzBelegt = ergebnis.platz !== null;
      datei.zeitgeber = zeitgeber.stellen(SEKUNDE_MS, () => wartenZaehlen(eintrag, datei));
    } else if (ergebnis.art === 'abgewiesen') {
      teile.ansicht.dateiScheitert(eintrag, datei, 'abgewiesen', ergebnis.grund);
      return;
    } else if (!stand.verbunden) {
      teile.verbindung.aufVerbindungWarten(eintrag, datei);
      return;
    } else {
      teile.ansicht.dateiScheitert(eintrag, datei, 'fehlgeschlagen', KEINE_ANTWORT);
      return;
    }
    teile.ansicht.dateiZeigen(eintrag, datei);
  }

  // Der Wartezustand zählt im Sekundentakt herunter (UX-Review #43 F0101 S2); bei 0 meldet die Datei neu an.
  function wartenZaehlen(eintrag, datei) {
    datei.sekunden -= 1;
    if (datei.sekunden <= 0) {
      dateiAnmelden(eintrag, datei);
      return;
    }
    teile.ansicht.dateiZeigen(eintrag, datei);
    datei.zeitgeber = zeitgeber.stellen(SEKUNDE_MS, () => wartenZaehlen(eintrag, datei));
  }

  // Was die Anmeldungen des Tabs am Platz halten — sie geben ihn erst frei, wenn ihre Nachricht gesendet ist.
  function eigeneAngemeldeteBytes() {
    return stand.eintraege.flatMap((eintrag) => eintrag.dateien).filter((datei) => datei.id !== null).reduce((summe, datei) => summe + zaehlmass(datei.bytes), 0);
  }

  // Kommt die Anmeldung erst nach „Abbrechen“ oder „Verwerfen“ an, wird sie sofort wieder freigegeben — außer die
  // Hauptansicht ist dem Beitreten-Bildschirm gewichen (das alte Token gilt nicht mehr, sie verfällt am Server).
  function anmeldungNachAbbruchFreigeben(datei, ergebnis) {
    if (ergebnis.art === 'angemeldet' && datei.freigebenNachAbbruch) {
      anmeldungFreigeben(ergebnis.id);
    }
  }

  // Ohne Verbindung wartet die Freigabe (ein DELETE ginge ins Leere und die Anmeldung bliebe reserviert) und geht mit dem
  // Wiederverbinden hinaus (Code-Review #43 F0119 Sollte 3).
  const wartendeFreigaben = [];
  function anmeldungFreigeben(id) {
    if (stand.verbunden) {
      verwerfen(id);
      return;
    }
    wartendeFreigaben.push(id);
  }

  return {
    dateiAnmelden,
    anmeldungFreigeben,
    wartendeFreigabenSenden: () => wartendeFreigaben.splice(0).forEach(verwerfen),
  };
}

// Der PUT unter einem Platz mit Balken, sein Ausgang und „Erneut hochladen“.
function hochladungAnlegen({ stand, teile, hochladen, plaetze, zeitgeber }) {
  // Unter einem Platz (höchstens 2 je Browser) der PUT mit Balken; bis zum Platz „wartet“. Eine Ausnahme beim Hochladen
  // gibt den Platz frei und endet als Fehlschlag mit „Erneut hochladen“ (Code-Review #43 F0101 A4).
  function dateiHochladen(eintrag, datei) {
    datei.zustand = 'wartetAufPlatz';
    datei.prozent = 0;
    teile.ansicht.dateiZeigen(eintrag, datei);
    plaetze(async () => {
      if (!eintrag.dateien.includes(datei)) {
        return;
      }
      if (!stand.verbunden) {
        teile.verbindung.aufVerbindungWarten(eintrag, datei);
        return;
      }
      datei.zustand = 'laedt';
      teile.ansicht.dateiZeigen(eintrag, datei);
      datei.laufend = hochladen(datei.id, datei.datei, (geladen, gesamt) => {
        datei.prozent = prozentAus(geladen, gesamt);
        teile.ansicht.balkenZeigen(datei);
      });
      const antwort = await datei.laufend.ergebnis;
      const ergebnis = hochladenBewerten(antwort);
      datei.laufend = null;
      if (!eintrag.dateien.includes(datei) || ergebnis.art === 'abgebrochen') {
        return;
      }
      if (antwort.art === 'netzfehler' && !stand.verbunden) {
        teile.verbindung.aufVerbindungWarten(eintrag, datei);
        return;
      }
      hochladenAuswerten(eintrag, datei, ergebnis);
    }).catch((ausnahme) => {
      datei.laufend = null;
      if (eintrag.dateien.includes(datei)) {
        teile.ansicht.dateiScheitert(eintrag, datei, 'fehlgeschlagen', `Nicht hochgeladen (${ausnahme.message}).`);
      }
    });
  }

  function hochladenAuswerten(eintrag, datei, ergebnis) {
    if (ergebnis.art === 'nochEinmal') {
      nochEinmalHochladen(eintrag, datei);
      return;
    }
    if (ergebnis.art === 'fertig') {
      datei.zustand = 'fertig';
      datei.neuAngemeldet = false;
      datei.nochEinmalVersuche = 0;
      teile.ansicht.dateiZeigen(eintrag, datei);
      teile.eintraege.sendenWennFertig(eintrag);
      return;
    }
    if (ergebnis.art === 'neuAnmelden' && !datei.neuAngemeldet) {
      datei.neuAngemeldet = true;
      datei.id = null;
      teile.anmeldung.dateiAnmelden(eintrag, datei);
      return;
    }
    if (ergebnis.art === 'neuAnmelden') {
      teile.ansicht.dateiScheitert(eintrag, datei, 'fehlgeschlagen', ergebnis.grund);
      return;
    }
    teile.ansicht.dateiScheitert(eintrag, datei, ergebnis.art, ergebnis.grund);
  }

  // Der Server hält den abgebrochenen PUT dieser Anmeldung noch fest (409 `laeuft`, längstens die Stückfrist von 10 s):
  // die Datei wartet und lädt nach kurzer Zeit dieselbe Anmeldung neu; hält er sie länger, scheitert sie mit seinem Text.
  function nochEinmalHochladen(eintrag, datei) {
    const NOCH_EINMAL_MS = 2000;
    const VERSUCHE_HOECHSTENS = 8;
    datei.nochEinmalVersuche += 1;
    if (datei.nochEinmalVersuche > VERSUCHE_HOECHSTENS) {
      teile.ansicht.dateiScheitert(eintrag, datei, 'fehlgeschlagen', 'Für diese Anmeldung läuft noch ein Hochladen.');
      return;
    }
    datei.zustand = 'wartetAufPlatz';
    teile.ansicht.dateiZeigen(eintrag, datei);
    datei.zeitgeber = zeitgeber.stellen(NOCH_EINMAL_MS, () => dateiHochladen(eintrag, datei));
  }

  // „Erneut hochladen“: dieselbe Anmeldung neu — scheiterte schon die Anmeldung, eine neue —, der Fokus auf
  // „Abbrechen“ derselben Datei.
  function erneut(eintrag, datei) {
    datei.neuAngemeldet = false;
    datei.nochEinmalVersuche = 0;
    if (datei.id === null) {
      teile.anmeldung.dateiAnmelden(eintrag, datei);
    } else {
      dateiHochladen(eintrag, datei);
    }
    datei.schalter.querySelector('button').focus();
  }

  return { dateiHochladen, erneut };
}

// Verbindung weg und wieder da (B0520).
function verbindungAnlegen({ stand, teile }) {
  function aufVerbindungWarten(eintrag, datei) {
    datei.neuAngemeldet = false;
    datei.zustand = 'wartetAufVerbindung';
    teile.ansicht.dateiZeigen(eintrag, datei);
  }

  // Die Verbindung ist weg (B0520): ein laufender PUT bricht ab und wartet — sein Abbruch ist kein Fehlschlag —, ebenso
  // eine Datei, die eben ohne Antwort scheiterte; „Erneut hochladen“ gibt es nur bei bestehender Verbindung.
  function getrennt() {
    stand.verbunden = false;
    for (const eintrag of stand.eintraege) {
      for (const datei of eintrag.dateien) {
        const ohneAntwortGescheitert = datei.zustand === 'fehlgeschlagen' && datei.grund === KEINE_ANTWORT;
        if (datei.zustand === 'laedt' || ohneAntwortGescheitert) {
          aufVerbindungWarten(eintrag, datei);
        }
        if (datei.laufend !== null) {
          datei.laufend.abbrechen();
        }
      }
    }
  }

  // „Wieder verbunden“: jede wartende Datei meldet sich neu an bzw. lädt dieselbe Anmeldung neu (nach `abgelaufen`
  // meldet dateiHochladen neu an).
  function wiederVerbunden() {
    stand.verbunden = true;
    teile.anmeldung.wartendeFreigabenSenden();
    for (const eintrag of stand.eintraege) {
      for (const datei of eintrag.dateien.filter((wartend) => wartend.zustand === 'wartetAufVerbindung')) {
        if (datei.id === null) {
          teile.anmeldung.dateiAnmelden(eintrag, datei);
        } else {
          teile.hochladung.dateiHochladen(eintrag, datei);
        }
      }
    }
  }

  return { aufVerbindungWarten, getrennt, wiederVerbunden };
}

// Die Einträge selbst: beginnen, Datei oder Eintrag verlassen, senden, anhalten.
function eintraegeAnlegen({ stand, teile, bereich, senden, zurueckgeben, ansagen, fokusVerloren, zeitgeber }) {
  function zeigen() {
    bereich.replaceChildren(...stand.eintraege.filter((eintrag) => eintrag.channel === stand.offenerChannel).map((eintrag) => eintrag.artikel));
  }

  // Ein Absenden mit Dateien: Leerraum allein wird zum leeren Text (B5).
  function beginnen({ text, bezug, dateien }) {
    let textFuerSenden = text;
    if (nurLeerraum(text)) {
      textFuerSenden = '';
    }
    const eintrag = { channel: stand.offenerChannel, text: textFuerSenden, bezug, dateien: [], artikel: null, liste: null, zustandText: null };
    eintrag.artikel = teile.ansicht.eintragZeichnen(eintrag);
    eintrag.dateien = dateien.map((datei) => teile.ansicht.dateiAnlegen(eintrag, datei));
    stand.eintraege.push(eintrag);
    zeigen();
    eintrag.dateien.forEach((datei) => teile.anmeldung.dateiAnmelden(eintrag, datei));
  }

  // „Abbrechen“: der PUT bricht ab, die Anmeldung wird freigegeben, die Datei verlässt den Eintrag.
  function dateiAbbrechen(eintrag, datei) {
    dateiFreigeben(datei);
    dateiEntfernen(eintrag, datei);
  }

  function dateiFreigeben(datei) {
    zeitgeber.loeschen(datei.zeitgeber);
    if (datei.laufend !== null) {
      datei.laufend.abbrechen();
    }
    if (datei.id !== null) {
      teile.anmeldung.anmeldungFreigeben(datei.id);
    }
  }

  // „Verwerfen“ (U2): alle Hochladungen brechen ab, Text und Bezug gehen zurück in die Eingabe.
  function eintragVerwerfen(eintrag) {
    const dateien = eintrag.dateien;
    eintrag.dateien = [];
    dateien.forEach(dateiFreigeben);
    eintragBeenden(eintrag);
    zurueckgeben({ text: eintrag.text, bezug: eintrag.bezug });
  }

  // Eine Datei verlässt den Eintrag; der Fokus geht auf einen Schalter des Eintrags, sonst ins Feld (nie auf body).
  // War es die letzte, verschwindet der Eintrag — mit Text wird der Text gesendet.
  function dateiEntfernen(eintrag, datei) {
    zeitgeber.loeschen(datei.zeitgeber);
    const stelle = eintrag.dateien.indexOf(datei);
    eintrag.dateien = eintrag.dateien.filter((andere) => andere !== datei);
    datei.knoten.remove();
    teile.ansicht.kopfZeigen(eintrag);
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
    if (!alleOben || !stand.eintraege.includes(eintrag)) {
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
    stand.eintraege.splice(stand.eintraege.indexOf(eintrag), 1);
    eintrag.artikel.remove();
  }

  // Die Hauptansicht weicht dem Beitreten-Bildschirm (R00030 F0119 B0519): jede Hochladung hält an, ohne die Anmeldung
  // freizugeben — das alte Token gilt nicht mehr, die Anmeldungen verfallen am Server von selbst.
  function alleAnhalten() {
    for (const eintrag of [...stand.eintraege]) {
      const dateien = eintrag.dateien;
      eintrag.dateien = [];
      for (const datei of dateien) {
        datei.freigebenNachAbbruch = false;
        zeitgeber.loeschen(datei.zeitgeber);
        if (datei.laufend !== null) {
          datei.laufend.abbrechen();
        }
      }
      eintragBeenden(eintrag);
    }
  }

  return { zeigen, beginnen, verwerfen: eintragVerwerfen, entfernen: dateiEntfernen, abbrechen: dateiAbbrechen, sendenWennFertig, alleAnhalten };
}

function zustandBeschriftung(datei) {
  if (datei.zustand === 'wartet') {
    return wartetText(datei.sekunden, datei.platzBelegt);
  }
  if (datei.zustand === 'wartetAufPlatz') {
    return 'wartet';
  }
  if (datei.zustand === 'wartetAufVerbindung') {
    return 'wartet auf Verbindung';
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
  const ohneSperren = globalThis.navigator === undefined || globalThis.navigator.locks === undefined;
  if (ohneSperren) {
    return plaetzeImTab(PLAETZE.length);
  }
  const sperren = globalThis.navigator.locks;
  return (arbeit) => new Promise((fertig, gescheitert) => {
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
          fertig();
        } catch (ausnahme) {
          gescheitert(ausnahme);
        }
      }).catch((ausnahme) => {
        const zurueckgezogen = ausnahme.name === 'AbortError';
        if (!zurueckgezogen) {
          gescheitert(ausnahme);
        }
      });
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
  return (arbeit) => new Promise((fertig, gescheitert) => {
    const starten = () => {
      frei -= 1;
      laufen(arbeit).then(fertig, gescheitert);
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

// Eine auf die Seite gezogene Datei lässt die Seite unverändert, bis Ablegen gebaut ist (R00030 „Abgelegte Datei“,
// F0104): dragover und drop mit Dateien verhindern die Standardaktion — keine Navigation; angehängt wird über „Anhang“.
export function ablegenAbfangen(wurzel, signal) {
  for (const art of ['dragover', 'drop']) {
    wurzel.addEventListener(art, (ereignis) => {
      const mitDateien = ereignis.dataTransfer !== null && [...ereignis.dataTransfer.types].includes('Files');
      if (mitDateien) {
        ereignis.preventDefault();
      }
    }, { signal });
  }
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
