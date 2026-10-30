// Eigene Nachricht löschen (R00041 F0139, ADR-0048 Regel 12): vor jedem Löschen genau eine Rückfrage, die bei Anhängen
// Zahl und Größe der mitgelöschten nennt (UX S1, PO 95).
import { uhrzeit } from './gliederung.js';
import { groesseInWorten } from './groesse.js';

// Die Texte der Rückfrage, rein: Frage (mit Zahl und Größe der Anhänge im Zustand `vorhanden`), darunter Uhrzeit und
// Textanfang (40 Zeichen, eine Zeile, längerer Text mit „…“; ohne Text nur die Uhrzeit), zuletzt die Warnung.
// → { frage, auszug, warnung }
export function rueckfrageText({ zeit, text, anhaenge = [] }) {
  return { frage: frageText(anhaenge), auszug: auszugText(zeit, text), warnung: 'Das lässt sich nicht rückgängig machen.' };
}

function frageText(anhaenge) {
  const vorhanden = anhaenge.filter((anhang) => anhang.zustand !== 'geloescht');
  const groesse = groesseInWorten(vorhanden.reduce((summe, anhang) => summe + anhang.bytes, 0));
  if (vorhanden.length === 0) {
    return 'Diese Nachricht löschen?';
  }
  if (vorhanden.length === 1) {
    return `Diese Nachricht mit 1 Anhang (${groesse}) löschen? Der Anhang wird mitgelöscht.`;
  }
  return `Diese Nachricht mit ${vorhanden.length} Anhängen (${groesse}) löschen? Die Anhänge werden mitgelöscht.`;
}

function auszugText(zeit, text) {
  const TEXTANFANG_ZEICHEN = 40;
  const zeichen = Array.from(text.replace(/\s+/gu, ' ').trim());
  if (zeichen.length === 0) {
    return uhrzeit(zeit);
  }
  let anfang = zeichen.join('');
  if (zeichen.length > TEXTANFANG_ZEICHEN) {
    anfang = `${zeichen.slice(0, TEXTANFANG_ZEICHEN).join('')}…`;
  }
  return `${uhrzeit(zeit)}: ${anfang}`;
}


// Was die Antwort des Löschens bedeutet (UX S2), rein: 200 (auch „schon gelöscht“) → die gelöschte Fassung; sonst der
// Text der Instanz mit ⚠, ohne Antwort „⚠ Keine Antwort vom Server beim Löschen.“
// → { art: 'geloescht', nachricht } | { art: 'abgewiesen', text }
export function loeschenAuswerten(antwort) {
  if (antwort.art !== 'antwort') {
    return { art: 'abgewiesen', text: '⚠ Keine Antwort vom Server beim Löschen.' };
  }
  if (antwort.status === 200) {
    return { art: 'geloescht', nachricht: antwort.daten.nachricht };
  }
  const mitText = antwort.daten !== null && typeof antwort.daten === 'object' && typeof antwort.daten.fehler === 'string';
  if (mitText) {
    return { art: 'abgewiesen', text: `⚠ ${antwort.daten.fehler}` };
  }
  return { art: 'abgewiesen', text: `⚠ Nicht gelöscht (Status ${antwort.status}).` };
}

// Die Rückfrage als modaler Dialog (UX M5, S7, PO 126): Frage als Name, Auszug und Warnung als Beschreibung, Knöpfe
// „Abbrechen“ · „Löschen“; der Startfokus steht auf „Abbrechen“, damit ein schnelles Enter nichts löscht; Tab bleibt
// im Dialog. Ein Klick auf „Löschen“ schließt sofort — ein Doppelklick bestätigt einmal. ort: Element, in das der Dialog
// kommt · bestaetigt() nach „Löschen“ · abgebrochen() nach „Abbrechen“ und Esc.
export function rueckfrageOeffnen(ort, nachricht, { bestaetigt, abgebrochen }) {
  const texte = rueckfrageText(nachricht);
  const frage = element('h2', { id: 'lc-rueckfrage-frage', className: 'lc-dialog-titel', textContent: texte.frage });
  const auszug = element('p', { id: 'lc-rueckfrage-auszug', className: 'lc-rueckfrage-auszug', textContent: texte.auszug });
  const warnung = element('p', { id: 'lc-rueckfrage-warnung', className: 'lc-rueckfrage-warnung', textContent: texte.warnung });
  const abbrechenKnopf = element('button', { type: 'button', className: 'lc-btn', textContent: 'Abbrechen' });
  const loeschenKnopf = element('button', { type: 'button', className: 'lc-btn lc-gefahr', textContent: 'Löschen' });
  const dialog = element('dialog', { className: 'lc-dialog lc-rueckfrage' }, [
    frage, auszug, warnung, element('div', { className: 'lc-dialog-schalter' }, [abbrechenKnopf, loeschenKnopf]),
  ]);
  dialog.setAttribute('aria-labelledby', frage.id);
  dialog.setAttribute('aria-describedby', `${auszug.id} ${warnung.id}`);
  let bestaetigung = false;
  abbrechenKnopf.addEventListener('click', () => dialog.close());
  loeschenKnopf.addEventListener('click', () => {
    bestaetigung = true;
    dialog.close();
  });
  dialog.addEventListener('keydown', (ereignis) => fokusImDialogHalten(ereignis, [abbrechenKnopf, loeschenKnopf]));
  dialog.addEventListener('close', () => {
    dialog.remove();
    if (bestaetigung) {
      bestaetigt();
      return;
    }
    abgebrochen();
  });
  ort.append(dialog);
  dialog.showModal();
  abbrechenKnopf.focus();
}

// Tab und Umschalt Tab wandern zwischen den Knöpfen des Dialogs, nie hinaus.
function fokusImDialogHalten(ereignis, knoepfe) {
  if (ereignis.key !== 'Tab') {
    return;
  }
  ereignis.preventDefault();
  const stelle = knoepfe.indexOf(document.activeElement);
  let naechste = (stelle + 1) % knoepfe.length;
  if (ereignis.shiftKey) {
    naechste = (stelle - 1 + knoepfe.length) % knoepfe.length;
  }
  knoepfe[naechste].focus();
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
