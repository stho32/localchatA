// Eine Nachricht im Verlauf zeichnen (Designsystem components/Nachricht, ADR-0026): Raster mit linker Spalte
// lc-gutter (Kachel bzw. Uhrzeit) und Inhalt; Kopf mit Kennung name@herkunft in lc-id und Uhrzeit HH:MM in lc-time,
// Datum mit Sekunden im title. Der Text steht nur als Text (textContent), Zeilenumbrüche bleiben (CSS pre-wrap);
// Markdown kommt mit I0017. Die Art des Absenders (Baustein Kennung): Kachel und Name in kd-agent mit der Marke
// „Agent“ für Agenten, in kd-mensch ohne Marke für Menschen und Sätze ohne Art.
import { kuerzel } from './kopfzeile.js';
import { uhrzeit, zeitTitel, datumBenennen, datumUhrzeit } from './gliederung.js';

// nachricht: { zeit, von, art, text, zustand? } aus der API; ohne Zeit (eigene, noch unterwegs) ohne Uhrzeit im Kopf
// und ohne zugänglichen Namen (sie ist keine Station des Rollfokus, F0141). Gilt für die Nachricht mindestens eine
// Aktion (aktionenFuer mit der eigenen Kennung), trägt sie die Leiste der Aktionen.
export function nachrichtZeichnen(nachricht, eigeneKennung = '') {
  const artikel = element('article', { className: 'lc-msg' });
  artikel.dataset.von = nachricht.von;
  artikel.dataset.art = darstellungsart(nachricht.art);
  if (nachricht.zustand !== undefined) {
    artikel.dataset.zustand = nachricht.zustand;
  }
  const darstellung = inhaltDarstellen(nachricht);
  const text = textZeichnen(darstellung);
  if (nachricht.zeit !== undefined) {
    artikel.dataset.zeit = nachricht.zeit;
    artikel.setAttribute('aria-label', zugaenglicherName(nachricht));
    text.id = `lc-text-${nachricht.zeit}`;
  }
  const inhalt = element('div', { className: 'lc-msg-inhalt' }, [text]);
  if (darstellung.marke === 'bearbeitet') {
    inhalt.append(markeBearbeitetZeichnen(nachricht.zeit, darstellung.zuletztBearbeitet));
  }
  artikel.append(element('div', { className: 'g' }), inhalt);
  const aktionen = aktionenFuer(nachricht, eigeneKennung);
  if (aktionen.length > 0) {
    artikel.append(...aktionenZeichnen(aktionen, nachricht.zeit));
  }
  kopfSetzen(artikel, true);
  return artikel;
}

// Die Leiste der Aktionen (Designsystem Nachricht): am Desktop rechts oben bei Hover und Fokus; auf Touch-Geräten statt
// dessen der Schalter „⋯“ mit dem Namen „Aktionen“, der sie als Menü öffnet (UX M6, PO 125) — welcher Weg gilt,
// entscheidet das CSS nach der Zeigerart. Die Knöpfe tragen ihre Aktion in data-aktion, der Verlauf führt sie aus.
// → [Schalter, Leiste]
function aktionenZeichnen(aktionen, zeit) {
  const BESCHRIFTUNG = { kopieren: 'Kopieren', bearbeiten: 'Bearbeiten', loeschen: 'Löschen' };
  const knoepfe = aktionen.map((aktion) => {
    const knopf = element('button', { type: 'button', className: 'lc-aktion' });
    knopf.dataset.aktion = aktion;
    knopf.append(element('span', { className: 'lc-aktion-anzeige', textContent: BESCHRIFTUNG[aktion] }));
    if (aktion === 'kopieren') {
      // Der sichtbare Text wechselt kurz auf die Rückmeldung (kopierRueckmeldungZeigen); der Name bleibt „Kopieren“.
      knopf.firstChild.setAttribute('aria-hidden', 'true');
      knopf.setAttribute('aria-label', BESCHRIFTUNG[aktion]);
    }
    return knopf;
  });
  const leiste = element('div', { className: 'lc-aktionen', id: `lc-aktionen-${zeit}` }, knoepfe);
  const schalter = element('button', { type: 'button', className: 'lc-aktionen-schalter', textContent: '⋯' });
  schalter.setAttribute('aria-label', 'Aktionen');
  schalter.setAttribute('aria-expanded', 'false');
  schalter.setAttribute('aria-controls', leiste.id);
  return [schalter, leiste];
}

const KOPIER_ANZEIGE = { kopiert: '✓ Kopiert.', nicht_moeglich: '⚠ Kopieren nicht möglich' };
const kopierAnzeigeTimer = new WeakMap();

// Rückmeldung am Knopf „Kopieren“ der Nachricht (R00046 I0042, UX-Vorprüfung): die Beschriftung wechselt für `dauerMs` auf
// „✓ Kopiert.“ bzw. „⚠ Kopieren nicht möglich“ (Fehler in kd-danger, per CSS am Attribut); erneutes Kopieren beginnt die
// Zeit neu. Die Breite ist per CSS für „✓ Kopiert.“ vorab reserviert.
export function kopierRueckmeldungZeigen(artikel, anzeige, dauerMs) {
  const knopf = artikel.querySelector('.lc-aktion[data-aktion="kopieren"]');
  if (knopf === null) {
    return;
  }
  const sichtbar = knopf.querySelector('.lc-aktion-anzeige');
  sichtbar.textContent = KOPIER_ANZEIGE[anzeige];
  knopf.dataset.rueckmeldung = anzeige;
  clearTimeout(kopierAnzeigeTimer.get(knopf));
  kopierAnzeigeTimer.set(knopf, setTimeout(() => {
    sichtbar.textContent = 'Kopieren';
    delete knopf.dataset.rueckmeldung;
  }, dauerMs));
}

// Was eine Nachricht zeigt (R00041 F0134, ADR-0048 Regel 4): ihren Text und — bearbeitet — die Marke „bearbeitet“ mit
// der Zeit der letzten Änderung („Zuletzt bearbeitet: TT.MM.JJJJ, HH:MM“ im Versatz der Änderung, PO 123); die frühere
// Fassung kennt der Tab nicht. Gelöscht (F0138): kein Text, keine Anhänge, die Marke „Nachricht gelöscht“.
// → { text: Text | null, marke: null | 'bearbeitet' | 'geloescht', zuletztBearbeitet: Text | null }
export function inhaltDarstellen({ text, zustand, geaendert }) {
  if (zustand === 'geloescht') {
    return { text: null, marke: 'geloescht', zuletztBearbeitet: null };
  }
  if (zustand === 'bearbeitet') {
    return { text, marke: 'bearbeitet', zuletztBearbeitet: `Zuletzt bearbeitet: ${datumUhrzeit(geaendert)}` };
  }
  return { text, marke: null, zuletztBearbeitet: null };
}

// An der Stelle des Textes einer gelöschten Nachricht steht ihre Marke.
function textZeichnen(darstellung) {
  if (darstellung.marke === 'geloescht') {
    return element('div', { className: 'lc-body lc-fold lc-marke', textContent: 'Nachricht gelöscht' });
  }
  return element('div', { className: 'lc-body lc-fold', textContent: darstellung.text });
}

// Die Marke „bearbeitet“ unter dem Text, auch an eingeklappten und gruppierten Nachrichten (Designsystem Nachricht): ein
// Schalter (UX M8, PO 128), der „Zuletzt bearbeitet: …“ ein- und ausblendet — mit der Maus steht derselbe Text als title,
// der Bildschirmleser hört ihn als Beschreibung.
function markeBearbeitetZeichnen(zeit, zuletztBearbeitet) {
  const zuletzt = element('span', { className: 'lc-zuletzt', id: `lc-zuletzt-${zeit}`, textContent: zuletztBearbeitet, hidden: true });
  const marke = element('button', { type: 'button', className: 'lc-marke', textContent: 'bearbeitet', title: zuletztBearbeitet });
  marke.setAttribute('aria-expanded', 'false');
  marke.setAttribute('aria-describedby', zuletzt.id);
  return element('div', { className: 'lc-zusatz' }, [marke, zuletzt]);
}

// Blendet die Zeit der letzten Änderung an der Marke ein oder aus.
export function markeUmschalten(marke) {
  const offen = marke.getAttribute('aria-expanded') !== 'true';
  marke.setAttribute('aria-expanded', String(offen));
  document.getElementById(marke.getAttribute('aria-describedby')).hidden = !offen;
}

// Eigene Nachricht (R00041 F0135, B0608): `von` ist die eigene Kennung — auch aus einer früheren Sitzung —, bestätigt
// (nur bestätigte tragen eine Zeit; „wird gesendet …“ und fehlgeschlagene nicht) und nicht gelöscht.
export function bearbeitbar({ zeit, von, zustand }, eigeneKennung) {
  return eigeneKennung !== '' && von === eigeneKennung && zeit !== undefined && zustand !== 'geloescht';
}

// Den Text einer Nachricht kopieren (R00046 I0042, ADR-0052 Regel 1): jede bestätigte, nicht gelöschte Nachricht mit
// Text — eigene, fremde, von Agenten; je Aktion gebunden (PO 145), anders als bearbeitbar.
export function kopierbar({ zeit, text = '', zustand }) {
  return zeit !== undefined && zustand !== 'geloescht' && text !== '';
}

// Die Aktionen einer Nachricht in der Reihenfolge der Leiste (Designsystem Nachricht: Antworten · Kopieren · Bearbeiten
// · Löschen); jede Aktion trägt ihre eigene Bedingung (PO 145): Kopieren gilt an jeder kopierbaren Nachricht, Bearbeiten
// und Löschen nur an der eigenen.
export function aktionenFuer(nachricht, eigeneKennung) {
  const eigen = bearbeitbar(nachricht, eigeneKennung);
  return [['kopieren', kopierbar(nachricht)], ['bearbeiten', eigen], ['loeschen', eigen]]
    .filter(([, gilt]) => gilt).map(([aktion]) => aktion);
}

const NAME_TEXTANFANG_ZEICHEN = 120;

// Der zugängliche Name einer Nachricht im Verlauf (F0141, B0632): „<Kennung>, <HH:MM>[, bearbeitet | , Nachricht
// gelöscht]: <Textanfang>“; der Textanfang ist eine Zeile und endet nach 120 Zeichen mit „…“, eine gelöschte
// Nachricht nennt statt seiner die Marke.
export function zugaenglicherName({ von, zeit, text = '', zustand }) {
  const kopf = `${von}, ${uhrzeit(zeit)}`;
  if (zustand === 'geloescht') {
    return `${kopf}, Nachricht gelöscht`;
  }
  let marke = '';
  if (zustand === 'bearbeitet') {
    marke = ', bearbeitet';
  }
  const anfang = textanfang(text);
  if (anfang === '') {
    return `${kopf}${marke}`;
  }
  return `${kopf}${marke}: ${anfang}`;
}

function textanfang(text) {
  const eineZeile = text.replace(/\s+/gu, ' ').trim();
  const zeichen = Array.from(eineZeile);
  if (zeichen.length <= NAME_TEXTANFANG_ZEICHEN) {
    return eineZeile;
  }
  return `${zeichen.slice(0, NAME_TEXTANFANG_ZEICHEN).join('')}…`;
}

// Mit Kopf: Kachel links, darüber Kennung und Uhrzeit; ohne Kopf: die Uhrzeit links (Gruppe).
export function kopfSetzen(artikel, mitKopf) {
  const gutter = artikel.querySelector('.g');
  const inhalt = artikel.querySelector('.lc-msg-inhalt');
  const vorhandenerKopf = inhalt.querySelector('.lc-meta');
  if (vorhandenerKopf !== null) {
    vorhandenerKopf.remove();
  }
  const zeit = artikel.dataset.zeit;
  const von = artikel.dataset.von;
  artikel.classList.toggle('first', mitKopf);
  if (!mitKopf) {
    gutter.replaceChildren(zeitElement(zeit));
    return;
  }
  const [name, herkunft] = von.split('@');
  const art = artikel.dataset.art;
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(name) });
  kachel.setAttribute('aria-hidden', 'true');
  gutter.replaceChildren(kachel);
  const kennung = element('span', { className: `lc-id ${art}` }, [name, element('i', { textContent: `@${herkunft}` })]);
  const kopf = [kennung];
  if (art === 'agent') {
    kopf.push(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  if (zeit !== undefined) {
    kopf.push(zeitElement(zeit));
  }
  inhalt.prepend(element('div', { className: 'lc-meta' }, kopf));
}

// Ein Satz ohne Art (von Hand, ältere Sätze) wird wie ein Mensch gezeigt.
function darstellungsart(art) {
  if (art === 'agent') {
    return 'agent';
  }
  return 'mensch';
}

// Lange Nachricht (Designsystem LangeNachricht): jeder Inhalt ist per CSS auf lc-collapse begrenzt; läuft er über,
// steht darunter die Leiste „Ganze Nachricht anzeigen · n Zeilen“. Gemessen wird erst, wenn alle Nachrichten eines
// Blocks im DOM stehen — einmal lesen, dann schreiben (ADR-0026 Regel 4).
export function langeEinklappen(artikel) {
  const ueberlang = artikel.filter((einzeln) => {
    const text = einzeln.querySelector('.lc-body');
    return text.scrollHeight > text.clientHeight;
  });
  for (const einzeln of ueberlang) {
    const text = einzeln.querySelector('.lc-body');
    const schalter = element('button', { type: 'button' });
    schalter.setAttribute('aria-controls', text.id);
    schalter.dataset.zeilen = String(text.textContent.split('\n').length);
    leisteBeschriften(schalter, false);
    einzeln.querySelector('.lc-msg-inhalt').append(element('div', { className: 'lc-foldbar' }, [schalter]));
  }
}

// Auf- oder zuklappen an Ort und Stelle; den Kopf hält der Aufrufer an seiner Bildschirmstelle.
export function langeUmschalten(schalter) {
  const offen = schalter.getAttribute('aria-expanded') !== 'true';
  document.getElementById(schalter.getAttribute('aria-controls')).classList.toggle('lc-offen', offen);
  leisteBeschriften(schalter, offen);
}

function leisteBeschriften(schalter, offen) {
  schalter.setAttribute('aria-expanded', String(offen));
  if (offen) {
    schalter.textContent = 'Einklappen';
    return;
  }
  schalter.textContent = leistenBeschriftung(Number(schalter.dataset.zeilen));
}

export function leistenBeschriftung(zeilen) {
  if (zeilen === 1) {
    return 'Ganze Nachricht anzeigen · 1 Zeile';
  }
  return `Ganze Nachricht anzeigen · ${zeilen} Zeilen`;
}

// Tagestrenner (Designsystem VerlaufsMarken): Datum zentriert zwischen zwei Haarlinien, klebt oben.
export function trennerZeichnen(datum, heute) {
  const trenner = element('div', { className: 'lc-day', textContent: datumBenennen(datum, heute) });
  trenner.setAttribute('role', 'separator');
  trenner.dataset.datum = datum;
  return trenner;
}

function zeitElement(zeit) {
  const zeitpunkt = element('time', { className: 'lc-time', dateTime: zeit, textContent: uhrzeit(zeit) });
  zeitpunkt.title = zeitTitel(zeit);
  return zeitpunkt;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
