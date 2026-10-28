// Hauptansicht (Designsystem: Bildschirm 4.2, Baustein Hauptansicht): Kopfzeile, Mitte und rechts die Region
// „Teilnehmer“. Tab-Reihenfolge: Kopfzeile → Mitte → Teilnehmer. Der Hinweis auf eine unterbrochene
// Verbindung steht in Fehlergestalt (⚠ und Wort, kd-danger) oben in der Mitte.
import * as kopfzeile from './kopfzeile.js';

const UNTERBROCHEN = '⚠ Verbindung unterbrochen — localchatA verbindet neu.';

export function zeichnen(wurzel, teilnehmer, optionen) {
  kopfzeile.zeichnen(wurzel, teilnehmer, optionen);
  const hinweis = element('p', { id: 'lc-verbindung', className: 'lc-meldung lc-fehler', hidden: true });
  hinweis.setAttribute('role', 'alert');
  const region = element('aside', { className: 'lc-members', tabIndex: 0 });
  region.setAttribute('aria-label', 'Teilnehmer');
  const mitte = element('main', { className: 'lc-center' }, [hinweis]);
  wurzel.classList.add('lc-app');
  wurzel.append(element('div', { className: 'lc-main' }, [mitte, region]));
  return { region, verbindungZeigen: (zustand) => verbindungZeigen(hinweis, zustand) };
}

// Designsystem README: unter 1100 px klappt die Teilnehmerliste ein, damit der Chat neben die IDE passt; hat der
// Mensch sie ein- oder ausgeblendet (wunsch 'ein' | 'aus'), gilt das bis zum Neuladen, sonst die Breite.
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
