// Hauptansicht (Designsystem: Bildschirm 4.2, Baustein Hauptansicht): Kopfzeile, Mitte und rechts die Region
// „Teilnehmer“. Tab-Reihenfolge: Kopfzeile → Mitte → Teilnehmer. Der Hinweis auf eine unterbrochene
// Verbindung steht in Fehlergestalt (⚠ und Wort, kd-danger) oben in der Mitte. `Strg .` und der Schalter
// „Teilnehmer“ blenden die Liste ein und aus (1-tastatur.md).
import * as kopfzeile from './kopfzeile.js';

const UNTERBROCHEN = '⚠ Verbindung unterbrochen — localchatA verbindet neu.';
let vorigeAnsichtAbmelden = new AbortController();

export function zeichnen(wurzel, teilnehmer, optionen) {
  vorigeAnsichtAbmelden.abort();
  vorigeAnsichtAbmelden = new AbortController();
  const schalter = element('button', { type: 'button', className: 'lc-hbtn', textContent: 'Teilnehmer' });
  schalter.setAttribute('aria-controls', 'lc-teilnehmer');
  kopfzeile.zeichnen(wurzel, teilnehmer, { ...optionen, schalter: [schalter] });
  const hinweis = element('p', { id: 'lc-verbindung', className: 'lc-meldung lc-fehler', hidden: true });
  hinweis.setAttribute('role', 'alert');
  const region = element('aside', { id: 'lc-teilnehmer', className: 'lc-members', tabIndex: 0 });
  region.setAttribute('aria-label', 'Teilnehmer');
  const mitte = element('main', { className: 'lc-center' }, [hinweis]);
  const haupt = element('div', { className: 'lc-main' }, [mitte, region]);
  wurzel.classList.add('lc-app');
  wurzel.append(haupt);
  einAusblendenBedienen({ schalter, region, haupt }, vorigeAnsichtAbmelden.signal);
  return { region, verbindungZeigen: (zustand) => verbindungZeigen(hinweis, zustand) };
}

// Der Wunsch des Menschen gilt bis zum Neuladen; ohne ihn folgt die Liste der Fensterbreite. Liegt der Fokus
// beim Ausblenden in der Liste, geht er auf den Schalter — nie verloren an den Seitenanfang.
function einAusblendenBedienen({ schalter, region, haupt }, signal) {
  let wunsch = null;
  function anwenden() {
    const sichtbar = teilnehmerSichtbar(window.innerWidth, wunsch);
    region.hidden = !sichtbar;
    haupt.classList.toggle('ohne-teilnehmer', !sichtbar);
    schalter.setAttribute('aria-expanded', String(sichtbar));
  }
  function umschalten() {
    const warSichtbar = !region.hidden;
    if (warSichtbar && region.contains(document.activeElement)) {
      schalter.focus();
    }
    wunsch = 'ein';
    if (warSichtbar) {
      wunsch = 'aus';
    }
    anwenden();
  }
  schalter.addEventListener('click', umschalten, { signal });
  document.addEventListener('keydown', (ereignis) => {
    const strgPunkt = (ereignis.ctrlKey || ereignis.metaKey) && ereignis.key === '.';
    if (strgPunkt) {
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

function verbindungZeigen(hinweis, zustand) {
  if (zustand === 'unterbrochen') {
    hinweis.textContent = UNTERBROCHEN;
    hinweis.hidden = false;
  }
  if (zustand === 'verbunden') {
    hinweis.textContent = '';
    hinweis.hidden = true;
  }
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
