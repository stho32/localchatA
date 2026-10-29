// Der Verlauf eines Channels in der Mitte (ADR-0026): Öffnen holt die neuesten 100 mit einem Abruf, zeichnet sie
// älteste oben und steht am Ende; solange der Blick dort steht, hält ein ResizeObserver ihn dort, auch wenn Höhen
// sich nachträglich ändern (Schrift, Fenster). Beim Scrollen lädt das Fenster an beiden Enden je 100 nach und hält
// fünf Blöcke, bis zu acht, solange das andere Ende nah am Blick ist (Abwurfschutz, fenster.js); die oberste
// sichtbare Nachricht bleibt dabei an ihrer Stelle (Anker von Hand, in einem Zug). Ein Wechsel bricht laufende
// Abrufe ab, eine Antwort für einen nicht mehr gewählten Channel wird verworfen, Daten des vorigen Channels fallen
// weg. Der Verlauf ist die einzige scrollende Fläche der Mitte: role="log", aria-live="off", mit der Tastatur
// fokussierbar; Nachladen wird nie angesagt.
import { nachrichtZeichnen, kopfSetzen, trennerZeichnen, langeEinklappen, langeUmschalten } from './nachricht.js';
import { gliederung, folgeGliedern, datumBenennen } from './gliederung.js';
import { nachladen, einfuegen } from './fenster.js';
import { einordnen, zusammenfuehren, buendlerAnlegen, folgenOderZaehlen, pillenText, auslaufNach, nichtGeladenText, hinweisTextNachziehen, sprungZiel as sprungZielBerechnen } from './live-verlauf.js';

// abrufen: async (name, cursor, signal) → { art: 'block' | 'fehler' | 'abgebrochen', … } (api.js verlaufAbrufen);
// das Ergebnis von `oeffnen` sagt, ob der Verlauf steht.
// Live (ADR-0037 Regeln 1, 6): nachrichtAnnehmen(nachricht, eigen) ordnet über die Zeit ein und fügt je Frame ein —
// am Ende folgt der Blick, sonst zählt die Pille „n neue Nachrichten“; vorlaeufigeZeigen(liste) zeichnet eigene
// Nachrichten unterwegs oder fehlgeschlagen unter allen bestätigten; an fehlgeschlagenen rufen „Erneut senden“ und
// „Verwerfen“ schalter('erneut' | 'verwerfen', schluessel) (ADR-0038 Regel 5).
export function verlaufAnlegen({ abrufen, schalter = () => {}, fokusRueckgaengig = () => {}, fokusVerloren = () => {} }) {
  const inhalt = element('div', { className: 'lc-log-inhalt' });
  const ausstehend = element('div', { className: 'lc-ausstehend' });
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

  function abstandUnten() {
    return log.scrollHeight - log.scrollTop - log.clientHeight;
  }

  function ansEnde() {
    log.scrollTop = log.scrollHeight;
    amEnde = true;
  }

  function lageMerken() {
    auslaufAbbauen(auslaufSichtbar(), 0);
    amEnde = abstandUnten() <= 1 && fenster !== null && !fenster.neuer;
    if (amEnde) {
      neueGezaehlt = 0;
      ersteNeue = null;
    }
    const weitOben = abstandUnten() > log.clientHeight;
    pille.hidden = fenster === null || !(weitOben || fenster.neuer || neueGezaehlt > 0);
    pillenBeschriftung.textContent = 'Zum Neuesten';
    if (neueGezaehlt > 0) {
      pillenBeschriftung.textContent = pillenText(neueGezaehlt);
    }
  }

  log.addEventListener('scroll', () => {
    lageMerken();
    pruefen();
  }, { passive: true });
  const hoehen = new ResizeObserver(() => {
    if (amEnde) {
      ansEnde();
    }
    lageMerken();
  });
  hoehen.observe(inhalt);
  hoehen.observe(ausstehend);
  hoehen.observe(log);

  // Aufklappen erweitert an Ort und Stelle; der Kopf der Nachricht bleibt an seiner Bildschirmstelle.
  inhalt.addEventListener('click', (ereignis) => {
    const schalter = ereignis.target.closest('.lc-foldbar button');
    if (schalter === null) {
      return;
    }
    const anker = ankerVon(schalter.closest('.lc-msg'));
    langeUmschalten(schalter);
    ankerHalten(log, anker);
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
    const geklickt = ereignis.target.closest('button[data-aktion]');
    if (geklickt !== null) {
      schalter(geklickt.dataset.aktion, geklickt.closest('[data-schluessel]').dataset.schluessel);
    }
  });
  log.addEventListener('keydown', (ereignis) => {
    const nurEnde = ereignis.key === 'End' && !ereignis.altKey && !ereignis.ctrlKey && !ereignis.metaKey && !ereignis.shiftKey;
    if (nurEnde && offenerName !== null) {
      ereignis.preventDefault();
      zumNeuesten();
    }
  });

  // Öffnet den Channel mit den neuesten 100 (ADR-0037 Ergänzung „Karte #61“, 7h): → { art: 'geladen' | 'fehler' |
  // 'abgebrochen' }. Bricht der Aufrufer über `signal` ab (Zeitgrenze, Ende des Laufs), gilt das als Fehlschlag;
  // übernimmt ein anderes Öffnen oder schließt der Verlauf, ist es abgebrochen. Ein erneutes Öffnen nach „Verlauf nicht
  // geladen“ lässt den Hinweis bis zum Ergebnis stehen (7k).
  async function oeffnen(name, signal = new AbortController().signal) {
    laufend.abort();
    laufend = new AbortController();
    const dieser = laufend;
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
    log.setAttribute('aria-label', `Verlauf # ${name}`);
    if (!hinweisBleibt) {
      inhalt.replaceChildren();
    }
    lageMerken();
    const ergebnis = await abrufen(name, {}, dieser.signal);
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
      return { art: 'geladen' };
    }
    angenommenMerken(ergebnis.block.nachrichten);
    eingeordnetMerken(ergebnis.block.nachrichten);
    fenster = { bloecke: [ergebnis.block.nachrichten], aelter: ergebnis.block.aelter, neuer: false };
    juengste = ergebnis.block.nachrichten[ergebnis.block.nachrichten.length - 1].zeit;
    const knoten = gegliedertZeichnen(ergebnis.block.nachrichten, null, heute);
    bloeckeKnoten = [knoten];
    inhalt.replaceChildren(...knoten);
    langeEinklappen(nurNachrichten(knoten));
    ansEnde();
    lageMerken();
    gepuffertesAnnehmen();
    pruefen();
    return { art: 'geladen' };
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

  // Was vor dem Block ankam, geht jetzt den Weg jeder neuen Nachricht (ADR-0028 Regel 5).
  function gepuffertesAnnehmen() {
    const gepuffert = puffer;
    puffer = [];
    gepuffert.forEach(({ nachricht, eigen }) => nachrichtAnnehmen(nachricht, eigen));
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
    const lage = folgenOderZaehlen(abstandUnten(), zeilenhoehe(), neuesterImFenster);
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
    const knoten = gegliedertZeichnen(folge, vorige, heute, ersteNeue);
    bloeckeKnoten[stelle].forEach((einzeln) => einzeln.remove());
    inhalt.append(...knoten);
    fenster.bloecke[stelle] = folge;
    bloeckeKnoten[stelle] = knoten;
    langeEinklappen(nurNachrichten(knoten));
  }

  // Neue Nachrichten ans Ende des jüngsten Blocks; im leeren Channel entsteht der erste Block. Die erste gezählte neue
  // trägt ihren Kopf (R00020 F0067).
  function anfuegen(neue) {
    if (fenster === null) {
      fenster = { bloecke: [neue], aelter: false, neuer: false };
      const knoten = gegliedertZeichnen(neue, null, heute, ersteNeue);
      bloeckeKnoten = [knoten];
      inhalt.replaceChildren(...knoten);
      langeEinklappen(nurNachrichten(knoten));
      return;
    }
    const letzter = fenster.bloecke[fenster.bloecke.length - 1];
    const knoten = gegliedertZeichnen(neue, letzter[letzter.length - 1], heute, ersteNeue);
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
    ausstehend.replaceChildren(...eigene.map((vorlaeufig) => vorlaeufigZeichnen(vorlaeufig, eigeneKennung)));
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

  // Abstand des Blicks zu den Rändern und Höhe des obersten und untersten Blocks (fenster.js, Abwurfschutz).
  function masse() {
    return {
      abstandOben: log.scrollTop,
      abstandUnten: abstandUnten(),
      hoeheOben: blockHoehe(bloeckeKnoten[0]),
      hoeheUnten: blockHoehe(bloeckeKnoten[bloeckeKnoten.length - 1]),
      sicht: log.clientHeight,
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
      const knoten = gegliedertZeichnen(nachrichten, null, heute);
      inhalt.prepend(...knoten);
      bloeckeKnoten.unshift(knoten);
      grenzeGliedern(nachrichten[nachrichten.length - 1], 1);
      langeEinklappen(nurNachrichten(knoten));
      return;
    }
    const letzte = fenster.bloecke[fenster.bloecke.length - 1];
    const knoten = gegliedertZeichnen(nachrichten, letzte[letzte.length - 1], heute);
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
    laufend.abort();
    offenerName = null;
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
    nachgeholtAnnehmen,
    vorlaeufigeZeigen,
    nachRueckgaengigFokussieren: (schluessel) => { fokusAufErneut = schluessel; },
    zumEndeFuerSenden,
    kennungSetzen: (kennung) => { eigeneKennung = kennung; },
    // Woran das Nachholen ansetzt (ADR-0037 Regel 7, 7g): der offene Channel, ob sein Verlauf steht oder das Laden
    // gescheitert ist, die jüngste bekannte Zeit und ob der neueste Block im Fenster ist.
    stand: () => ({
      channel: offenerName, geladen, gescheitert, juengste, neuesterImFenster: fenster === null || !fenster.neuer,
    }),
  };
}

// „Noch keine Nachrichten in # name.“ und darunter, wie Agenten hier schreiben (ADR-0038 Regel 9).
function leerHinweis(name) {
  const api = element('p', { className: 'lc-mitte-hinweis lc-leer-api' }, [
    'Agenten schreiben hier mit ', element('code', { textContent: `POST /api/channels/${name}/nachrichten` }), '.',
  ]);
  return [hinweis(`Noch keine Nachrichten in # ${name}.`, 'lc-mitte-hinweis'), api];
}

// Eine eigene Nachricht unterwegs („wird gesendet …“ mit ◌) oder fehlgeschlagen (⚠ und Grund in kd-danger), mit dem
// Kopf der eigenen Kennung, ohne Zeit (Baustein Nachricht).
function vorlaeufigZeichnen(vorlaeufig, kennung) {
  if (vorlaeufig.zustand === 'verworfen') {
    return verworfenZeichnen(vorlaeufig);
  }
  const artikel = nachrichtZeichnen({ von: kennung, art: 'mensch', text: vorlaeufig.text });
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

// Nachrichten nach der vorigen (oder null) gegliedert: Tagestrenner vor dem ersten des Tages, Kopf nur bei neuer
// Gruppe (ADR-0026 Regel 4) und an der ersten gezählten neuen (Zeit oder null).
function gegliedertZeichnen(nachrichten, vorige, heute, ersteNeue = null) {
  const gliederungen = folgeGliedern(vorige, nachrichten, ersteNeue);
  const knoten = [];
  nachrichten.forEach((nachricht, stelle) => {
    const { kopf, tagestrenner } = gliederungen[stelle];
    if (tagestrenner !== null) {
      knoten.push(trennerZeichnen(tagestrenner, heute));
    }
    const artikel = nachrichtZeichnen(nachricht);
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
