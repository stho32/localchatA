// Gewählte Dateien über dem Feld (R00030 F0101 B0449): je Channel im Speicher des Tabs, höchstens 10 je Nachricht — so
// viele offene Hochladungen erlaubt die Instanz je Kennung (R00029). Eine Liste role="list" „Angehängte Dateien“, je
// Datei Typkürzel, Name, Größe in Worten und „Entfernen“; jede Auswahl wird einmal über die eine Ansage-Region gesagt.
import { groesseInWorten } from './loeschen.js';

export const HOECHSTENS_DATEIEN = 10;

// Rein: die ersten gewählten bis zur Zehn kommen dazu. → { dateien, uebernommen, nichtUebernommen }
export function uebernehmen(bisher, gewaehlt) {
  const platz = Math.max(0, HOECHSTENS_DATEIEN - bisher.length);
  const uebernommen = gewaehlt.slice(0, platz);
  return { dateien: [...bisher, ...uebernommen], uebernommen, nichtUebernommen: gewaehlt.length - uebernommen.length };
}

// Die Ansage einer Auswahl (UX M2): „<n> Dateien angehängt: <namen>“ bzw. „Datei angehängt: <name>“, bei mehr als zehn
// mit „ — <k> nicht übernommen, höchstens 10 je Nachricht“.
export function auswahlAnsage(namen, nichtUebernommen) {
  let text = `${namen.length} Dateien angehängt: ${namen.join(', ')}`;
  if (namen.length === 1) {
    text = `Datei angehängt: ${namen[0]}`;
  }
  if (namen.length === 0) {
    text = 'Keine Datei angehängt';
  }
  if (nichtUebernommen > 0) {
    text = `${text} — ${nichtUebernommen} nicht übernommen, höchstens ${HOECHSTENS_DATEIEN} je Nachricht`;
  }
  return text;
}

// Die Hinweiszeile nach einer Auswahl über die Zehn hinaus; sonst leer.
export function uebernahmeHinweis(nichtUebernommen) {
  if (nichtUebernommen === 0) {
    return '';
  }
  return `Höchstens ${HOECHSTENS_DATEIEN} Dateien je Nachricht — ${nichtUebernommen} nicht übernommen.`;
}

// Das Typkürzel aus der letzten Endung, nur zur Anzeige: höchstens 5 Zeichen, groß; ohne Endung „DATEI“.
export function typkuerzel(name) {
  const HOECHSTENS_ZEICHEN = 5;
  const punkt = name.lastIndexOf('.');
  const endung = name.slice(punkt + 1);
  if (punkt <= 0 || endung === '') {
    return 'DATEI';
  }
  return Array.from(endung.toUpperCase()).slice(0, HOECHSTENS_ZEICHEN).join('');
}

// Die Liste im DOM. `geaendert({ hinweis })` erfährt jede Änderung (Hinweiszeile der Eingabe, Bereit-Regel); `ansagen`
// ist die eine Ansage-Region. Nach „Entfernen“ geht der Fokus auf das nächste „Entfernen“ (sonst das vorige), ohne
// Zeile ins Feld.
export function dateilisteAnlegen({ feld, ansagen, geaendert }) {
  const liste = element('ul', { className: 'lc-dateien', hidden: true });
  liste.setAttribute('role', 'list');
  liste.setAttribute('aria-label', 'Angehängte Dateien');
  const jeChannel = new Map();
  let channel = null;

  function dateien() {
    return jeChannel.get(channel) || [];
  }

  function zeigen(neue, hinweis) {
    jeChannel.set(channel, neue);
    liste.replaceChildren(...neue.map(zeileZeichnen));
    liste.hidden = neue.length === 0;
    geaendert({ hinweis });
  }

  function hinzufuegen(gewaehlt) {
    const ergebnis = uebernehmen(dateien(), gewaehlt);
    zeigen(ergebnis.dateien, uebernahmeHinweis(ergebnis.nichtUebernommen));
    ansagen({ art: 'sofort', text: auswahlAnsage(ergebnis.uebernommen.map((datei) => datei.name), ergebnis.nichtUebernommen) });
  }

  function entfernen(datei) {
    const bisher = dateien();
    const stelle = bisher.indexOf(datei);
    zeigen(bisher.filter((andere) => andere !== datei), '');
    const schalterDort = liste.querySelectorAll('.lc-datei-entfernen');
    if (schalterDort.length === 0) {
      feld.focus();
      return;
    }
    schalterDort[Math.min(stelle, schalterDort.length - 1)].focus();
  }

  function zeileZeichnen(datei) {
    const entfernenSchalter = element('button', { type: 'button', className: 'lc-datei-entfernen', textContent: 'Entfernen' });
    entfernenSchalter.setAttribute('aria-label', `${datei.name} entfernen`);
    entfernenSchalter.addEventListener('click', () => entfernen(datei));
    const kuerzel = element('span', { className: 'lc-datei-kuerzel', textContent: typkuerzel(datei.name) });
    kuerzel.setAttribute('aria-hidden', 'true');
    return element('li', { className: 'lc-datei' }, [
      kuerzel,
      element('span', { className: 'lc-datei-name', textContent: datei.name }),
      element('span', { className: 'lc-datei-groesse', textContent: groesseInWorten(datei.size) }),
      entfernenSchalter,
    ]);
  }

  return {
    element: liste,
    hinzufuegen,
    anzahl: () => dateien().length,
    // Beim Absenden: die gewählten Dateien des Channels, die Liste ist danach leer.
    nehmen: () => {
      const genommen = dateien();
      zeigen([], '');
      return genommen;
    },
    // Beim Wechsel bleibt die Liste des alten Channels im Speicher des Tabs; die des neuen erscheint.
    channelSetzen: (name) => {
      channel = name;
      zeigen(dateien(), '');
    },
  };
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
