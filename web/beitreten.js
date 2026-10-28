// Beitreten-Bildschirm (Designsystem 4.1): Namensfeld mit Fokus, „Beitreten“, Hinweis beim Tippen,
// Zustände „Name vergeben“ und „ungültig“ (Vorschlag vorausgefüllt), „Server nicht erreichbar“ mit
// „Erneut versuchen“. Fehler in kd-danger mit ⚠ und Wort. Text vom Server nur über textContent, nie als HTML.
import { pruefen } from './namensregel.js';

const LEER = { art: 'leer' };
const OHNE_MELDUNG = { art: 'beitreten' };

export function zeichnen(wurzel, zustand, aktionen) {
  const feld = element('input', {
    id: 'lc-name', className: 'lc-feld', type: 'text', autocomplete: 'nickname', spellcheck: false,
    value: feldinhalt(zustand),
  });
  feld.setAttribute('autocapitalize', 'none');
  feld.setAttribute('aria-describedby', 'lc-name-zweck lc-name-hinweis lc-meldung');
  const schalter = element('button', { type: 'submit', className: 'lc-btn primary', textContent: 'Beitreten' });
  const hinweisRegel = element('span', { className: 'lc-hinweis-regel' });
  const hinweisVorschlag = element('span', { className: 'lc-hinweis-vorschlag' });
  const hinweis = element('p', { id: 'lc-name-hinweis', className: 'lc-hinweis' }, [hinweisRegel, hinweisVorschlag]);
  hinweis.setAttribute('aria-live', 'polite');
  const meldung = element('div', { id: 'lc-meldung', className: 'lc-meldung' });
  meldung.setAttribute('role', 'alert');
  const formular = element('form', { className: 'lc-beitreten-karte', noValidate: true }, [
    element('h1', { className: 'lc-beitreten-titel', textContent: 'localchatA' }),
    element('p', { id: 'lc-name-zweck', className: 'lc-zweck' }, [
      'Der Name steht mit der Herkunft als ', element('span', { className: 'lc-id', textContent: 'name@herkunft' }),
      ' an jeder Nachricht; erlaubt sind 1 bis 32 Zeichen aus a–z, 0–9 und -.',
    ]),
    element('label', { htmlFor: 'lc-name', className: 'lc-label', textContent: 'Name' }),
    element('div', { className: 'lc-beitreten-zeile' }, [feld, schalter]),
    hinweis,
    meldung,
  ]);

  let laeuft = false;
  let leerGemeldet = false;
  async function beitreten() {
    if (laeuft) {
      return;
    }
    if (feld.value.trim() === '') {
      meldungZeigen(meldung, LEER, beitreten);
      leerGemeldet = true;
      feld.setAttribute('aria-invalid', 'true');
      feld.focus();
      return;
    }
    laeuft = true;
    schalter.disabled = true;
    formular.setAttribute('aria-busy', 'true');
    await aktionen.beitreten(feld.value);
  }

  function beimTippen() {
    hinweisZeigen({ hinweis, hinweisRegel, hinweisVorschlag }, feld.value);
    gueltigkeitZeigen(feld);
    if (leerGemeldet && feld.value.trim() !== '') {
      meldungZeigen(meldung, OHNE_MELDUNG, beitreten);
      leerGemeldet = false;
    }
  }

  beimTippen();
  meldungZeigen(meldung, zustand, beitreten);
  feld.addEventListener('input', beimTippen);
  formular.addEventListener('submit', (ereignis) => {
    ereignis.preventDefault();
    beitreten();
  });

  wurzel.replaceChildren(element('main', { className: 'lc-beitreten' }, [formular]));
  feld.focus();
}

// Teile der Meldung: Text oder { kennung } für Namen, Kennungen und Vorschläge (Mono).
export function meldungsteile(zustand) {
  switch (zustand.art) {
    case 'vergeben':
      return [
        '⚠ Der Name ', { kennung: zustand.name }, ' ist vergeben — ', { kennung: zustand.belegtVon },
        ` ist seit ${uhrzeit(zustand.seit)} dabei.`, ...vorschlagsteile(zustand.vorschlag),
      ];
    case 'ungueltig':
      return [
        '⚠ Der Name ', { kennung: zustand.name }, ` ist nicht erlaubt — ${zustand.regel}.`,
        ...vorschlagsteile(zustand.vorschlag),
      ];
    case 'leer':
      return ['⚠ Bitte einen Namen eingeben.'];
    case 'serverWeg':
      return ['⚠ Server nicht erreichbar.'];
    case 'fehler':
      return [`⚠ ${zustand.fehler}`];
    default:
      return [];
  }
}

function istFehler(zustand) {
  return ['vergeben', 'ungueltig', 'leer', 'serverWeg', 'fehler'].includes(zustand.art);
}

export function hinweisFuer(text) {
  const ergebnis = pruefen(text);
  if (text.trim() === '' || ergebnis.gueltig) {
    return { regel: '', vorschlagsteile: [] };
  }
  return { regel: ergebnis.hinweis, vorschlagsteile: vorschlagsteile(ergebnis.vorschlag) };
}

function vorschlagsteile(vorschlag) {
  if (!vorschlag) {
    return [];
  }
  return [' Wie wäre ', { kennung: vorschlag }, '?'];
}

function feldinhalt(zustand) {
  if (zustand.art === 'vergeben') {
    return zustand.vorschlag;
  }
  if (zustand.art === 'ungueltig' && zustand.vorschlag) {
    return zustand.vorschlag;
  }
  return zustand.name;
}

function gueltigkeitZeigen(feld) {
  const text = feld.value;
  const ungueltig = text.trim() !== '' && !pruefen(text).gueltig;
  if (ungueltig) {
    feld.setAttribute('aria-invalid', 'true');
  } else {
    feld.removeAttribute('aria-invalid');
  }
}

// Die Regel steht in einer Live-Region: nur geänderter Text wird vorgelesen, darum bleibt ihr Knoten
// unberührt, solange sie gleich lautet — beim Tippen ändert sich nur der Vorschlag.
function hinweisZeigen({ hinweis, hinweisRegel, hinweisVorschlag }, text) {
  const { regel, vorschlagsteile: teile } = hinweisFuer(text);
  if (hinweisRegel.textContent !== regel) {
    hinweisRegel.textContent = regel;
  }
  hinweisVorschlag.replaceChildren(...teile.map(alsKnoten));
  hinweis.hidden = regel === '';
}

function meldungZeigen(meldung, zustand, erneutVersuchen) {
  const teile = meldungsteile(zustand);
  const text = element('span', {}, teile.map(alsKnoten));
  text.classList.toggle('lc-fehler', istFehler(zustand));
  meldung.replaceChildren(text);
  meldung.hidden = teile.length === 0;
  if (zustand.art === 'serverWeg') {
    const schalter = element('button', { type: 'button', className: 'lc-btn', textContent: 'Erneut versuchen' });
    schalter.addEventListener('click', erneutVersuchen);
    meldung.append(' ', schalter);
  }
}

function alsKnoten(teil) {
  if (typeof teil === 'string') {
    return teil;
  }
  return element('span', { className: 'lc-id', textContent: teil.kennung });
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
