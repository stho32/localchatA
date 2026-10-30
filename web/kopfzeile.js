// Eigene Kennung in der Kopfzeile (Designsystem: Bausteine Kopfzeile und Kennung): dunkle 40-px-Leiste,
// rechts Kürzel-Kachel und `name@herkunft`, Herkunft gedämpft, Farbe und Marke nach Art, danach die Schalter
// der Bedienung (z. B. „Teilnehmer“). Zone „Ort“: nach dem Instanznamen der gewählte Channel als Schalter
// `# name ▾` (öffnet die Channel-Einstellungen, ADR-0033), ab 720 px seine Frist und danach seine Beschreibung
// (ADR-0027 Regel 5); der Tab heißt dann „# name — localchatA“. Im Direktgespräch steht dort die volle Kennung des
// Gegenübers, und unter der Kopfzeile liegt der Hinweis als Streifen (R00050 F0178).
import { fristText } from './frist.js';

// Wortlaut ux-reviewer, PO 168 (V1): er verspricht nicht mehr, als gilt — nur in Oberfläche und API sind es die zwei.
const PRIVAT_HINWEIS = 'Direktnachricht — nur ihr beide im Chat; die Dateien liegen im Ordner der Instanz';

export function zeichnen(wurzel, teilnehmer, { kennungFokussieren = false, schalter = [], einstellungenOeffnen = () => {} } = {}) {
  const art = artDes(teilnehmer);
  const kachel = element('span', { className: `lc-tile ${art}`, textContent: kuerzel(teilnehmer.name) });
  kachel.setAttribute('aria-hidden', 'true');
  const kennung = element('span', { className: 'lc-me' }, [
    teilnehmer.name,
    element('i', { textContent: `@${teilnehmer.herkunft}` }),
  ]);
  kennung.tabIndex = -1;
  const bedienung = element('div', { className: 'lc-head-bed' }, [kachel, kennung]);
  if (art === 'agent') {
    bedienung.append(element('span', { className: 'lc-tag', textContent: 'Agent' }));
  }
  bedienung.append(...schalter);
  const ort = element('div', { className: 'lc-head-ort' }, [element('span', { className: 'lc-head-inst', textContent: 'localchatA' })]);
  const einstellungen = einstellungenSchalterAnlegen();
  einstellungen.addEventListener('click', () => einstellungenOeffnen(einstellungen));
  const kopf = element('header', { className: 'lc-head' }, [
    ort,
    element('div', { className: 'lc-head-nav' }),
    bedienung,
  ]);
  kopf.setAttribute('role', 'banner');
  const streifen = element('p', { className: 'lc-privat-hinweis', textContent: PRIVAT_HINWEIS, hidden: true });
  wurzel.replaceChildren(kopf, streifen);
  if (kennungFokussieren) {
    kennung.focus();
  }
  const frist = element('span', { className: 'lc-head-frist' });
  return {
    ortZeigen: (channel) => {
      streifen.hidden = true;
      ortZeigen({ ort, einstellungen, frist }, channel);
    },
    direktZeigen: (gegenueber) => direktZeigen({ ort, streifen }, gegenueber),
  };
}

// Der Schalter bleibt dasselbe Element, solange ein Channel gewählt ist — ein Wechsel aus dem Strom nimmt ihm den
// Fokus nicht, und der Dialog gibt ihn an ihn zurück.
function einstellungenSchalterAnlegen() {
  const pfeil = element('span', { className: 'lc-head-pfeil', textContent: '▾' });
  pfeil.setAttribute('aria-hidden', 'true');
  const schalter = element('button', { type: 'button', className: 'lc-head-schalter' }, [
    element('span', { className: 'lc-head-chan' }), ' ', pfeil,
  ]);
  schalter.setAttribute('aria-haspopup', 'dialog');
  return schalter;
}

// channel: { name, beschreibung, frist_tage } des gewählten bekannten Channels oder null.
function ortZeigen({ ort, einstellungen, frist }, channel) {
  const inst = ort.querySelector('.lc-head-inst');
  if (channel === null) {
    ort.replaceChildren(inst);
    document.title = 'localchatA';
    return;
  }
  einstellungen.querySelector('.lc-head-chan').textContent = `# ${channel.name}`;
  einstellungen.setAttribute('aria-label', `# ${channel.name} — Channel-Einstellungen`);
  einstellungen.dataset.channel = channel.name;
  frist.textContent = fristText(channel.frist_tage);
  if (einstellungen.parentNode !== ort) {
    ort.replaceChildren(inst, einstellungen, frist);
  }
  ort.querySelector('.lc-head-desc')?.remove();
  if (channel.beschreibung) {
    ort.append(beschreibungZeichnen(channel.beschreibung));
  }
  document.title = `# ${channel.name} — localchatA`;
}

// Im Direktgespräch (V1–V3): statt `# name ▾` die volle Kennung des Gegenübers, ungekürzt und ohne Schalter, ab 720 px
// die Frist — fest 7 Tage, wählbar erst mit F0179 (PO 166) —; der Hinweis steht bei jeder Breite unter der Kopfzeile.
function direktZeigen({ ort, streifen }, gegenueber) {
  const FRIST_TAGE = 7;
  const [name, herkunft] = gegenueber.split('@');
  const kennung = element('span', { className: 'lc-head-gegenueber' }, [
    name, element('wbr'), element('i', { textContent: `@${herkunft}` }),
  ]);
  const frist = element('span', { className: 'lc-head-frist', textContent: fristText(FRIST_TAGE) });
  ort.replaceChildren(ort.querySelector('.lc-head-inst'), kennung, frist);
  streifen.hidden = false;
  document.title = `Direktnachricht mit ${gegenueber} — localchatA`;
}

// Einzeilig mit Ellipse, ganz im title, die Schreibrichtung in <bdi> isoliert (ADR-0022 Regel 5); unter 720 px
// blendet das CSS sie aus.
function beschreibungZeichnen(beschreibung) {
  return element('span', { className: 'lc-head-desc', title: beschreibung }, [element('bdi', { textContent: beschreibung })]);
}

export function kuerzel(name) {
  const teile = name.split('-');
  if (teile.length > 1) {
    return teile.slice(0, 2).map((teil) => teil.charAt(0)).join('');
  }
  return name.slice(0, 2);
}

function artDes(teilnehmer) {
  if (teilnehmer.art === 'agent') {
    return 'agent';
  }
  return 'mensch';
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
