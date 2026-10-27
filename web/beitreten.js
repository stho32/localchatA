// Beitreten-Bildschirm (Designsystem 4.1): Namensfeld mit Fokus, „Beitreten“, Hinweis beim Tippen,
// Zustände „Name vergeben“ (Vorschlag vorausgefüllt) und „Server nicht erreichbar“ mit „Erneut versuchen“.
// Text vom Server nur über textContent, nie als HTML.
import { pruefen } from './namensregel.js';

export function zeichnen(wurzel, zustand, aktionen) {
  const feld = element('input', {
    id: 'lc-name', className: 'lc-feld', type: 'text', autocomplete: 'nickname', spellcheck: false,
    value: feldinhalt(zustand),
  });
  feld.setAttribute('autocapitalize', 'none');
  feld.setAttribute('aria-describedby', 'lc-name-hinweis lc-meldung');
  const schalter = element('button', { type: 'submit', className: 'lc-btn primary', textContent: 'Beitreten' });
  const hinweis = element('p', { id: 'lc-name-hinweis', className: 'lc-hinweis' });
  hinweis.setAttribute('aria-live', 'polite');
  const meldung = element('div', { id: 'lc-meldung', className: 'lc-meldung' });
  meldung.setAttribute('role', 'alert');
  const formular = element('form', { className: 'lc-beitreten-karte', noValidate: true }, [
    element('h1', { className: 'lc-beitreten-titel', textContent: 'localchatA' }),
    element('label', { htmlFor: 'lc-name', className: 'lc-label', textContent: 'Name' }),
    element('div', { className: 'lc-beitreten-zeile' }, [feld, schalter]),
    hinweis,
    meldung,
  ]);

  let laeuft = false;
  async function beitreten() {
    if (laeuft) {
      return;
    }
    laeuft = true;
    schalter.disabled = true;
    formular.setAttribute('aria-busy', 'true');
    await aktionen.beitreten(feld.value);
  }

  hinweisZeigen(hinweis, hinweisZumZustand(zustand, feld.value));
  meldungZeigen(meldung, zustand, beitreten);
  feld.addEventListener('input', () => hinweisZeigen(hinweis, hinweisFuer(feld.value)));
  formular.addEventListener('submit', (ereignis) => {
    ereignis.preventDefault();
    beitreten();
  });

  wurzel.replaceChildren(element('main', { className: 'lc-beitreten' }, [formular]));
  feld.focus();
}

export function meldungFuer(zustand) {
  switch (zustand.art) {
    case 'vergeben':
      return `Der Name ${zustand.name} ist vergeben — ${zustand.belegtVon} ist seit ${uhrzeit(zustand.seit)} dabei. Wie wäre ${zustand.vorschlag}?`;
    case 'serverWeg':
      return '⚠ Server nicht erreichbar.';
    case 'fehler':
      return `⚠ ${zustand.fehler}`;
    default:
      return '';
  }
}

export function hinweisFuer(text) {
  const ergebnis = pruefen(text);
  if (text === '' || ergebnis.gueltig) {
    return '';
  }
  return mitVorschlag(ergebnis.hinweis, ergebnis.vorschlag);
}

function hinweisZumZustand(zustand, text) {
  if (zustand.art === 'ungueltig') {
    return mitVorschlag(zustand.fehler, zustand.vorschlag);
  }
  return hinweisFuer(text);
}

function mitVorschlag(text, vorschlag) {
  if (vorschlag === '') {
    return text;
  }
  return `${text} Vorschlag: ${vorschlag}`;
}

function feldinhalt(zustand) {
  if (zustand.art === 'vergeben') {
    return zustand.vorschlag;
  }
  return zustand.name;
}

function hinweisZeigen(hinweis, text) {
  hinweis.textContent = text;
  hinweis.hidden = text === '';
}

function meldungZeigen(meldung, zustand, erneutVersuchen) {
  const text = meldungFuer(zustand);
  meldung.replaceChildren(element('span', { textContent: text }));
  meldung.hidden = text === '';
  if (zustand.art === 'serverWeg') {
    const schalter = element('button', { type: 'button', className: 'lc-btn', textContent: 'Erneut versuchen' });
    schalter.addEventListener('click', erneutVersuchen);
    meldung.append(' ', schalter);
  }
}

function uhrzeit(zeitpunkt) {
  const zeit = new Date(zeitpunkt);
  const zweistellig = (zahl) => String(zahl).padStart(2, '0');
  return `${zweistellig(zeit.getHours())}:${zweistellig(zeit.getMinutes())}`;
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
