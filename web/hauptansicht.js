// Hauptansicht (Designsystem: Bildschirm 4.2, Baustein Hauptansicht): Kopfzeile, Mitte und rechts die Region
// „Teilnehmer“. Tab-Reihenfolge: Kopfzeile → Mitte → Teilnehmer. Der Hinweis auf eine unterbrochene
// Verbindung steht in Fehlergestalt (⚠ und Wort, kd-danger) unter der Kopfzeile über die ganze Breite. `Strg .` und der Schalter
// „Teilnehmer“ blenden die Liste ein und aus (1-tastatur.md).
import * as kopfzeile from './kopfzeile.js';

const UNTERBROCHEN = '⚠ Verbindung unterbrochen — localchatA verbindet neu.';
const STAND_UNBEKANNT = 'Stand unbekannt — zuletzt bekannte Liste.';
let vorigeAnsichtAbmelden = new AbortController();

export function zeichnen(wurzel, teilnehmer, optionen) {
  vorigeAnsichtAbmelden.abort();
  vorigeAnsichtAbmelden = new AbortController();
  const anzahl = element('span', { className: 'lc-count', textContent: '0' });
  const schalter = element('button', { type: 'button', className: 'lc-hbtn' }, ['Teilnehmer ', anzahl]);
  schalter.setAttribute('aria-controls', 'lc-teilnehmer');
  kopfzeile.zeichnen(wurzel, teilnehmer, { ...optionen, schalter: [schalter] });
  const hinweis = element('p', { id: 'lc-verbindung', className: 'lc-meldung lc-fehler lc-verbindung', hidden: true });
  hinweis.setAttribute('role', 'alert');
  const standUnbekannt = element('p', { className: 'lc-stand', hidden: true });
  const zeilen = element('div', { className: 'lc-members-zeilen' });
  const region = element('aside', { id: 'lc-teilnehmer', className: 'lc-members', tabIndex: 0 }, [standUnbekannt, zeilen]);
  region.setAttribute('aria-label', 'Teilnehmer');
  const mitte = element('main', { className: 'lc-center' });
  const haupt = element('div', { className: 'lc-main' }, [mitte, region]);
  wurzel.classList.add('lc-app');
  wurzel.append(hinweis, haupt);
  einAusblendenBedienen({ schalter, region, haupt }, vorigeAnsichtAbmelden.signal);
  let letzterZustand = 'verbunden';
  return {
    region: zeilen,
    anzahlZeigen: (wieViele) => {
      anzahl.textContent = String(wieViele);
    },
    verbindungZeigen: (zustand) => {
      letzterZustand = verbindungZeigen({ hinweis, region, standUnbekannt }, letzterZustand, zustand);
    },
  };
}

// Unter 720 px legt sich die eingeblendete Liste als Fläche über die Mitte (Gestaltungsfrage 1): beim Öffnen
// geht der Fokus in die Liste, Esc und der Schalter schließen sie, der Fokus kehrt auf den Schalter zurück.
const UEBERLAGERT_UNTER_PX = 720;

// Der Wunsch des Menschen gilt bis zum Neuladen; ohne ihn folgt die Liste der Fensterbreite. Liegt der Fokus
// beim Ausblenden in der Liste — auch wenn sie durch Verkleinern einklappt —, geht er auf den Schalter.
function einAusblendenBedienen({ schalter, region, haupt }, signal) {
  let wunsch = null;
  function anwenden() {
    const sichtbar = teilnehmerSichtbar(window.innerWidth, wunsch);
    if (!sichtbar && region.contains(document.activeElement)) {
      schalter.focus();
    }
    region.hidden = !sichtbar;
    haupt.classList.toggle('ohne-teilnehmer', !sichtbar);
    schalter.setAttribute('aria-expanded', String(sichtbar));
  }
  function umschalten() {
    const warSichtbar = !region.hidden;
    const ueberlagert = window.innerWidth < UEBERLAGERT_UNTER_PX;
    wunsch = 'ein';
    if (warSichtbar) {
      wunsch = 'aus';
    }
    anwenden();
    if (warSichtbar && ueberlagert) {
      schalter.focus();
    }
    if (!warSichtbar && ueberlagert) {
      region.focus();
    }
  }
  schalter.addEventListener('click', umschalten, { signal });
  document.addEventListener('keydown', (ereignis) => {
    const strgPunkt = (ereignis.ctrlKey || ereignis.metaKey) && ereignis.key === '.';
    const ueberlagerteListeOffen = !region.hidden && window.innerWidth < UEBERLAGERT_UNTER_PX;
    const schliessen = ereignis.key === 'Escape' && ueberlagerteListeOffen;
    if (strgPunkt || schliessen) {
      ereignis.preventDefault();
      umschalten();
    }
  }, { signal });
  window.matchMedia(`(min-width: ${SICHTBAR_AB_PX}px)`).addEventListener('change', anwenden, { signal });
  anwenden();
}

// Designsystem README: unter 1100 px klappt die Teilnehmerliste ein, damit der Chat neben die IDE passt; hat der
// Mensch sie ein- oder ausgeblendet (wunsch 'ein' | 'aus'), gilt das bis zum Neuladen, sonst die Breite.
// Die Grenze steht nur hier (die Media-Query wird aus ihr gebildet, J06).
const SICHTBAR_AB_PX = 1100;

export function teilnehmerSichtbar(fensterbreite, wunsch) {
  if (wunsch === 'ein') {
    return true;
  }
  if (wunsch === 'aus') {
    return false;
  }
  return fensterbreite >= SICHTBAR_AB_PX;
}

// Der Alarmtext wird nur beim Wechsel gesetzt, damit ein Screenreader ihn nicht bei jedem Versuch neu ansagt.
// Solange die Verbindung fehlt, ist die Liste als „Stand unbekannt“ markiert: gedämpft, mit einem Satz, und
// kein ● gilt mehr als sicher online. Liefert den nun gültigen Zustand.
function verbindungZeigen({ hinweis, region, standUnbekannt }, letzterZustand, zustand) {
  if (zustand === 'unterbrochen' && letzterZustand !== 'unterbrochen') {
    hinweis.textContent = UNTERBROCHEN;
    hinweis.hidden = false;
    standUnbekannt.textContent = STAND_UNBEKANNT;
    standUnbekannt.hidden = false;
    region.classList.add('lc-unbekannt');
    return 'unterbrochen';
  }
  if (zustand === 'verbunden' && letzterZustand !== 'verbunden') {
    hinweis.textContent = '';
    hinweis.hidden = true;
    standUnbekannt.hidden = true;
    region.classList.remove('lc-unbekannt');
    return 'verbunden';
  }
  return letzterZustand;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
