// Der Verlauf eines Channels in der Mitte (ADR-0026): Öffnen holt die neuesten 100 mit einem Abruf, zeichnet sie
// älteste oben und steht am Ende; solange der Blick dort steht, hält ein ResizeObserver ihn dort, auch wenn Höhen
// sich nachträglich ändern (Schrift, Fenster). Beim Scrollen lädt das Fenster an beiden Enden je 100 nach und hält
// fünf Blöcke, bis zu acht, solange das andere Ende nah am Blick ist (Abwurfschutz, fenster.js); die oberste
// sichtbare Nachricht bleibt dabei an ihrer Stelle (Anker von Hand, in einem Zug). Ein Wechsel bricht laufende
// Abrufe ab, eine Antwort für einen nicht mehr gewählten Channel wird verworfen, Daten des vorigen Channels fallen
// weg. Der Verlauf ist die einzige scrollende Fläche der Mitte: role="log", aria-live="off", mit der Tastatur
// fokussierbar; Nachladen wird nie angesagt.
import { nachrichtZeichnen, kopfSetzen, trennerZeichnen, langeEinklappen, langeUmschalten, markeUmschalten, kopierRueckmeldungZeigen } from './nachricht.js';
import { gliederung, folgeGliedern, datumBenennen } from './gliederung.js';
import { nachladen, einfuegen } from './fenster.js';
import { pfeilRichtung, pfeilZiel, tabStopp, eigeneLetzte, nachrichtTaste, kopierTaste, antwortTaste, tasteGehoertDemElement } from './fokus.js';
import { bearbeitenAnlegen } from './bearbeiten.js';
import { rueckfrageOeffnen, loeschenAuswerten } from './loeschen.js';
import { kopieren, rueckmeldungFuer, RUECKMELDUNG_LEER, MENUE_OFFEN_MS } from './kopieren.js';
import { menueLage, schalterGanzSichtbar } from './menuelage.js';
import { zitatNachfuehren } from './zitat.js';
import { gegenueberIn, apiPfad, ortText } from './gespraech.js';
import { zeigerart } from './eingabe.js';
import { mediumSpielt, medienUebernehmen, medienAblageAnlegen, blockSpielt, endeVorWiedergabe } from './player.js';
import {
  einordnen, zusammenfuehren, fassungUebernehmen, buendlerAnlegen, folgenOderZaehlen, pillenText, auslaufNach, nichtGeladenText, hinweisTextNachziehen,
  sprungZiel as sprungZielBerechnen, scrollTopNachHoehe, fensterNeuLesen, zeitDanach, zeitDavor, anhaengeAlsGeloescht, geloeschteAnhaenge,
} from './live-verlauf.js';

// abrufen: async (name, cursor, signal) → { art: 'block' | 'fehler' | 'abgebrochen', … } (api.js verlaufAbrufen);
// das Ergebnis von `oeffnen` sagt, ob der Verlauf steht.
// Live (ADR-0037 Regeln 1, 6): nachrichtAnnehmen(nachricht, eigen) ordnet über die Zeit ein und fügt je Frame ein —
// am Ende folgt der Blick, sonst zählt die Pille „n neue Nachrichten“; fassungAnnehmen(channel, nachricht) ersetzt eine
// Nachricht im Fenster durch ihre geänderte Fassung (ADR-0048 Regel 11); zitatAnnehmen(channel, zitat) führt die Zitate
// der Antworten auf diese Nachricht nach (R00047 B0721, ADR-0053 Regel 5); vorlaeufigeZeigen(liste) zeichnet eigene
// Nachrichten unterwegs oder fehlgeschlagen unter allen bestätigten; an fehlgeschlagenen rufen „Erneut senden“ und
// „Verwerfen“ schalter('erneut' | 'verwerfen', schluessel) (ADR-0038 Regel 5).
// bearbeitenSpeichern: async (channel, zeit, text) → Antwort (api.js nachrichtBearbeiten); loeschen: async (channel, zeit) →
// Antwort (api.js nachrichtLoeschen); hinweisen(text): die Hinweiszeile der Seite; ansagen(text): eine Ansage über die
// Region der Seite (R00041 F0135, F0139). antworten(nachricht): die Aktion „Antworten“ an einer Nachricht des Fensters
// (R00047 B0713), synchron in der Geste. signal: Abbau der Ansicht — Listener an document und window enden mit ihr.
export function verlaufAnlegen({
  abrufen: abrufenRoh, schalter = () => {}, fokusRueckgaengig = () => {}, fokusVerloren = () => {}, bearbeitenSpeichern = async () => ({ art: 'netzfehler' }),
  loeschen = async () => ({ art: 'netzfehler' }), hinweisen = () => {}, ansagen = () => {}, antworten = () => {}, signal = new AbortController().signal,
  hochladungen = element('div'),
}) {
  const inhalt = element('div', { className: 'lc-log-inhalt' });
  // Unter den vorläufigen die Einträge „wird hochgeladen …“ (R00030 B0450): sie gehören anhaenge.js und werden beim
  // Neuzeichnen der vorläufigen nicht angefasst — ein Fokus auf ihren Schaltern bleibt.
  const vorlaeufigeKnoten = element('div', { className: 'lc-vorlaeufige' });
  const ausstehend = element('div', { className: 'lc-ausstehend' }, [vorlaeufigeKnoten, hochladungen]);
  const auslauf = element('div', { className: 'lc-auslauf' });
  const log = element('div', { className: 'lc-log', tabIndex: 0 }, [inhalt, ausstehend, auslauf]);
  log.setAttribute('role', 'log');
  log.setAttribute('aria-live', 'off');
  const ladezeile = element('div', { className: 'lc-loadmore', textContent: 'Ältere Nachrichten werden geladen …', hidden: true });
  // Pille (R00020 F0068): sichtbar „↓ Zum Neuesten“ bzw. „↓ n neue Nachrichten“, mit Leerraum vor der Taste Ende;
  // Pfeil und Taste sind für Bildschirmleser verborgen, die Taste nennt aria-keyshortcuts.
  const pillenBeschriftung = element('span', { textContent: 'Zum Neuesten' });
  const pille = element('button', { type: 'button', className: 'lc-pill', hidden: true }, [
    element('span', {}, [element('span', { textContent: '↓', ariaHidden: 'true' }), ' ', pillenBeschriftung]),
    element('kbd', { textContent: 'Ende', ariaHidden: 'true' }),
  ]);
  pille.setAttribute('aria-keyshortcuts', 'End');
  const rahmen = element('div', { className: 'lc-verlauf' }, [ladezeile, log, element('div', { className: 'lc-pillen' }, [pille])]);
  let laufend = new AbortController();
  let offenerName = null;
  let amEnde = true;
  let heute = null;
  let fenster = null;
  let bloeckeKnoten = [];
  let gescheitert = false;
  let geladen = false;
  let puffer = [];
  let juengste = null;
  let neueGezaehlt = 0;
  let ersteNeue = null;
  let nurGezaehlte = [];
  // Zeitpunkte aller Nachrichten, die der Tab im offenen Channel angenommen hat — geladen oder live, bis zum
  // Neu-Öffnen (R00025 F0084: „n nachgeladen“ zählt nur, was erst das Nachholen brachte).
  let angenommen = new Set();
  // Zeitpunkte aller Nachrichten, die im Fenster stehen, aus ihm gefallen oder nur gezählt sind — der Abgleich für
  // Neues geht gegen sie alle, nicht nur gegen den jüngsten Block (ADR-0037 Ergänzung „Karte #36 Wortlaut“ Punkt 3).
  let eingeordnet = new Set();
  let vorlaeufige = [];
  // Lädt der Tab von selbst (Strom getrennt oder Laden-Lauf)? Die Hauptansicht setzt die Auskunft (R00037 Regel 1).
  let selbstversuchend = () => false;
  const buendler = buendlerAnlegen({ einfuegen: jeFrameEinfuegen });
  // IDs aller Anhänge, deren Löschen der Tab erfahren hat (R00030 B0518): eine Antwort, die vor dem Löschen begann und
  // erst danach ankommt, zeigt sie sonst als vorhanden.
  const geloeschteIds = new Set();

  async function abrufen(name, cursor, abbruch) {
    const ergebnis = await abrufenRoh(name, cursor, abbruch);
    if (ergebnis.art !== 'block' || geloeschteIds.size === 0) {
      return ergebnis;
    }
    const nachrichten = ergebnis.block.nachrichten.map((nachricht) => anhaengeAlsGeloescht(nachricht, geloeschteIds));
    return { ...ergebnis, block: { ...ergebnis.block, nachrichten } };
  }

  function abstandUnten() {
    return log.scrollHeight - log.scrollTop - log.clientHeight;
  }

  function ansEnde() {
    log.scrollTop = log.scrollHeight;
    amEnde = true;
  }

  // Sichthöhe und Stand von scrollTop, wie sie bei ruhiger Größe waren (R00048 F0164): ändert sich die Sichthöhe,
  // klemmt der Browser scrollTop noch vor dem Beobachter — die Nachführung rechnet darum vom gemerkten Stand aus.
  let sichtVorher = log.clientHeight;
  let scrollTopGemerkt = log.scrollTop;

  // Die Sichthöhe, an der die Pille „weit oben“ misst: sie ändert sich nur durch Scrollen, Öffnen und das Ende — wächst
  // allein die Eingabe, taucht eine versteckte Pille nicht auf (Karte #82, Code-Review #80).
  let schwelleSicht = log.clientHeight;
  let nachgefuehrtAuf = null; // scrollTop, auf den die Nachführung nach einer Höhenänderung gesetzt hat: kein Lesen

  function sichtGeaendert() {
    return log.clientHeight !== sichtVorher;
  }

  function lageMerken() {
    if (!sichtGeaendert()) {
      scrollTopGemerkt = log.scrollTop;
    }
    auslaufAbbauen(auslaufSichtbar(), 0);
    amEnde = abstandUnten() <= 1 && fenster !== null && !fenster.neuer;
    if (amEnde) {
      neueGezaehlt = 0;
      ersteNeue = null;
      schwelleSicht = log.clientHeight;
    }
    const weitOben = abstandUnten() > schwelleSicht;
    pille.hidden = fenster === null || !(weitOben || fenster.neuer || neueGezaehlt > 0);
    pillenBeschriftung.textContent = 'Zum Neuesten';
    if (neueGezaehlt > 0) {
      pillenBeschriftung.textContent = pillenText(neueGezaehlt);
    }
  }

  // Ein Scroll, den nur die geänderte Sichthöhe auslöste (Klemmen), ist keine Bewegung der Lesenden: der Beobachter
  // unten führt nach und merkt dann die Lage.
  log.addEventListener('scroll', () => {
    if (sichtGeaendert()) {
      return;
    }
    if (log.scrollTop !== nachgefuehrtAuf) {
      schwelleSicht = log.clientHeight;
    }
    lageMerken();
    pruefen();
  }, { passive: true });
  // Ändert sich die Sichthöhe (Feld wächst oder schrumpft, Tippanzeige, Antwortzeile, Tastatur), bleibt die Lesestelle
  // im Verlauf: am Ende folgt der Blick dem Ende, sonst bleibt der Abstand zur Unterkante gleich.
  const hoehen = new ResizeObserver(hoehenNachfuehren);
  hoehen.observe(inhalt);
  hoehen.observe(ausstehend);
  hoehen.observe(log);

  function hoehenNachfuehren() {
    if (amEnde) {
      ansEnde();
    } else if (sichtGeaendert() && sichtVorher > 0) {
      log.scrollTop = scrollTopNachHoehe({
        scrollTop: scrollTopGemerkt,
        hoeheVorher: sichtVorher,
        hoeheNachher: log.clientHeight,
        inhaltsHoehe: log.scrollHeight,
      });
      nachgefuehrtAuf = log.scrollTop;
    }
    sichtVorher = log.clientHeight;
    lageMerken();
  }

  // Erscheint die Antwortzeile über dem Feld (R00047 B0714, Z2), führt der Verlauf die geänderte Sichthöhe sofort nach
  // (R00048) und bringt das beantwortete Original danach ganz ins Bild — liegt es schon dort, bewegt sich nichts weiter.
  function originalSichtbarHalten(zeit) {
    hoehenNachfuehren();
    const knoten = nachrichtKnoten(zeit);
    if (knoten !== null) {
      inSichtBringen(knoten);
    }
  }

  // Aufklappen erweitert an Ort und Stelle; der Kopf der Nachricht bleibt an seiner Bildschirmstelle. Die Marke
  // „bearbeitet“ blendet die Zeit der letzten Änderung ein und aus.
  inhalt.addEventListener('click', (ereignis) => {
    const zitat = ereignis.target.closest('.lc-zitat');
    if (zitat !== null) {
      zitatBedienen(zitat);
      return;
    }
    const marke = ereignis.target.closest('.lc-marke');
    if (marke !== null) {
      markeUmschalten(marke);
      return;
    }
    const menueSchalter = ereignis.target.closest('.lc-aktionen-schalter');
    if (menueSchalter !== null) {
      menueUmschalten(menueSchalter);
      return;
    }
    const aktion = ereignis.target.closest('.lc-aktion');
    if (aktion !== null && aktion.dataset.aktion === 'kopieren') {
      aktionAusfuehren('kopieren', aktion.closest('.lc-msg'));
      return;
    }
    if (aktion !== null) {
      menueSchliessen();
      aktionAusfuehren(aktion.dataset.aktion, aktion.closest('.lc-msg'));
      return;
    }
    const schalter = ereignis.target.closest('.lc-foldbar button');
    if (schalter === null) {
      return;
    }
    const anker = ankerVon(schalter.closest('.lc-msg'));
    langeUmschalten(schalter);
    ankerHalten(log, anker);
    if (ereignis.detail === 0) {
      inSichtBringen(schalter);
    }
    lageMerken();
  });

  // „Zum Neuesten“: Klick oder Ende im Verlauf öffnet den Channel neu wie beim Wählen; der Fokus geht in den Verlauf.
  // Mit gezählten neuen Nachrichten: steht die erste im Fenster, kommt sie unter den klebenden Tagestrenner, eine Zeile
  // Kontext darüber (ADR-0037 Regel 6, R00020 F0067); steht sie schon dort (± 1 px), geht es ans Ende des Inhalts;
  // sonst neu öffnen.
  function zumNeuesten() {
    log.focus();
    const erste = ersteNeueImFenster();
    if (erste === null) {
      oeffnen(offenerName);
      return;
    }
    const ziel = sprungZiel(erste);
    const stehtSchonDort = Math.abs(ziel - log.scrollTop) <= 1;
    if (stehtSchonDort) {
      zumEndeDesInhalts();
      return;
    }
    auslaufFuer(ziel);
    log.scrollTop = ziel;
    lageMerken();
  }

  // Ans Ende des Inhalts, über dem Auslauf, der dabei unter das Fenster gerät und wegfällt (ADR-0037 Ergänzung
  // „Auslauf“ Punkt 4); steht der neueste Block nicht im Fenster, öffnet der Tab neu.
  function zumEndeDesInhalts() {
    if (fenster.neuer) {
      oeffnen(offenerName);
      return;
    }
    log.scrollTop = log.scrollHeight - log.clientHeight - auslauf.getBoundingClientRect().height;
    lageMerken();
  }

  // Der Kontext ist die letzte Zeile der Nachricht vor der ersten neuen, auch über den Tagestrenner hinweg, wenn die
  // erste neue einen Tag beginnt: ihre Unterkante kommt eine Zeile unter den Trenner, der über ihr klebt. Ohne
  // Nachricht davor kommt die Oberkante des Platzes der ersten neuen dorthin.
  function sprungZiel(erste) {
    const kontext = nachrichtDavor(erste);
    const bezug = kontext === null ? erste : kontext;
    return sprungZielBerechnen({
      scrollTop: log.scrollTop,
      oberkanteErste: oberkanteImVerlauf(erste),
      unterkanteKontext: kontext === null ? null : kontext.getBoundingClientRect().bottom - log.getBoundingClientRect().top,
      trennerUnterkante: klebenderTrennerUnterkante(bezug),
      zeilenhoehe: zeilenhoehe(),
    });
  }

  // Die obere Kante des Platzes einer Nachricht, mit ihrem Abstand nach oben, gemessen vom oberen Rand des Verlaufs.
  function oberkanteImVerlauf(nachricht) {
    const abstandNachOben = parseFloat(getComputedStyle(nachricht).marginTop);
    return nachricht.getBoundingClientRect().top - abstandNachOben - log.getBoundingClientRect().top;
  }

  // Reicht der Inhalt unter der ersten neuen nicht für eine Sichthöhe, entsteht darunter so viel Raum, dass sie oben
  // stehen kann — mehr als eine Zeile mehr, damit der Blick dabei nicht am Ende gilt und weitere Nachrichten gezählt
  // statt verfolgt werden; der Raum baut sich ab, wie neuer Inhalt ihn einnimmt oder er aus dem Fenster gerät
  // (auslaufNach).
  function auslaufFuer(ziel) {
    const UEBER_DIE_ZEILE_PX = 2;
    const fehlt = ziel - (log.scrollHeight - log.clientHeight);
    if (fehlt > 0) {
      auslauf.style.height = `${fehlt + zeilenhoehe() + UEBER_DIE_ZEILE_PX}px`;
    }
  }

  function auslaufSichtbar() {
    return log.getBoundingClientRect().bottom - auslauf.getBoundingClientRect().top;
  }

  function auslaufAbbauen(sichtbar, angefuegt) {
    const hoehe = auslauf.getBoundingClientRect().height;
    if (hoehe === 0) {
      return;
    }
    const danach = auslaufNach({ hoehe, sichtbar, angefuegt });
    auslauf.style.height = `${danach}px`;
    if (danach === 0) {
      auslauf.style.height = '';
    }
  }

  function ersteNeueImFenster() {
    if (neueGezaehlt === 0 || ersteNeue === null) {
      return null;
    }
    return inhalt.querySelector(`.lc-msg[data-zeit="${ersteNeue}"]`);
  }

  // Die Zeilenhöhe des Verlaufs steht im CSS (J06).
  function zeilenhoehe() {
    return parseFloat(getComputedStyle(log).lineHeight);
  }
  pille.addEventListener('click', zumNeuesten);
  ausstehend.addEventListener('click', (ereignis) => {
    const zitat = ereignis.target.closest('.lc-zitat');
    if (zitat !== null) {
      zitatBedienen(zitat);
      return;
    }
    const geklickt = ereignis.target.closest('button[data-aktion]');
    if (geklickt !== null) {
      schalter(geklickt.dataset.aktion, geklickt.closest('[data-schluessel]').dataset.schluessel);
    }
  });
  // Tasten in einem Textfeld des Verlaufs (dem Bearbeiten-Feld) gehören dem Text, nie dem Verlauf (R00041 UX M2); Ende,
  // Pos1, Leertaste und Pfeile im Fokus eines Players, Links oder Knopfs gehören dem Element (R00030 B0752).
  log.addEventListener('keydown', (ereignis) => {
    if (ereignis.target.closest('textarea, input') !== null) {
      return;
    }
    if (tasteGehoertDemElement(ereignis, tagsBisZumVerlauf(ereignis.target))) {
      // `Ende` im Player wirkt erst nach begonnener Wiedergabe; vorher bleibt der Verlauf stehen und nichts lädt (PO 178).
      if (endeVorWiedergabe(ereignis)) {
        ereignis.preventDefault();
      }
      return;
    }
    const nurEnde = ereignis.key === 'End' && !ereignis.altKey && !ereignis.ctrlKey && !ereignis.metaKey && !ereignis.shiftKey;
    if (nurEnde && offenerName !== null) {
      ereignis.preventDefault();
      zumNeuesten();
    }
    if (ereignis.target.classList.contains('lc-msg')) {
      pfeilBedienen(ereignis);
      tasteAufNachricht(ereignis);
    }
  });

  function tagsBisZumVerlauf(ziel) {
    const tags = [];
    for (let knoten = ziel; knoten !== null && knoten !== log; knoten = knoten.parentElement) {
      tags.push(knoten.localName);
    }
    return tags;
  }

  // Aktionen an der Nachricht (R00041 F0135, F0139; R00046; R00047): aus der Leiste, dem Menü „⋯“ oder per Taste (R, C, E,
  // Entf) auf der fokussierten Nachricht — nur, wenn die Nachricht die Aktion trägt.
  const bearbeiten = bearbeitenAnlegen({
    zeiger: zeigerart,
    fokusAufNachricht: (zeit) => {
      const knoten = nachrichtKnoten(zeit);
      if (knoten !== null) {
        nachrichtFokussieren(knoten);
      }
    },
    inSicht: inSichtBringen,
    feldHolen: zumFeldOeffnen,
    speichern: bearbeitenSpeichern,
    uebernehmen: (nachricht) => fassungenErsetzen([nachricht]),
    alsGeloeschtZeigen: geloeschtSetzen,
    geschlossen: nachFeldNeuZeichnen,
    hinweisen,
    ansagen,
    signal,
  });

  // Menü „⋯“ auf Touch-Geräten (UX M6, PO 125): höchstens eines offen; ein zweites Tippen, ein Tippen daneben und Esc
  // schließen es — nach Esc steht der Fokus wieder auf „Aktionen“. Reicht der Platz unter der Nachricht nicht, öffnet
  // es nach oben.
  function menueUmschalten(menueSchalter) {
    const warOffen = menueSchalter.getAttribute('aria-expanded') === 'true';
    menueSchliessen();
    if (warOffen) {
      return;
    }
    menueSchalter.setAttribute('aria-expanded', 'true');
    menueEinpassen(menueSchalter, document.getElementById(menueSchalter.getAttribute('aria-controls')));
  }

  // Das offene Menü steht fest im Fenster (position: fixed, der Verlauf schneidet es nicht ab) und liegt ganz im sichtbaren
  // Teil — des Verlaufs, wenn es dort hineinpasst, sonst des Fensters (UX K1).
  function menueEinpassen(menueSchalter, menue) {
    const menueRand = menue.getBoundingClientRect();
    const { fenster, verlaufSichtbar } = sichtbareBereiche();
    const lage = menueLage({
      schalter: schalterRand(menueSchalter),
      menue: { breite: menueRand.width, hoehe: menueRand.height },
      bereiche: [verlaufSichtbar, fenster],
    });
    menue.style.top = `${lage.oben}px`;
    menue.style.left = `${lage.links}px`;
  }

  function sichtbareBereiche() {
    const sicht = window.visualViewport;
    const fenster = { oben: sicht.offsetTop, unten: sicht.offsetTop + sicht.height, links: sicht.offsetLeft, rechts: sicht.offsetLeft + sicht.width };
    const verlaufRand = log.getBoundingClientRect();
    const verlaufSichtbar = {
      oben: Math.max(verlaufRand.top, fenster.oben), unten: Math.min(verlaufRand.bottom, fenster.unten), links: fenster.links, rechts: fenster.rechts,
    };
    return { fenster, verlaufSichtbar };
  }

  // Scrollt der Verlauf von selbst (neue Nachricht am Ende, Nachladen, Lesestelle halten), wandert das Menü mit seinem
  // Schalter und schließt erst, wenn der Schalter den sichtbaren Teil des Verlaufs verlässt (UX S1). Scrollt der Nutzer,
  // schließt es: Rad hier, Wischen und Ziehen am Balken über pointerdown.
  function menueNachfuehren() {
    const offenerSchalter = inhalt.querySelector('.lc-aktionen-schalter[aria-expanded="true"]');
    if (offenerSchalter === null) {
      return;
    }
    if (!schalterGanzSichtbar(schalterRand(offenerSchalter), sichtbareBereiche().verlaufSichtbar)) {
      menueSchliessen();
      return;
    }
    menueEinpassen(offenerSchalter, document.getElementById(offenerSchalter.getAttribute('aria-controls')));
  }

  function schalterRand(menueSchalter) {
    const rand = menueSchalter.getBoundingClientRect();
    return { oben: rand.top, unten: rand.bottom, links: rand.left, rechts: rand.right };
  }

  // Nach dem Kopieren aus dem Menü: es schließt, der Fokus geht auf „Aktionen“ zurück statt mit dem Eintrag zu verschwinden.
  function menueNachKopierenSchliessen() {
    const offenerSchalter = inhalt.querySelector('.lc-aktionen-schalter[aria-expanded="true"]');
    menueSchliessen();
    if (offenerSchalter !== null) {
      offenerSchalter.focus({ preventScroll: true });
    }
  }

  function menueSchliessen() {
    for (const offenerSchalter of inhalt.querySelectorAll('.lc-aktionen-schalter[aria-expanded="true"]')) {
      offenerSchalter.setAttribute('aria-expanded', 'false');
    }
  }
  document.addEventListener('pointerdown', (ereignis) => {
    if (ereignis.target.closest('.lc-aktionen-schalter, .lc-aktionen') === null) {
      menueSchliessen();
    }
  }, { signal });
  log.addEventListener('scroll', menueNachfuehren, { passive: true, signal });
  log.addEventListener('wheel', menueSchliessen, { passive: true, signal });
  log.addEventListener('keydown', (ereignis) => {
    const offenerSchalter = inhalt.querySelector('.lc-aktionen-schalter[aria-expanded="true"]');
    if (ereignis.key === 'Escape' && offenerSchalter !== null) {
      ereignis.preventDefault();
      menueSchliessen();
      offenerSchalter.focus();
    }
  });

  // Das offene Feld gehört dem Tab (F0142): steht seine Nachricht wieder im Verlauf (neu gezeichnet, Rückkehr in den
  // Channel), hängt es sich ein — den Fokus bekommt es bei der Rückkehr und wenn er mit dem alten Knoten verloren ging.
  function feldEinsetzen(mitFokus) {
    const zeit = bearbeiten.offeneZeitIn(offenerName);
    if (zeit === null || bearbeiten.eingesetzt()) {
      return;
    }
    const knoten = nachrichtKnoten(zeit);
    if (knoten === null || knoten.dataset.zustand === 'geloescht') {
      return;
    }
    const fokusVerloren = document.activeElement === null || document.activeElement === document.body;
    bearbeiten.einsetzen(knoten, mitFokus || fokusVerloren);
  }

  // Nach dem Öffnen des Channels: steht die Nachricht des Felds im Fenster, ist es wieder offen mit Fokus; ist sie
  // gelöscht oder fehlt sie im Zeitraum des Fensters, schließt es mit Hinweis (PO 124). Liegt sie vor dem Fenster, holt
  // die Rückkehr in den Channel das Fenster bis zu ihr (F0142); sonst wartet das Feld, bis sie wieder geladen ist.
  function feldNachOeffnen(rueckkehr) {
    const zeit = bearbeiten.offeneZeitIn(offenerName);
    if (zeit === null) {
      return;
    }
    const bekannt = bekannteNachricht(zeit);
    if (bekannt !== null && bekannt.nachricht.zustand !== 'geloescht') {
      feldEinsetzen(true);
      return;
    }
    const vorDemFenster = fenster !== null && Date.parse(zeit) < Date.parse(fenster.bloecke[0][0].zeit) && fenster.aelter;
    if (!vorDemFenster) {
      bearbeiten.weilWegSchliessen();
      return;
    }
    if (rueckkehr) {
      zumFeldOeffnen();
    }
  }

  // Öffnet den Channel mit dem Block, der mit der Nachricht des Felds endet; das Nachladen holt das Neuere dazu.
  function zumFeldOeffnen() {
    oeffnen(offenerName, undefined, { vor: zeitDanach(bearbeiten.offeneZeitIn(offenerName)) });
  }

  function blockMitFeld(block) {
    const zeit = bearbeiten.offeneZeitIn(offenerName);
    return zeit !== null && block !== undefined && block.some((nachricht) => Date.parse(nachricht.zeit) === Date.parse(zeit));
  }

  function tasteAufNachricht(ereignis) {
    const lage = {
      zielIstNachricht: ereignis.target.matches('.lc-msg'),
      markierungLeer: document.getSelection().isCollapsed,
      dialogOffen: document.querySelector('dialog[open]') !== null,
    };
    const buchstabe = buchstabenAktion(ereignis, lage);
    if (buchstabe !== null && ereignis.target.querySelector(`.lc-aktion[data-aktion="${buchstabe}"]`) !== null) {
      ereignis.preventDefault();
      aktionAusfuehren(buchstabe, ereignis.target);
      return;
    }
    const aktion = nachrichtTaste(ereignis);
    if (aktion === null || ereignis.target.querySelector(`.lc-aktion[data-aktion="${aktion}"]`) === null) {
      return;
    }
    ereignis.preventDefault();
    aktionAusfuehren(aktion, ereignis.target);
  }

  function buchstabenAktion(ereignis, lage) {
    if (antwortTaste(ereignis, lage)) {
      return 'antworten';
    }
    if (kopierTaste(ereignis, lage)) {
      return 'kopieren';
    }
    return null;
  }

  function aktionAusfuehren(aktion, artikel) {
    const bekannt = bekannteNachricht(artikel.dataset.zeit);
    if (bekannt === null) {
      return;
    }
    if (aktion === 'antworten') {
      antworten(bekannt.nachricht);
    }
    if (aktion === 'kopieren') {
      nachrichtKopieren(artikel, bekannt.nachricht.text);
    }
    if (aktion === 'bearbeiten') {
      menueSchliessen();
      bearbeiten.beginnen(offenerName, artikel, bekannt.nachricht);
    }
    if (aktion === 'loeschen') {
      loeschenFragen(bekannt.nachricht);
    }
  }

  // Der Text kommt aus dem Fenster, nie aus dem DOM (ADR-0052 Regel 1); der Aufruf bleibt in der Geste. Das versteckte
  // Feld nimmt für den Aufruf den Fokus und die Leiste (nur bei Hover oder Fokus sichtbar) verlöre ihn mit: sie bleibt
  // stehen, bis der Fokus auf ihren Knopf zurückgelegt ist. Wirft das Kopieren, gilt es als nicht möglich.
  function nachrichtKopieren(artikel, text) {
    artikel.classList.add('lc-kopiert-laeuft');
    const ausgang = kopieren(text, document);
    artikel.classList.remove('lc-kopiert-laeuft');
    ausgang.catch(() => 'nicht_moeglich').then((ergebnis) => kopierenMelden(artikel, ergebnis));
  }

  // Rückmeldung nach dem Kopieren: am Knopf, in der Ansage-Region; ein offenes Menü auf Touch bleibt kurz stehen, damit
  // man die Rückmeldung im Eintrag sieht, und schließt dann (UX K2).
  let kopierZustand = RUECKMELDUNG_LEER;
  function kopierenMelden(artikel, ergebnis) {
    const rueckmeldung = rueckmeldungFuer(kopierZustand, ergebnis, artikel.dataset.zeit, Date.now());
    kopierZustand = rueckmeldung.zustand;
    kopierRueckmeldungZeigen(artikel, rueckmeldung.anzeige, rueckmeldung.dauerMs);
    if (rueckmeldung.ansage !== null) {
      ansagen(rueckmeldung.ansage);
    }
    const menueSchalter = artikel.querySelector('.lc-aktionen-schalter[aria-expanded="true"]');
    if (menueSchalter !== null) {
      // Die Rückmeldung kann den Eintrag verbreitern (Fehler): das Menü bleibt links neben dem Schalter.
      menueEinpassen(menueSchalter, document.getElementById(menueSchalter.getAttribute('aria-controls')));
      setTimeout(() => {
        if (menueSchalter.getAttribute('aria-expanded') === 'true') {
          menueNachKopierenSchliessen();
        }
      }, MENUE_OFFEN_MS);
    }
  }

  // Löschen nach genau einer Rückfrage (F0139), höchstens ein Aufruf zugleich: gelöscht → die Fassung der Antwort, Fokus
  // auf der Nachricht, Ansage „Nachricht gelöscht.“; abgewiesen → Hinweiszeile, die Nachricht bleibt und lässt sich
  // erneut löschen. Ein Wechsel des Channels verwirft die Antwort nicht — die Fassung trifft nur den offenen. Solange es
  // läuft, kennt der Verlauf { channel, zeit } — ein offenes Feld an dieser Nachricht schließt dann ohne „inzwischen“.
  let loeschenLaeuftFuer = null;
  function loeschenFragen(nachricht) {
    const channel = offenerName;
    const zurueckZurNachricht = () => {
      const knoten = nachrichtKnoten(nachricht.zeit);
      if (knoten !== null) {
        nachrichtFokussieren(knoten);
      }
    };
    const bestaetigt = async () => {
      zurueckZurNachricht();
      if (loeschenLaeuftFuer !== null) {
        return;
      }
      loeschenLaeuftFuer = { channel, zeit: nachricht.zeit };
      const ergebnis = loeschenAuswerten(await loeschen(channel, nachricht.zeit));
      if (ergebnis.art === 'abgewiesen') {
        loeschenLaeuftFuer = null;
        hinweisen(ergebnis.text);
        return;
      }
      if (channel === offenerName) {
        fassungenErsetzen([ergebnis.nachricht]);
        zurueckZurNachricht();
      }
      loeschenLaeuftFuer = null;
      ansagen('Nachricht gelöscht.');
    };
    rueckfrageOeffnen(rahmen, nachricht, { bestaetigt, abgebrochen: zurueckZurNachricht });
  }

  function nachrichtKnoten(zeit) {
    return inhalt.querySelector(`:scope > .lc-msg[data-zeit="${CSS.escape(zeit)}"]`);
  }

  // Wo eine Nachricht im Fenster steht: { blockStelle, stelle, nachricht } oder null.
  function bekannteNachricht(zeit) {
    if (fenster === null) {
      return null;
    }
    const zeitpunkt = Date.parse(zeit);
    for (let blockStelle = 0; blockStelle < fenster.bloecke.length; blockStelle += 1) {
      const stelle = fenster.bloecke[blockStelle].findIndex((bekannt) => Date.parse(bekannt.zeit) === zeitpunkt);
      if (stelle >= 0) {
        return { blockStelle, stelle, nachricht: fenster.bloecke[blockStelle][stelle] };
      }
    }
    return null;
  }

  // Rollfokus (R00041 F0141): genau eine Nachricht des Verlaufs ist Tab-Stopp (tabindex 0, alle anderen -1) — die
  // zuletzt fokussierte, sonst die neueste; solange der Verlauf Nachrichten hat, ist er selbst keiner (leer bleibt er
  // der Stopp, ADR-0026). ↑/↓ auf einer Nachricht wechseln zur vorigen/nächsten, auch über Blockgrenzen: der Blick
  // folgt, und das Nachladen läuft wie beim Scrollen. Tasten in Knöpfen und Feldern einer Nachricht bleiben dort.
  let rollZeit = null;
  function nachrichtenImVerlauf() {
    return Array.from(inhalt.querySelectorAll(':scope > .lc-msg'));
  }

  function rollfokusPflegen() {
    const liste = nachrichtenImVerlauf();
    const stelle = tabStopp(liste.map((einzeln) => einzeln.dataset.zeit), rollZeit);
    liste.forEach((einzeln, nummer) => {
      let soll = '-1';
      if (nummer === stelle) {
        soll = '0';
      }
      if (einzeln.getAttribute('tabindex') !== soll) {
        einzeln.setAttribute('tabindex', soll);
      }
    });
    log.tabIndex = -1;
    if (stelle === null) {
      log.tabIndex = 0;
    }
    schalterRollen(liste[stelle]);
  }

  // Auch die Knöpfe einer Nachricht („Ganze Nachricht anzeigen“, die Marke „bearbeitet“) und ihre Anhänge (Vorschau,
  // „Öffnen“, „Laden“, Player; R00030 B0732) sind nur Stopp, solange ihre Nachricht der Stopp ist.
  function schalterRollen(stoppNachricht) {
    for (const schalterEinzeln of inhalt.querySelectorAll('.lc-msg button, .lc-msg a[href], .lc-msg video, .lc-msg audio')) {
      let soll = '-1';
      if (schalterEinzeln.closest('.lc-msg') === stoppNachricht) {
        soll = '0';
      }
      if (schalterEinzeln.getAttribute('tabindex') !== soll) {
        schalterEinzeln.setAttribute('tabindex', soll);
      }
    }
  }
  new MutationObserver(() => {
    rollfokusPflegen();
    feldEinsetzen(false);
    fokusZurueckAufNachricht();
  }).observe(inhalt, { childList: true, subtree: true });
  log.addEventListener('focusin', (ereignis) => {
    const nachricht = ereignis.target.closest('.lc-msg');
    if (nachricht !== null && inhalt.contains(nachricht)) {
      rollZeit = nachricht.dataset.zeit;
      fokusZeit = rollZeit;
      rollfokusPflegen();
    }
  });

  // Ansagen aus den Playern (R00030 F0165, ux-reviewer S2): der Fehlerhinweis eines Mediums mit Fokus geht an die Region.
  inhalt.addEventListener('lc-ansage', (ereignis) => ansagen(ereignis.detail));

  // Wird die fokussierte Nachricht ersetzt (der jüngste Block wird neu gezeichnet), fällt der Fokus mit dem alten Knoten
  // auf die Seite. Der Verlauf merkt sich die Zeit der Nachricht, die den Fokus hat, und legt ihn auf den neuen Knoten
  // derselben Zeit — ohne zu scrollen, ohne Ansage (R00041 F0141 B0634). Verlässt der Fokus den Knoten, ohne dass er
  // verschwindet, ist nichts mehr zu halten: Chromium meldet `focusout` auch beim Entfernen noch am verbundenen
  // Knoten, erst nach dem laufenden Zug zeigt `isConnected`, was geschah.
  let fokusZeit = null;
  log.addEventListener('focusout', (ereignis) => {
    setTimeout(() => {
      if (ereignis.target.isConnected && !inhalt.contains(document.activeElement)) {
        fokusZeit = null;
      }
    }, 0);
  });

  function fokusZurueckAufNachricht() {
    const aktiv = document.activeElement;
    const fokusVerloren = aktiv === null || aktiv === document.body;
    if (fokusZeit === null || !fokusVerloren) {
      return;
    }
    const neu = inhalt.querySelector(`:scope > .lc-msg[data-zeit="${CSS.escape(fokusZeit)}"]`);
    if (neu !== null) {
      neu.focus({ preventScroll: true });
    }
  }

  // ↑ in der leeren Eingabe: die jüngste eigene, bestätigte, nicht gelöschte Nachricht unter den neuesten 100; keine
  // → nichts. Steht der Verlauf nicht am Neuesten, öffnet er zuerst neu (wie beim Senden, ADR-0037 Regel 1).
  // `nochGueltig` fragt nach dem Neuöffnen, ob die Eingabe noch leer ist und den Fokus hat: wer inzwischen weitertippt,
  // behält ihn (Karte #82).
  async function eigeneLetzteFokussieren(nochGueltig = () => true) {
    if (fenster !== null && fenster.neuer) {
      const ergebnis = await oeffnen(offenerName);
      if (ergebnis.art !== 'geladen' || !nochGueltig()) {
        return;
      }
    }
    const liste = nachrichtenImVerlauf();
    const stelle = eigeneLetzte(liste.map((einzeln) => einzeln.dataset), eigeneKennung);
    if (stelle !== null) {
      nachrichtFokussieren(liste[stelle]);
    }
  }

  function pfeilBedienen(ereignis) {
    const richtung = pfeilRichtung(ereignis);
    if (richtung === null) {
      return;
    }
    ereignis.preventDefault();
    const liste = nachrichtenImVerlauf();
    const ziel = pfeilZiel(richtung, liste.indexOf(ereignis.target), liste.length);
    if (ziel !== null) {
      nachrichtFokussieren(liste[ziel]);
    }
  }

  // Fokus auf die Nachricht und in Sicht bringen, ohne den Browser springen zu lassen: nicht unter den klebenden
  // Tagestrenner, nicht unter den unteren Rand. Das Scrollen löst das Nachladen aus wie jedes Scrollen.
  function nachrichtFokussieren(nachricht) {
    nachricht.focus({ preventScroll: true });
    inSichtBringen(nachricht);
  }

  // Ein Element einer Nachricht (sie selbst, ihr Bearbeiten-Feld) ganz in den sichtbaren Teil des Verlaufs.
  function inSichtBringen(element) {
    const rahmenLage = log.getBoundingClientRect();
    const lage = element.getBoundingClientRect();
    const obereGrenze = rahmenLage.top + klebenderTrennerUnterkante(element.closest('.lc-msg'));
    if (lage.top < obereGrenze) {
      log.scrollTop -= obereGrenze - lage.top;
    } else if (lage.bottom > rahmenLage.bottom) {
      log.scrollTop += Math.min(lage.bottom - rahmenLage.bottom, lage.top - obereGrenze);
    }
  }

  // Das Zitat einer Antwort führt zu ihrem Original (R00047 B0717, B0718, ADR-0053 Regel 8, Vorprüfung `ux-reviewer` Z4);
  // Klick, Antippen und Enter kommen als Klick an. Der Sprung wird nicht angesagt. Ein entfallenes Original holt der Tab
  // gar nicht erst: der Hinweis steht am Zitat und wird angesagt; scheitert der Abruf, steht dort der Fehler, wird ohne ⚠
  // angesagt (UX S2, Karte #79), und der nächste Klick versucht es neu.
  const ENTFALLEN = 'Die ältere Nachricht gibt es nicht mehr (Aufbewahrungsfrist abgelaufen).';
  const NICHT_GELADEN = '⚠ Original nicht geladen: keine Antwort vom Server. Zum Wiederholen erneut anklicken.';
  async function zitatBedienen(zitat) {
    zitatHinweisSetzen(zitat, null);
    if (zitat.dataset.zustand === 'entfallen') {
      zitatHinweisSetzen(zitat, ENTFALLEN);
      ansagen(ENTFALLEN);
      return;
    }
    const ergebnis = await springen(zitat.dataset.zeit);
    if (ergebnis === 'nicht-geladen') {
      zitatHinweisSetzen(zitat, NICHT_GELADEN);
      ansagen(NICHT_GELADEN.replace(/^⚠ /, ''));
    }
    if (ergebnis === 'nicht-da') {
      zitatHinweisSetzen(zitat, ENTFALLEN);
      ansagen(ENTFALLEN);
    }
  }

  // Der Hinweis steht unter dem Zitat; null nimmt ihn weg. Ist das Zitat inzwischen neu gezeichnet, gibt es nichts zu zeigen.
  function zitatHinweisSetzen(zitat, text) {
    const vorhanden = zitat.nextElementSibling;
    if (vorhanden !== null && vorhanden.classList.contains('lc-zitat-hinweis')) {
      vorhanden.remove();
    }
    if (text === null || !zitat.isConnected) {
      return;
    }
    const hinweisZeile = hinweis(text, 'lc-zitat-hinweis');
    hinweisZeile.classList.toggle('lc-fehler', text === NICHT_GELADEN);
    zitat.after(hinweisZeile);
  }

  // Der Sprung zu einer Nachricht über ihre Zeit (ADR-0053 Regel 8), der gemeinsame Baustein für Zitat und Suche (I0011):
  // (a) steht sie im Fenster, kommt sie zentriert ins Bild, wird hervorgehoben und bekommt den Fokus; (b) sonst holt ein
  // eigener Abruf den Block ab ihr — erst nach seinem Erfolg ersetzt er das Fenster, dann wie (a); (c) scheitert der Abruf
  // oder fehlt sie im Block, bleibt das Fenster, wie es ist. Ein Wechsel des Channels oder ein neuer Sprung bricht ab.
  // → 'gesprungen' | 'nicht-geladen' | 'nicht-da' | 'abgebrochen'
  let sprungLauf = new AbortController();
  let waehrendDesSprungs = null;
  async function springen(zeit) {
    const knoten = nachrichtKnoten(zeit);
    if (knoten !== null) {
      hervorheben(knoten);
      return 'gesprungen';
    }
    sprungLauf.abort();
    sprungLauf = new AbortController();
    const dieser = sprungLauf;
    const kanal = laufend;
    kanal.signal.addEventListener('abort', () => dieser.abort(), { once: true });
    waehrendDesSprungs = [];
    const ergebnis = await abrufen(offenerName, { nach: zeitDavor(zeit) }, dieser.signal);
    if (dieser !== sprungLauf || kanal !== laufend) {
      return 'abgebrochen';
    }
    const waehrendDesAbrufs = waehrendDesSprungs;
    waehrendDesSprungs = null;
    if (ergebnis.art !== 'block') {
      return 'nicht-geladen';
    }
    const zeitpunkt = Date.parse(zeit);
    if (!ergebnis.block.nachrichten.some((nachricht) => Date.parse(nachricht.zeit) === zeitpunkt)) {
      return 'nicht-da';
    }
    fensterDurchBlockErsetzen(ergebnis.block);
    fassungenErsetzen(waehrendDesAbrufs.filter((eintrag) => eintrag.fassung !== undefined).map((eintrag) => eintrag.fassung));
    zitateNachfuehren(waehrendDesAbrufs.filter((eintrag) => eintrag.zitat !== undefined).map((eintrag) => eintrag.zitat));
    hervorheben(nachrichtKnoten(zeit));
    lageMerken();
    pruefen();
    return 'gesprungen';
  }

  // Das Fenster ist danach genau der geholte Block wie beim Öffnen, aber ohne „ans Ende“ (ADR-0053 Regel 8 b); laufende
  // Abrufe des alten Fensters (Nachladen, Neu-Lesen) gelten nicht mehr. Das Nachladen oben und unten übernimmt danach
  // ADR-0026 Regel 2, „Zum Neuesten“ führt zurück.
  function fensterDurchBlockErsetzen(block) {
    laufend.abort();
    laufend = new AbortController();
    ladenBeendet();
    neueGezaehlt = 0;
    ersteNeue = null;
    nurGezaehlte = [];
    eingeordnet = new Set();
    angenommenMerken(block.nachrichten);
    eingeordnetMerken(block.nachrichten);
    fenster = { bloecke: [block.nachrichten], aelter: block.aelter, neuer: block.neuer };
    juengste = block.nachrichten[block.nachrichten.length - 1].zeit;
    auslauf.style.height = '';
    const knoten = gegliedertZeichnen(block.nachrichten, null, { heute, kennung: eigeneKennung });
    bloeckeKnoten = [knoten];
    inhalt.replaceChildren(...knoten);
    langeEinklappen(nurNachrichten(knoten));
    amEnde = false;
  }

  // Hervorhebung (Vorprüfung `ux-reviewer`): 2000 ms Grund, Balken und Fokusring per CSS-Klasse — nicht nur durch Farbe —,
  // zentriert ohne Animation, Fokus ohne weiteres Scrollen. Wird die Nachricht in der Zeit neu gezeichnet, endet die
  // Hervorhebung mit dem alten Knoten.
  const HERVORHEBUNG_MS = 2000;
  let hervorhebungEnde = null;
  function hervorheben(knoten) {
    for (const vorige of inhalt.querySelectorAll('.lc-hervorgehoben')) {
      vorige.classList.remove('lc-hervorgehoben');
    }
    clearTimeout(hervorhebungEnde);
    knoten.scrollIntoView({ block: 'center', behavior: 'instant' });
    knoten.classList.add('lc-hervorgehoben');
    // Frisch gezeichnete Nachrichten sind erst nach dem Rollfokus fokussierbar (tabindex); er wird hier sofort gepflegt.
    rollZeit = knoten.dataset.zeit;
    rollfokusPflegen();
    knoten.focus({ preventScroll: true });
    hervorhebungEnde = setTimeout(() => knoten.classList.remove('lc-hervorgehoben'), HERVORHEBUNG_MS);
  }

  // Öffnet den Channel mit den neuesten 100 (ADR-0037 Ergänzung „Karte #61“, 7h), mit `cursor` `vor` mit dem Block
  // davor (Rückkehr zum offenen Feld): → { art: 'geladen' | 'fehler' | 'abgebrochen' }. Bricht der Aufrufer über `signal`
  // ab (Zeitgrenze, Ende des Laufs), gilt das als Fehlschlag; übernimmt ein anderes Öffnen oder schließt der Verlauf, ist
  // es abgebrochen. Ein erneutes Öffnen nach „Verlauf nicht geladen“ lässt den Hinweis bis zum Ergebnis stehen (7k).
  async function oeffnen(name, signal = new AbortController().signal, cursor = {}) {
    const rueckkehr = name !== offenerName;
    aufbewahren();
    laufend.abort();
    laufend = new AbortController();
    const dieser = laufend;
    rollZeit = null;
    fokusZeit = null;
    schwelleSicht = log.clientHeight;
    waehrendDesSprungs = null;
    signal.addEventListener('abort', () => dieser.abort());
    const hinweisBleibt = name === offenerName && nichtGeladenSteht();
    offenerName = name;
    fenster = null;
    bloeckeKnoten = [];
    gescheitert = false;
    geladen = false;
    puffer = [];
    juengste = null;
    neueGezaehlt = 0;
    ersteNeue = null;
    nurGezaehlte = [];
    angenommen = new Set();
    eingeordnet = new Set();
    auslauf.style.height = '';
    ladenBeendet();
    log.setAttribute('aria-label', `Verlauf ${ortText(name)}`);
    if (!hinweisBleibt) {
      inhalt.replaceChildren();
    }
    lageMerken();
    const ergebnis = await abrufen(name, cursor, dieser.signal);
    const vomAufruferAbgebrochen = ergebnis.art === 'abgebrochen' && signal.aborted;
    if (dieser !== laufend || (ergebnis.art === 'abgebrochen' && !vomAufruferAbgebrochen)) {
      return { art: 'abgebrochen' };
    }
    if (ergebnis.art !== 'block') {
      nichtGeladen();
      return { art: 'fehler' };
    }
    geladen = true;
    if (ergebnis.block.nachrichten.length === 0) {
      inhalt.replaceChildren(...leerHinweis(name));
      gepuffertesAnnehmen();
      feldNachOeffnen(rueckkehr);
      return { art: 'geladen' };
    }
    angenommenMerken(ergebnis.block.nachrichten);
    eingeordnetMerken(ergebnis.block.nachrichten);
    fenster = { bloecke: [ergebnis.block.nachrichten], aelter: ergebnis.block.aelter, neuer: ergebnis.block.neuer };
    juengste = ergebnis.block.nachrichten[ergebnis.block.nachrichten.length - 1].zeit;
    const knoten = gegliedertZeichnen(ergebnis.block.nachrichten, null, { heute, kennung: eigeneKennung });
    bloeckeKnoten = [knoten];
    inhalt.replaceChildren(...knoten);
    medienAblage.zurueck(name, inhalt);
    langeEinklappen(nurNachrichten(knoten));
    ansEnde();
    lageMerken();
    feldNachOeffnen(rueckkehr);
    gepuffertesAnnehmen();
    pruefen();
    return { art: 'geladen' };
  }

  // Wiedergabe beim Verlassen (R00030 B0754): wer den Inhalt verlässt, hält jedes Medium an; was schon gespielt hat,
  // wartet je Channel in der Ablage und kehrt mit dem Channel an seine Stelle zurück.
  const medienAblage = medienAblageAnlegen();
  function aufbewahren() {
    if (offenerName !== null) {
      medienAblage.aufbewahren(offenerName, inhalt);
    }
  }

  // „Verlauf nicht geladen“: steht der Hinweis schon, bleibt er derselbe Knoten (kein zweiter Alarm). Was bis hier
  // gepuffert war, fällt weg, und bis zum nächsten Öffnen wird nichts gepuffert — das nächste Laden bringt es (7j).
  function nichtGeladen() {
    gescheitert = true;
    puffer = [];
    if (nichtGeladenSteht()) {
      hinweisNachziehen();
      return;
    }
    const fehler = hinweis(nichtGeladenText(selbstversuchend()), 'lc-mitte-hinweis lc-fehler');
    fehler.setAttribute('role', 'alert');
    inhalt.replaceChildren(fehler);
  }

  function nichtGeladenSteht() {
    return inhalt.querySelector('[role="alert"]') !== null;
  }

  // Ändert sich der Zustand, während der Hinweis steht (Trennung, Rückkehr mit Laden-Lauf), tauscht derselbe Knoten
  // nur den Text — bei Unterschied.
  function hinweisNachziehen() {
    const knoten = inhalt.querySelector('[role="alert"]');
    if (knoten !== null) {
      hinweisTextNachziehen(knoten, selbstversuchend());
    }
  }

  // Was vor dem Block ankam, geht jetzt den Weg jeder neuen Nachricht bzw. Fassung (ADR-0028 Regel 5, ADR-0048 Regel 11).
  function gepuffertesAnnehmen() {
    const gepuffert = puffer;
    puffer = [];
    gepuffert.forEach(({ nachricht, eigen, fassung, zitat }) => {
      if (fassung !== undefined) {
        fassungAnnehmen(offenerName, fassung);
        return;
      }
      if (zitat !== undefined) {
        zitatAnnehmen(offenerName, zitat);
        return;
      }
      nachrichtAnnehmen(nachricht, eigen);
    });
  }

  // Eine geänderte Fassung einer Nachricht des offenen Channels: vor dem ersten Block gepuffert, sonst im nächsten Frame
  // nach den neuen Nachrichten übernommen — so trifft sie auch eine Nachricht, die im selben Frame erst ankommt.
  function fassungAnnehmen(channel, fassung) {
    if (channel !== offenerName || offenerName === null || gescheitert) {
      return;
    }
    if (waehrendDesSprungs !== null) {
      waehrendDesSprungs.push({ fassung });
    }
    if (!geladen) {
      puffer.push({ fassung });
      return;
    }
    buendler.annehmen({ channel, nachricht: null, fassung, eigen: false });
  }

  // Anhänge, die gelöscht wurden (Ereignis `anhaenge`, R00030 B0518): der Tab vermerkt ihre IDs, auch für Channels und
  // Nachrichten, die nicht im Fenster stehen; im offenen Verlauf ersetzt die Zeile „Gelöscht“ im nächsten Frame Vorschau
  // oder Player. geloescht: [{ id, zeit }] (Zeit der Nachricht)
  function anhaengeAnnehmen(channel, geloescht) {
    geloescht.forEach(({ id }) => geloeschteIds.add(id));
    const nichtOffen = channel !== offenerName || offenerName === null || gescheitert || !geladen;
    if (nichtOffen || geloescht.length === 0) {
      return;
    }
    buendler.annehmen({ channel, nachricht: null, geloescht, eigen: false });
  }

  // Das neue Zitat einer geänderten Nachricht (R00047 B0721, ADR-0053 Regel 5) geht denselben Weg wie ihre Fassung: vor dem
  // ersten Block gepuffert, während eines Sprungs gemerkt, sonst im nächsten Frame übernommen.
  function zitatAnnehmen(channel, zitat) {
    if (channel !== offenerName || offenerName === null || gescheitert) {
      return;
    }
    if (waehrendDesSprungs !== null) {
      waehrendDesSprungs.push({ zitat });
    }
    if (!geladen) {
      puffer.push({ zitat });
      return;
    }
    buendler.annehmen({ channel, nachricht: null, zitat, eigen: false });
  }

  // Eine Nachricht des offenen Channels, fremd oder eigen bestätigt; vor dem ersten Block gepuffert, nach „Verlauf nicht
  // geladen“ verworfen.
  function nachrichtAnnehmen(nachricht, eigen) {
    if (offenerName === null || gescheitert) {
      return;
    }
    angenommenMerken([nachricht]);
    if (!geladen) {
      puffer.push({ nachricht, eigen });
      return;
    }
    buendler.annehmen({ channel: offenerName, nachricht, eigen });
  }

  // Nachgeholtes wie Live-Nachrichten annehmen; gezählt wird nur, was der Tab noch nicht angenommen hatte — eine
  // während des Abrufs live angekommene Nachricht steht oft auch im Block. → Zahl der erst jetzt angenommenen
  function nachgeholtAnnehmen(nachrichten) {
    const unbekannte = new Set(nachrichten.map((nachricht) => Date.parse(nachricht.zeit)).filter((zeitpunkt) => !angenommen.has(zeitpunkt)));
    nachrichten.forEach((nachricht) => nachrichtAnnehmen(nachricht, false));
    return unbekannte.size;
  }

  // Je Zeitpunkt der erste Eintrag, der noch nirgends eingeordnet ist.
  function nochNichtEingeordnet(eintraege) {
    const imStapel = new Set();
    return eintraege.filter((eintrag) => {
      const zeitpunkt = Date.parse(eintrag.nachricht.zeit);
      const neu = !eingeordnet.has(zeitpunkt) && !imStapel.has(zeitpunkt);
      imStapel.add(zeitpunkt);
      return neu;
    });
  }

  function eingeordnetMerken(nachrichten) {
    nachrichten.forEach((nachricht) => eingeordnet.add(Date.parse(nachricht.zeit)));
  }

  function angenommenMerken(nachrichten) {
    nachrichten.forEach((nachricht) => angenommen.add(Date.parse(nachricht.zeit)));
  }

  function vorlaeufigeZeigen(liste) {
    vorlaeufige = liste;
    buendler.annehmen({ channel: offenerName, nachricht: null, eigen: true });
  }

  // Einmal je Frame (ADR-0037 Regel 6): neue Nachrichten einordnen, folgen oder zählen, dann die vorläufigen neu —
  // eine Anker-Korrektur, ein Abgleich des Fensters.
  function jeFrameEinfuegen(gesammelt) {
    const sichtbar = auslaufSichtbar();
    const inhaltVorher = inhaltsHoehe();
    const offene = gesammelt.filter((eintrag) => eintrag.channel === offenerName && eintrag.nachricht !== null);
    if (offene.length > 0 && !gescheitert) {
      nachrichtenEinfuegen(offene);
    }
    const fassungen = gesammelt.filter((eintrag) => eintrag.channel === offenerName && eintrag.fassung !== undefined);
    if (fassungen.length > 0 && !gescheitert) {
      fassungenErsetzen(fassungen.map((eintrag) => eintrag.fassung));
    }
    const zitate = gesammelt.filter((eintrag) => eintrag.channel === offenerName && eintrag.zitat !== undefined);
    if (zitate.length > 0 && !gescheitert) {
      zitateNachfuehren(zitate.map((eintrag) => eintrag.zitat));
    }
    const loeschungen = gesammelt.filter((eintrag) => eintrag.channel === offenerName && eintrag.geloescht !== undefined);
    if (loeschungen.length > 0 && !gescheitert) {
      anhaengeErsetzen(loeschungen.flatMap((eintrag) => eintrag.geloescht));
    }
    ausstehendZeichnen();
    auslaufAbbauen(sichtbar, inhaltsHoehe() - inhaltVorher);
    if (amEnde) {
      ansEnde();
    }
    lageMerken();
  }

  function inhaltsHoehe() {
    return inhalt.getBoundingClientRect().height + ausstehend.getBoundingClientRect().height;
  }

  function nachrichtenEinfuegen(alleEintraege) {
    const eintraege = nochNichtEingeordnet(alleEintraege);
    if (eintraege.length === 0) {
      return;
    }
    eingeordnetMerken(eintraege.map((eintrag) => eintrag.nachricht));
    const fremde = eintraege.filter((eintrag) => !eintrag.eigen).length;
    const neuesterImFenster = fenster === null || !fenster.neuer;
    const lage = folgenOderZaehlen(abstandUnten(), zeilenhoehe(), neuesterImFenster, fremde > 0 && mediumSpielt(inhalt));
    if (lage === 'nurZaehlen') {
      neueGezaehlt += fremde;
      nurGezaehlte.push(...eintraege.map((eintrag) => eintrag.nachricht));
      return;
    }
    const neue = neueImJuengstenBlock(eintraege.map((eintrag) => eintrag.nachricht));
    if (neue.length === 0) {
      return;
    }
    const neueFremde = neue.filter((nachricht) => eintraege.some((eintrag) => eintrag.nachricht === nachricht && !eintrag.eigen));
    if (lage === 'zaehlen' && neueFremde.length > 0 && ersteNeue === null) {
      ersteNeue = neueFremde[0].zeit;
    }
    const anker = ankerMerken(log, inhalt);
    inDenJuengstenBlock(neue);
    ankerHalten(log, anker);
    if (lage === 'folgen') {
      amEnde = true;
      return;
    }
    amEnde = false;
    neueGezaehlt += neueFremde.length;
  }

  // Was neu in den jüngsten Block kommt (ADR-0037 Ergänzung Karte #22/#36, 7f): jede Zeit einmal, auch Nachrichten der
  // Lücke, die älter sind als eine schon live gezeigte — im leeren Channel auch älter als sein Anfang.
  function neueImJuengstenBlock(nachrichten) {
    if (fenster === null) {
      return zusammenfuehren([], nachrichten).neue;
    }
    return zusammenfuehren(fenster.bloecke[fenster.bloecke.length - 1], nachrichten).neue;
  }

  // Nach einer Lücke (ADR-0048 Regel 11, F1): das Fenster neu lesen und Fassungen mit größerem `geaendert` übernehmen
  // (Ausnahme zu ADR-0028 Regel 5 nur für dieses Feld); ein Wechsel verwirft das Ergebnis.
  async function fassungenAuffrischen() {
    if (fenster === null) {
      return;
    }
    const dieser = laufend;
    const channel = offenerName;
    await fensterNeuLesen({
      abrufen: (cursor) => abrufen(channel, cursor, dieser.signal),
      anfang: fenster.bloecke[0][0].zeit,
      juengste: juengsteImFenster(),
      uebernehmen: (nachricht) => {
        fassungAnnehmen(channel, nachricht);
        anhaengeAnnehmen(channel, geloeschteAnhaenge(nachricht));
        if (nachricht.antwort_auf !== undefined && nachricht.antwort_auf !== null) {
          zitatAnnehmen(channel, nachricht.antwort_auf);
        }
      },
      gilt: () => dieser === laufend,
    });
  }

  function juengsteImFenster() {
    const letzter = fenster.bloecke[fenster.bloecke.length - 1];
    return letzter[letzter.length - 1].zeit;
  }

  // Geänderte Fassungen ersetzen ihre Nachricht an Ort und Stelle, wenn sie im Fenster steht und die Fassung neuer ist
  // (fassungUebernehmen); die Lesestelle bleibt (Anker), der Fokus folgt dem neuen Knoten (fokusZurueckAufNachricht).
  function fassungenErsetzen(fassungen) {
    const anker = ankerMerken(log, inhalt);
    fassungen.forEach(fassungErsetzen);
    ankerHalten(log, anker);
  }

  // Steht an der Nachricht das offene Feld (UX M3): gelöscht schließt es mit Hinweis und einer Ansage — beim eigenen
  // Löschen ohne beides (UX S3), die Ansage „Nachricht gelöscht.“ kommt vom Löschen —, der Fokus geht auf die Nachricht;
  // bearbeitet merkt sich nur die neue Fassung — Feld, Text und Schreibmarke bleiben, gezeigt wird sie nach dem Schließen.
  function fassungErsetzen(fassung) {
    const bekannt = bekannteNachricht(fassung.zeit);
    if (bekannt === null) {
      return;
    }
    const uebernommen = fassungUebernehmen(bekannt.nachricht, fassung);
    if (uebernommen === null) {
      return;
    }
    const feldZeit = bearbeiten.offeneZeitIn(offenerName);
    const feldDaran = feldZeit !== null && Date.parse(feldZeit) === Date.parse(fassung.zeit);
    if (feldDaran && uebernommen.zustand !== 'geloescht') {
      fenster.bloecke[bekannt.blockStelle][bekannt.stelle] = uebernommen;
      return;
    }
    if (feldDaran) {
      feldSchliessenWeilGeloescht(fassung.zeit);
    }
    knotenErsetzen(bekannt, uebernommen);
    if (feldDaran) {
      nachrichtFokussieren(nachrichtKnoten(uebernommen.zeit));
    }
  }

  // Die Zeile „Gelöscht“ ersetzt Vorschau und Player; die Lesestelle bleibt (Anker). Hatte ein ersetztes Element den Fokus,
  // liegt er danach auf der Nachricht und die Ansage nennt den Anhang einmal. Steht an der Nachricht das Bearbeiten-Feld,
  // merkt sich das Fenster nur den neuen Stand, gezeichnet wird nach dem Schließen.
  function anhaengeErsetzen(geloescht) {
    const idsJeZeit = new Map();
    for (const { id, zeit } of geloescht) {
      const bekannt = bekannteNachricht(zeit);
      if (bekannt === null) {
        continue;
      }
      if (!idsJeZeit.has(bekannt.nachricht.zeit)) {
        idsJeZeit.set(bekannt.nachricht.zeit, new Set());
      }
      idsJeZeit.get(bekannt.nachricht.zeit).add(id);
    }
    const anker = ankerMerken(log, inhalt);
    idsJeZeit.forEach((ids, zeit) => nachrichtOhneAnhaengeZeigen(zeit, ids));
    ankerHalten(log, anker);
  }

  function nachrichtOhneAnhaengeZeigen(zeit, ids) {
    const bekannt = bekannteNachricht(zeit);
    const neu = anhaengeAlsGeloescht(bekannt.nachricht, ids);
    if (neu === bekannt.nachricht) {
      return;
    }
    const feldZeit = bearbeiten.offeneZeitIn(offenerName);
    if (feldZeit !== null && Date.parse(feldZeit) === Date.parse(zeit)) {
      fenster.bloecke[bekannt.blockStelle][bekannt.stelle] = neu;
      return;
    }
    const verlorenerName = nameDesAnhangsMitFokus(bekannt.nachricht, ids);
    knotenErsetzen(bekannt, neu);
    if (verlorenerName !== null) {
      nachrichtFokussieren(nachrichtKnoten(zeit));
      ansagen(`Anhang gelöscht: ${verlorenerName}`);
    }
  }

  function nameDesAnhangsMitFokus(nachricht, ids) {
    const knoten = nachrichtKnoten(nachricht.zeit);
    if (knoten === null) {
      return null;
    }
    const betroffen = nachricht.anhaenge.filter((anhang) => ids.has(anhang.id) && anhang.zustand !== 'geloescht');
    const mitFokus = betroffen.find((anhang) => {
      const anhangKnoten = knoten.querySelector(`.lc-anhang[data-anhang="${CSS.escape(anhang.id)}"]`);
      return anhangKnoten !== null && anhangKnoten.contains(document.activeElement);
    });
    if (mitFokus === undefined) {
      return null;
    }
    return mitFokus.name;
  }

  // Jede Antwort im Fenster auf die geänderte Nachricht zeigt ihr neues Zitat (zitatNachfuehren); die Lesestelle bleibt.
  // Steht an der Antwort das Bearbeiten-Feld, merkt sich das Fenster nur den neuen Stand, gezeichnet wird nach dem Schließen.
  function zitateNachfuehren(zitate) {
    if (fenster === null) {
      return;
    }
    const anker = ankerMerken(log, inhalt);
    const feldZeit = bearbeiten.offeneZeitIn(offenerName);
    for (const zitat of zitate) {
      const ereignis = { nachricht: { zeit: zitat.zeit }, zitat };
      fenster.bloecke.forEach((block, blockStelle) => {
        zitatNachfuehren(block, ereignis).forEach((neu, stelle) => {
          if (neu === block[stelle]) {
            return;
          }
          const feldDaran = feldZeit !== null && Date.parse(feldZeit) === Date.parse(neu.zeit);
          if (feldDaran) {
            block[stelle] = neu;
            return;
          }
          knotenErsetzen({ blockStelle, stelle }, neu);
        });
      });
    }
    ankerHalten(log, anker);
  }

  function feldSchliessenWeilGeloescht(zeit) {
    const selbstGeloescht = loeschenLaeuftFuer !== null && loeschenLaeuftFuer.channel === offenerName
      && Date.parse(loeschenLaeuftFuer.zeit) === Date.parse(zeit);
    if (selbstGeloescht) {
      bearbeiten.weilSelbstGeloeschtSchliessen();
      return;
    }
    bearbeiten.weilWegSchliessen();
  }

  // Schließt das Feld, zeigt die Nachricht wieder die Fassung, die der Verlauf kennt — auch eine, die während des
  // Bearbeitens kam.
  function nachFeldNeuZeichnen(zeit) {
    const bekannt = bekannteNachricht(zeit);
    if (bekannt !== null && nachrichtKnoten(zeit) !== null) {
      knotenErsetzen(bekannt, bekannt.nachricht);
    }
  }

  // Antwortet das Speichern mit 409 (PO 122), ist die Nachricht gelöscht, auch wenn ihr Ereignis noch nicht da ist —
  // gelöscht ist endgültig, ein späteres Ereignis ändert nichts mehr.
  function geloeschtSetzen(zeit) {
    const bekannt = bekannteNachricht(zeit);
    if (bekannt !== null) {
      knotenErsetzen(bekannt, { ...bekannt.nachricht, text: '', anhaenge: [], zustand: 'geloescht', antwort_auf: null });
    }
  }

  function knotenErsetzen({ blockStelle, stelle }, uebernommen) {
    const zeitpunkt = Date.parse(uebernommen.zeit);
    fenster.bloecke[blockStelle][stelle] = uebernommen;
    const knoten = bloeckeKnoten[blockStelle];
    const knotenStelle = knoten.findIndex((einzeln) => einzeln.classList.contains('lc-msg') && Date.parse(einzeln.dataset.zeit) === zeitpunkt);
    const alt = knoten[knotenStelle];
    const neu = nachrichtZeichnen(uebernommen, eigeneKennung);
    kopfSetzen(neu, alt.classList.contains('first'));
    medienUebernehmen(alt, neu);
    alt.replaceWith(neu);
    knoten[knotenStelle] = neu;
    langeEinklappen([neu]);
  }

  // Sind alle neuen jünger als die jüngste gezeigte, werden sie angefügt; sonst wird der jüngste Block in der Folge der
  // Zeiten neu gezeichnet.
  function inDenJuengstenBlock(neue) {
    const letzteNeue = neue[neue.length - 1].zeit;
    const alleJuenger = juengste === null || Date.parse(neue[0].zeit) > Date.parse(juengste);
    if (juengste === null || Date.parse(letzteNeue) > Date.parse(juengste)) {
      juengste = letzteNeue;
    }
    if (alleJuenger) {
      anfuegen(neue);
      return;
    }
    juengstenBlockNeuZeichnen(neue);
  }

  function juengstenBlockNeuZeichnen(neue) {
    const stelle = fenster.bloecke.length - 1;
    const folge = zusammenfuehren(fenster.bloecke[stelle], neue).folge;
    let vorige = null;
    if (stelle > 0) {
      const davor = fenster.bloecke[stelle - 1];
      vorige = davor[davor.length - 1];
    }
    const knoten = gegliedertZeichnen(folge, vorige, { heute, kennung: eigeneKennung }, ersteNeue);
    medienInNeueKnotenZiehen(bloeckeKnoten[stelle], knoten);
    bloeckeKnoten[stelle].forEach((einzeln) => einzeln.remove());
    inhalt.append(...knoten);
    fenster.bloecke[stelle] = folge;
    bloeckeKnoten[stelle] = knoten;
    langeEinklappen(nurNachrichten(knoten));
  }

  // Beim Neuzeichnen des Blocks zieht jeder Player in den neuen Knoten seiner Nachricht um (R00030 B0734).
  function medienInNeueKnotenZiehen(alte, neue) {
    const neueJeZeit = new Map(nurNachrichten(neue).map((einzeln) => [einzeln.dataset.zeit, einzeln]));
    nurNachrichten(alte).forEach((alt) => {
      const neu = neueJeZeit.get(alt.dataset.zeit);
      if (neu !== undefined) {
        medienUebernehmen(alt, neu);
      }
    });
  }

  // Neue Nachrichten ans Ende des jüngsten Blocks; im leeren Channel entsteht der erste Block. Die erste gezählte neue
  // trägt ihren Kopf (R00020 F0067).
  function anfuegen(neue) {
    if (fenster === null) {
      fenster = { bloecke: [neue], aelter: false, neuer: false };
      const knoten = gegliedertZeichnen(neue, null, { heute, kennung: eigeneKennung }, ersteNeue);
      bloeckeKnoten = [knoten];
      inhalt.replaceChildren(...knoten);
      langeEinklappen(nurNachrichten(knoten));
      return;
    }
    const letzter = fenster.bloecke[fenster.bloecke.length - 1];
    const knoten = gegliedertZeichnen(neue, letzter[letzter.length - 1], { heute, kennung: eigeneKennung }, ersteNeue);
    letzter.push(...neue);
    bloeckeKnoten[bloeckeKnoten.length - 1].push(...knoten);
    inhalt.append(...knoten);
    langeEinklappen(nurNachrichten(knoten));
  }

  // Neu gezeichnet bleibt der Fokus auf seinem Schalter; gibt es ihn nicht mehr (die verworfene ist verfallen), geht er
  // ins Feld, nie auf body (R00025 F0085, V2).
  function ausstehendZeichnen() {
    const fokussiert = fokussierterSchalter();
    const eigene = vorlaeufige.filter((vorlaeufig) => vorlaeufig.channel === offenerName);
    vorlaeufigeKnoten.replaceChildren(...eigene.map((vorlaeufig) => vorlaeufigZeichnen(vorlaeufig, eigeneKennung)));
    if (fokussiert !== null) {
      fokusZurueck(fokussiert);
    }
    erneutSendenFokussieren();
  }

  function fokussierterSchalter() {
    const aktiv = document.activeElement;
    if (aktiv === null || !ausstehend.contains(aktiv) || aktiv.dataset.aktion === undefined) {
      return null;
    }
    return { aktion: aktiv.dataset.aktion, schluessel: aktiv.closest('[data-schluessel]').dataset.schluessel };
  }

  let fokusWirdZurueckgelegt = false;
  function fokusZurueck({ aktion, schluessel }) {
    const derselbe = ausstehend.querySelector(`[data-schluessel="${CSS.escape(schluessel)}"] button[data-aktion="${aktion}"]`);
    if (derselbe === null) {
      fokusVerloren();
      return;
    }
    fokusWirdZurueckgelegt = true;
    derselbe.focus();
    fokusWirdZurueckgelegt = false;
  }
  ausstehend.addEventListener('focusin', (ereignis) => {
    const aufRueckgaengig = ereignis.target.matches('button[data-aktion="rueckgaengig"]');
    if (aufRueckgaengig && !fokusWirdZurueckgelegt) {
      fokusRueckgaengig(ereignis.target.closest('[data-schluessel]').dataset.schluessel);
    }
  });

  // Nach „Rückgängig“ liegt der Fokus auf „Erneut senden“ der Nachricht, sobald sie wieder gezeichnet ist (R00025 F0085).
  let fokusAufErneut = null;
  function erneutSendenFokussieren() {
    if (fokusAufErneut === null) {
      return;
    }
    const schalterDort = ausstehend.querySelector(`[data-schluessel="${CSS.escape(fokusAufErneut)}"] button[data-aktion="erneut"]`);
    if (schalterDort !== null) {
      fokusAufErneut = null;
      schalterDort.focus();
    }
  }

  // Senden holt ans Ende (ADR-0037 Regel 1): steht der neueste Block nicht im Fenster, öffnet der Tab neu; der Fokus
  // bleibt, wo er ist.
  function zumEndeFuerSenden() {
    if (fenster !== null && fenster.neuer) {
      oeffnen(offenerName);
      return;
    }
    amEnde = true;
    ansEnde();
  }

  // Höchstens ein Abruf zugleich; danach wird erneut geprüft — so holt auch ein Sprung an den Rand des Scrollbalkens
  // Block für Block nach, ohne Kaskade (ADR-0026 Regel 2).
  async function pruefen() {
    if (fenster === null) {
      return;
    }
    const richtung = nachladen({
      bloecke: fenster.bloecke.length,
      aelter: fenster.aelter,
      neuer: fenster.neuer,
      laeuft: log.dataset.laedt !== undefined,
      ...masse(),
    });
    if (richtung === null) {
      return;
    }
    const dieser = laufend;
    log.dataset.laedt = richtung;
    ladezeile.hidden = richtung !== 'aelter';
    const ergebnis = await abrufen(offenerName, cursorFuer(fenster, richtung), dieser.signal);
    if (dieser !== laufend) {
      return;
    }
    ladenBeendet();
    if (ergebnis.art !== 'block') {
      return;
    }
    blockEinfuegen(ergebnis.block, richtung);
    pruefen();
  }

  // Abstand des Blicks zu den Rändern und Höhe des obersten und untersten Blocks (fenster.js, Abwurfschutz); ein Ende
  // mit dem offenen Bearbeiten-Feld (F0142) oder einem spielenden Medium (R00030 B0754) ist geschützt.
  function masse() {
    return {
      abstandOben: log.scrollTop,
      abstandUnten: abstandUnten(),
      hoeheOben: blockHoehe(bloeckeKnoten[0]),
      hoeheUnten: blockHoehe(bloeckeKnoten[bloeckeKnoten.length - 1]),
      sicht: log.clientHeight,
      obenGeschuetzt: blockMitFeld(fenster.bloecke[0]) || blockSpielt(bloeckeKnoten[0]),
      untenGeschuetzt: blockMitFeld(fenster.bloecke[fenster.bloecke.length - 1]) || blockSpielt(bloeckeKnoten[bloeckeKnoten.length - 1]),
    };
  }

  function ladenBeendet() {
    delete log.dataset.laedt;
    ladezeile.hidden = true;
  }

  // Einfügen, am anderen Ende abwerfen, Kopf und Trenner an den Grenzen neu — dann den Anker korrigieren, alles im
  // selben Task vor dem nächsten Bild (ADR-0026 Regel 3).
  function blockEinfuegen(block, richtung) {
    angenommenMerken(block.nachrichten);
    eingeordnetMerken(block.nachrichten);
    const anker = ankerMerken(log, inhalt);
    const ergebnis = einfuegen(fenster, block, richtung, masse());
    if (block.nachrichten.length > 0) {
      knotenEinfuegen(block.nachrichten, richtung);
    }
    if (ergebnis.abgeworfen !== null) {
      knotenAbwerfen(richtung);
    }
    fenster = ergebnis.fenster;
    if (richtung === 'neuer') {
      juengste = einordnen(juengste, block.nachrichten).juengste;
    }
    if (richtung === 'neuer' && !fenster.neuer) {
      nurGezaehlteAnfuegen();
    }
    ankerHalten(log, anker);
    lageMerken();
  }

  // Nachrichten, die nur gezählt wurden, weil der neueste Block nicht im Fenster stand, kommen hinter ihn, sobald er
  // wieder da ist — was schon im Block steht, verwirft das Einordnen (ADR-0028 Regel 5); die Pille zählt weiter.
  function nurGezaehlteAnfuegen() {
    const { neue, juengste: neueJuengste } = einordnen(juengste, nurGezaehlte);
    nurGezaehlte = [];
    juengste = neueJuengste;
    if (neue.length > 0) {
      anfuegen(neue);
    }
  }

  function knotenEinfuegen(nachrichten, richtung) {
    if (richtung === 'aelter') {
      const knoten = gegliedertZeichnen(nachrichten, null, { heute, kennung: eigeneKennung });
      inhalt.prepend(...knoten);
      bloeckeKnoten.unshift(knoten);
      grenzeGliedern(nachrichten[nachrichten.length - 1], 1);
      langeEinklappen(nurNachrichten(knoten));
      return;
    }
    const letzte = fenster.bloecke[fenster.bloecke.length - 1];
    const knoten = gegliedertZeichnen(nachrichten, letzte[letzte.length - 1], { heute, kennung: eigeneKennung });
    inhalt.append(...knoten);
    bloeckeKnoten.push(knoten);
    langeEinklappen(nurNachrichten(knoten));
  }

  function knotenAbwerfen(richtung) {
    if (richtung === 'aelter') {
      bloeckeKnoten.pop().forEach((einzeln) => einzeln.remove());
      return;
    }
    bloeckeKnoten.shift().forEach((einzeln) => einzeln.remove());
    grenzeGliedern(null, 0);
  }

  // Die erste Nachricht des Blocks `index` steht jetzt nach `vorige` (oder ganz oben): Kopf und Tagestrenner neu.
  function grenzeGliedern(vorige, index) {
    const knoten = bloeckeKnoten[index];
    const artikel = knoten.find((einzeln) => einzeln.classList.contains('lc-msg'));
    const { kopf, tagestrenner } = gliederung(vorige, { zeit: artikel.dataset.zeit, von: artikel.dataset.von });
    kopfSetzen(artikel, kopf);
    const trenner = artikel.previousElementSibling;
    const hatTrenner = trenner !== null && trenner.classList.contains('lc-day');
    if (tagestrenner === null && hatTrenner) {
      trenner.remove();
      knoten.splice(knoten.indexOf(trenner), 1);
    }
    if (tagestrenner !== null && !hatTrenner) {
      const neuerTrenner = trennerZeichnen(tagestrenner, heute);
      artikel.before(neuerTrenner);
      knoten.splice(knoten.indexOf(artikel), 0, neuerTrenner);
    }
  }

  // Das heutige Datum des Servers ('JJJJ-MM-TT') benennt die Tagestrenner neu — gleiche Höhe, nichts springt.
  function heuteSetzen(datum) {
    if (datum === heute) {
      return;
    }
    heute = datum;
    for (const trenner of inhalt.querySelectorAll('.lc-day')) {
      trenner.textContent = datumBenennen(trenner.dataset.datum, heute);
    }
  }

  // Nach „Verlauf nicht geladen“ öffnet eine neue Wahl desselben Channels ihn neu (R00007); läuft ein Laden-Lauf, zieht
  // die Wahl stattdessen dessen Versuch vor (hauptansicht.js, ADR-0037 7i). Sonst bleibt der Verlauf, wie er ist.
  function erneutVersuchen() {
    if (gescheitert) {
      oeffnen(offenerName);
    }
  }

  function schliessen() {
    aufbewahren();
    laufend.abort();
    offenerName = null;
    rollZeit = null;
    fokusZeit = null;
    geladen = false;
    puffer = [];
    nurGezaehlte = [];
    fenster = null;
    bloeckeKnoten = [];
    gescheitert = false;
    ladenBeendet();
    inhalt.replaceChildren();
    lageMerken();
  }

  let eigeneKennung = '';
  return {
    element: rahmen,
    oeffnen,
    erneutVersuchen,
    selbstversuchendSetzen: (auskunft) => { selbstversuchend = auskunft; },
    hinweisNachziehen,
    schliessen,
    heuteSetzen,
    nachrichtAnnehmen,
    fassungAnnehmen,
    zitatAnnehmen,
    anhaengeAnnehmen,
    fassungenAuffrischen,
    nachgeholtAnnehmen,
    vorlaeufigeZeigen,
    nachRueckgaengigFokussieren: (schluessel) => { fokusAufErneut = schluessel; },
    zumEndeFuerSenden,
    kennungSetzen: (kennung) => { eigeneKennung = kennung; },
    eigeneLetzteFokussieren,
    originalSichtbarHalten,
    // Woran das Nachholen ansetzt (ADR-0037 Regel 7, 7g): der offene Channel, ob sein Verlauf steht oder das Laden
    // gescheitert ist, die jüngste bekannte Zeit und ob der neueste Block im Fenster ist.
    stand: () => ({
      channel: offenerName, geladen, gescheitert, juengste, neuesterImFenster: fenster === null || !fenster.neuer,
    }),
  };
}

// „Noch keine Nachrichten in # name.“ bzw. „… mit <Kennung>.“ und darunter, wie Agenten hier schreiben (ADR-0038
// Regel 9, R00050 V9) — der Pfad lesbar, nicht prozentkodiert.
function leerHinweis(name) {
  const gegenueber = gegenueberIn(name);
  let leer = `Noch keine Nachrichten in # ${name}.`;
  let pfad = apiPfad(name);
  if (gegenueber !== null) {
    leer = `Noch keine Nachrichten mit ${gegenueber}.`;
    pfad = `/api/direkt/${gegenueber}`;
  }
  const api = element('p', { className: 'lc-mitte-hinweis lc-leer-api' }, [
    'Agenten schreiben hier mit ', element('code', { textContent: `POST ${pfad}/nachrichten` }), '.',
  ]);
  return [hinweis(leer, 'lc-mitte-hinweis'), api];
}

// Eine eigene Nachricht unterwegs („wird gesendet …“ mit ◌) oder fehlgeschlagen (⚠ und Grund in kd-danger), mit dem
// Kopf der eigenen Kennung, ohne Zeit (Baustein Nachricht); eine Antwort mit dem Zitat ihres Bezugs (R00047 B0715).
function vorlaeufigZeichnen(vorlaeufig, kennung) {
  if (vorlaeufig.zustand === 'verworfen') {
    return verworfenZeichnen(vorlaeufig);
  }
  const artikel = nachrichtZeichnen({ von: kennung, art: 'mensch', text: vorlaeufig.text, antwort_auf: vorlaeufig.bezug });
  artikel.classList.add('lc-vorlaeufig');
  const zeile = element('div', { className: 'lc-send' });
  const inhalt = artikel.querySelector('.lc-msg-inhalt');
  if (vorlaeufig.zustand !== 'fehlgeschlagen') {
    zeile.append(element('span', { textContent: '◌', ariaHidden: 'true' }), ' wird gesendet …');
    inhalt.append(zeile);
    return artikel;
  }
  artikel.classList.add('lc-fehlgeschlagen');
  artikel.dataset.schluessel = vorlaeufig.schluessel;
  zeile.classList.add('lc-fehlgeschlagen');
  zeile.textContent = `⚠ ${vorlaeufig.grund}`;
  inhalt.append(zeile, element('div', { className: 'lc-send-schalter' }, [
    schalterZeichnen('Erneut senden', 'erneut'),
    schalterZeichnen('Verwerfen', 'verwerfen'),
  ]));
  return artikel;
}

// Eine verworfene Nachricht (R00025 F0085): 5 s lang an ihrer Stelle „Verworfen“ mit „Rückgängig“, ohne den Text und
// ohne eigene Region, nicht höher als die Nachricht.
function verworfenZeichnen(vorlaeufig) {
  const zeile = element('div', { className: 'lc-vorlaeufig lc-verworfen' }, [
    element('span', { className: 'lc-verworfen-text', textContent: 'Verworfen' }),
    element('div', { className: 'lc-send-schalter' }, [schalterZeichnen('Rückgängig', 'rueckgaengig')]),
  ]);
  zeile.dataset.schluessel = vorlaeufig.schluessel;
  return zeile;
}

function schalterZeichnen(beschriftung, aktion) {
  const schalter = element('button', { type: 'button', textContent: beschriftung });
  schalter.dataset.aktion = aktion;
  return schalter;
}

// Wie weit der Tagestrenner, der über `nachricht` klebt, vom oberen Rand des Verlaufs nach unten reicht (px): sein
// Abstand oben (top im CSS) und seine Höhe; ohne Trenner davor 0.
function klebenderTrennerUnterkante(nachricht) {
  let davor = nachricht.previousElementSibling;
  while (davor !== null && !davor.classList.contains('lc-day')) {
    davor = davor.previousElementSibling;
  }
  if (davor === null) {
    return 0;
  }
  return parseFloat(getComputedStyle(davor).top) + davor.getBoundingClientRect().height;
}

function nachrichtDavor(nachricht) {
  let davor = nachricht.previousElementSibling;
  while (davor !== null && !davor.classList.contains('lc-msg')) {
    davor = davor.previousElementSibling;
  }
  return davor;
}

// Höhe eines gezeichneten Blocks: von der Oberkante seines ersten bis zur Unterkante seines letzten Knotens.
function blockHoehe(knoten) {
  if (knoten === undefined || knoten.length === 0) {
    return 0;
  }
  return knoten[knoten.length - 1].getBoundingClientRect().bottom - knoten[0].getBoundingClientRect().top;
}

function cursorFuer(fenster, richtung) {
  if (richtung === 'aelter') {
    return { vor: fenster.bloecke[0][0].zeit };
  }
  const letzte = fenster.bloecke[fenster.bloecke.length - 1];
  return { nach: letzte[letzte.length - 1].zeit };
}

// Anker (ADR-0026 Regel 3): die oberste sichtbare Nachricht und ihr Abstand zum oberen Rand vor einer Änderung;
// danach wird scrollTop um ihre Verschiebung korrigiert. Gesucht wird binär über die Nachrichten im DOM.
function ankerMerken(log, inhalt) {
  const nachrichten = inhalt.querySelectorAll('.lc-msg');
  const rand = log.getBoundingClientRect().top;
  let unten = 0;
  let oben = nachrichten.length;
  while (unten < oben) {
    const mitte = Math.floor((unten + oben) / 2);
    if (nachrichten[mitte].getBoundingClientRect().bottom > rand) {
      oben = mitte;
    } else {
      unten = mitte + 1;
    }
  }
  if (unten === nachrichten.length) {
    return null;
  }
  return ankerVon(nachrichten[unten]);
}

function ankerVon(nachricht) {
  return { nachricht, top: nachricht.getBoundingClientRect().top };
}

// Wurde die Nachricht des Ankers neu gezeichnet, gilt ihre neue Gestalt mit derselben Zeit.
function ankerHalten(log, anker) {
  if (anker === null) {
    return;
  }
  let nachricht = anker.nachricht;
  if (!nachricht.isConnected) {
    nachricht = log.querySelector(`.lc-msg[data-zeit="${nachricht.dataset.zeit}"]`);
  }
  if (nachricht === null) {
    return;
  }
  log.scrollTop += nachricht.getBoundingClientRect().top - anker.top;
}

function nurNachrichten(knoten) {
  return knoten.filter((einzeln) => einzeln.classList.contains('lc-msg'));
}

// Nachrichten nach der vorigen (oder null) gegliedert: Tagestrenner vor dem ersten des Tages (benannt nach `heute`),
// Kopf nur bei neuer Gruppe (ADR-0026 Regel 4) und an der ersten gezählten neuen (Zeit oder null); Aktionen nach der
// eigenen `kennung`.
function gegliedertZeichnen(nachrichten, vorige, { heute, kennung }, ersteNeue = null) {
  const gliederungen = folgeGliedern(vorige, nachrichten, ersteNeue);
  const knoten = [];
  nachrichten.forEach((nachricht, stelle) => {
    const { kopf, tagestrenner } = gliederungen[stelle];
    if (tagestrenner !== null) {
      knoten.push(trennerZeichnen(tagestrenner, heute));
    }
    const artikel = nachrichtZeichnen(nachricht, kennung);
    kopfSetzen(artikel, kopf);
    knoten.push(artikel);
  });
  return knoten;
}

function hinweis(text, klasse) {
  return element('p', { className: klasse, textContent: text });
}

function element(tag, eigenschaften = {}, kinder = []) {
  const neu = Object.assign(document.createElement(tag), eigenschaften);
  neu.append(...kinder);
  return neu;
}
