// Unit-Tests des Nachhol-Laufs (R00012 F0047 „Neuversuch“, „Cursor“, „Trennung während des Wartens“, „Ein Lauf
// zugleich“; Nacharbeit #22 B0393; ADR-0037 Ergänzung Karte #22/#36, 7b–7e; node --test) mit gestellter Zeit, einem
// falschen Port statt der API und einem Ersatz für den Verlauf.
import test from 'node:test';
import assert from 'node:assert/strict';
import { nachholLaufAnlegen } from '../../web/nachhollauf.js';
import { zusammenfuehren } from '../../web/live-verlauf.js';
import { wiederVerbundenText } from '../../web/verbindungsleiste.js';

const t = (sekunde) => `2026-10-28T15:00:${String(sekunde).padStart(2, '0')}.000+01:00`;
const n = (sekunde) => ({ zeit: t(sekunde), von: 'beta@127.0.0.1', art: 'agent', text: `n${sekunde}` });
const zeiten = (nachrichten) => nachrichten.map((nachricht) => nachricht.zeit);
const naechsteRunde = () => new Promise((weiter) => setImmediate(weiter));

// Zeitgeber mit gestellter Uhr: laufenBis(ms) springt von Fälligkeit zu Fälligkeit und lässt dazwischen die
// angestoßene Arbeit laufen — ein Neuversuch wird so ab dem Ende des gescheiterten Abrufs gestellt.
function gestellteUhr() {
  let jetzt = 0;
  let wartende = [];
  const uhr = {
    jetzt: () => jetzt,
    zeitgeber: {
      stellen: (ms, rueckruf) => {
        const eintrag = { faellig: jetzt + ms, rueckruf };
        wartende.push(eintrag);
        return eintrag;
      },
      loeschen: (eintrag) => { wartende = wartende.filter((wartend) => wartend !== eintrag); },
    },
    laufenBis: async (ziel) => {
      await naechsteRunde();
      while (wartende.some((wartend) => wartend.faellig <= ziel)) {
        const naechster = wartende.reduce((frueh, wartend) => (wartend.faellig < frueh.faellig ? wartend : frueh));
        jetzt = naechster.faellig;
        wartende = wartende.filter((wartend) => wartend !== naechster);
        naechster.rueckruf();
        await naechsteRunde();
      }
      jetzt = ziel;
      await naechsteRunde();
    },
  };
  return uhr;
}

// Falscher Port: beantwortet die Abrufe der Reihe nach aus `antworten` — 'fehler' (500), 'haengt' (bis zum Abbruch),
// ein Block { nachrichten, neuer } oder ein Promise darauf; ohne weitere Antwort scheitert er. Merkt sich Zeit, Cursor
// und Signal.
function falscherPort(uhr, antworten) {
  const abrufe = [];
  const abrufen = async (channel, cursor, signal, anzahl) => {
    const geplant = antworten[abrufe.length];
    abrufe.push({ um: uhr.jetzt(), channel, nach: cursor.nach, signal, anzahl });
    const antwort = await geplant;
    if (antwort === 'haengt') {
      return new Promise((fertig) => signal.addEventListener('abort', () => fertig({ art: 'abgebrochen' })));
    }
    if (antwort === undefined || antwort === 'fehler') {
      return { art: 'fehler', status: 500 };
    }
    return { art: 'block', block: { channel, nachrichten: antwort.nachrichten, aelter: true, neuer: antwort.neuer } };
  };
  return { abrufen, abrufe, zeitpunkte: () => abrufe.map((abruf) => abruf.um) };
}

// Verlauf mit einer Folge nach Zeit (zusammengeführt wie im Tab); live() nimmt eine Nachricht des Stroms an.
function verlaufErsatz(bekannt) {
  const verlauf = {
    channel: 'allgemein',
    folge: bekannt,
    neuGeoeffnet: [],
    stand: () => ({
      channel: verlauf.channel, geladen: true, juengste: verlauf.folge[verlauf.folge.length - 1].zeit, neuesterImFenster: true,
    }),
    annehmen: (nachrichten) => {
      const ergebnis = zusammenfuehren(verlauf.folge, nachrichten);
      verlauf.folge = ergebnis.folge;
      return ergebnis.neue.length;
    },
    live: (nachricht) => verlauf.annehmen([nachricht]),
  };
  return verlauf;
}

function laufAnlegen({ antworten, geladen = true }) {
  const uhr = gestellteUhr();
  const port = falscherPort(uhr, antworten);
  const verlauf = verlaufErsatz([n(1), n(2), n(3)]);
  const fertig = [];
  const lauf = nachholLaufAnlegen({
    stand: () => ({ ...verlauf.stand(), geladen }),
    abrufen: port.abrufen,
    annehmen: verlauf.annehmen,
    neuOeffnen: (channel) => verlauf.neuGeoeffnet.push(channel),
    fertig: (ergebnis, mitLeiste) => fertig.push({ um: uhr.jetzt(), text: wiederVerbundenText(ergebnis), mitLeiste }),
    zeitgeber: uhr.zeitgeber,
  });
  return { uhr, port, verlauf, fertig, lauf };
}

test('Neuversuch: scheitert das Nachholen immer → Versuche bei 0, 1 000, 3 000, 8 000, 18 000, 28 000 ms, keine Meldung', async () => {
  const { uhr, port, fertig, lauf } = laufAnlegen({ antworten: [] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(28000);

  assert.deepEqual(port.zeitpunkte(), [0, 1000, 3000, 8000, 18000, 28000]);
  assert.deepEqual(fertig, []);
});

test('Neuversuch: gelingt der Versuch bei 8 000 ms mit 3 Nachrichten → „✓ Wieder verbunden · 3 nachgeladen“, Warteschlange weiter', async () => {
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({
    antworten: ['fehler', 'fehler', 'fehler', { nachrichten: [n(4), n(5), n(6)], neuer: false }],
  });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(30000);

  assert.deepEqual(port.zeitpunkte(), [0, 1000, 3000, 8000]);
  assert.deepEqual(fertig, [{ um: 8000, text: '✓ Wieder verbunden · 3 nachgeladen', mitLeiste: true }]);
  assert.deepEqual(zeiten(verlauf.folge), [1, 2, 3, 4, 5, 6].map(t));
  assert.deepEqual(port.abrufe.map((abruf) => abruf.anzahl), [200, 200, 200, 200]);
});

// „Cursor“ (ADR-0037, erster Test, wörtlich): „bekannt 1–3, Trennung; live 6 angenommen; erster Abruf scheitert → der
// zweite fragt `nach=3`, nicht `nach=6`; Block 4, 5 → Folge 1–6 je einmal, Meldung „· 2 nachgeladen“. Mutante „Cursor =
// jüngste gezeigte Zeit“ → rot.“
test('Cursor: nach live 6 und gescheitertem ersten Abruf fragt der zweite nach=3, die Folge wird 1–6', async () => {
  let ersterAbrufScheitert = null;
  const ersterAbruf = new Promise((fertig) => { ersterAbrufScheitert = () => fertig('fehler'); });
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({ antworten: [ersterAbruf, { nachrichten: [n(4), n(5)], neuer: false }] });

  lauf.getrennt();
  lauf.wiederverbunden();
  verlauf.live(n(6));
  ersterAbrufScheitert();
  await uhr.laufenBis(5000);

  assert.deepEqual(port.abrufe.map((abruf) => abruf.nach), [t(3), t(3)]);
  assert.deepEqual(zeiten(verlauf.folge), [1, 2, 3, 4, 5, 6].map(t));
  assert.deepEqual(fertig, [{ um: 1000, text: '✓ Wieder verbunden · 2 nachgeladen', mitLeiste: true }]);
});

test('gelungene Blöcke eines später gescheiterten Versuchs bleiben im Verlauf, der nächste fragt ab ihrem Ende', async () => {
  const erster = Array.from({ length: 200 }, (_, stelle) => ({ ...n(4), zeit: `2026-10-28T15:01:00.${String(stelle).padStart(3, '0')}+01:00` }));
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({
    antworten: [{ nachrichten: erster, neuer: true }, 'fehler', { nachrichten: [], neuer: false }],
  });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(500);
  const nachDemFehlschlag = verlauf.folge.length;
  await uhr.laufenBis(2000);

  assert.equal(nachDemFehlschlag, 203);
  assert.deepEqual(port.abrufe.map((abruf) => abruf.nach), [t(3), erster[199].zeit, erster[199].zeit]);
  assert.deepEqual(port.zeitpunkte(), [0, 0, 1000]);
  assert.deepEqual(fertig, [{ um: 1000, text: '✓ Wieder verbunden · 200 nachgeladen', mitLeiste: true }]);
});

// „Trennung während des Wartens“ (ADR-0037, erster Test, wörtlich): „Fehlschlag bei 0 ms, Trennung bei 500 ms,
// Wiederverbinden bei 700 ms → kein Versuch bei 1 000 ms aus dem alten Lauf, ein Versuch bei 700 ms ab demselben
// Cursor. Mutante „alter Zeitgeber läuft weiter“ → rot.“
test('Trennung während des Wartens: kein Versuch bei 1 000 ms aus dem alten Lauf, einer bei 700 ms ab demselben Cursor', async () => {
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({ antworten: ['fehler', { nachrichten: [n(4)], neuer: false }] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(500);
  lauf.getrennt();
  verlauf.live(n(6));
  await uhr.laufenBis(700);
  lauf.wiederverbunden();
  await uhr.laufenBis(5000);

  assert.deepEqual(port.zeitpunkte(), [0, 700]);
  assert.deepEqual(port.abrufe.map((abruf) => abruf.nach), [t(3), t(3)]);
  assert.deepEqual(fertig, [{ um: 700, text: '✓ Wieder verbunden · 1 nachgeladen', mitLeiste: true }]);
});

test('eine Trennung bricht den laufenden Abruf ab; ohne Wiederverbinden kein Versuch und keine Meldung', async () => {
  const { uhr, port, fertig, lauf } = laufAnlegen({ antworten: ['haengt'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(500);
  lauf.getrennt();
  await uhr.laufenBis(60000);

  assert.equal(port.abrufe[0].signal.aborted, true);
  assert.deepEqual(port.zeitpunkte(), [0]);
  assert.deepEqual(fertig, []);
});

// „Ein Lauf zugleich“ (R00012 F0047; ADR-0037 7e).
test('NeuerStand während eines wartenden Laufs: kein zweiter Abruf, der Neuversuch bleibt bei 1 000 ms', async () => {
  const { uhr, port, fertig, lauf } = laufAnlegen({ antworten: ['fehler', { nachrichten: [], neuer: false }] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(300);
  lauf.neuerStand();
  await uhr.laufenBis(2000);

  assert.deepEqual(port.zeitpunkte(), [0, 1000]);
  assert.deepEqual(fertig, [{ um: 1000, text: '✓ Wieder verbunden', mitLeiste: true }]);
});

test('Channelwechsel während des Wartens: kein Abruf bei 1 000 ms, „✓ Wieder verbunden“, Warteschlange weiter', async () => {
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({ antworten: ['fehler'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(500);
  verlauf.channel = 'technik';
  lauf.channelGewechselt();
  await uhr.laufenBis(30000);

  assert.deepEqual(port.zeitpunkte(), [0]);
  assert.deepEqual(fertig, [{ um: 500, text: '✓ Wieder verbunden', mitLeiste: true }]);
});

test('Channelwechsel während eines laufenden Abrufs bricht ihn ab', async () => {
  const { uhr, port, fertig, lauf } = laufAnlegen({ antworten: ['haengt'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(200);
  lauf.channelGewechselt();
  await uhr.laufenBis(30000);

  assert.equal(port.abrufe[0].signal.aborted, true);
  assert.deepEqual(port.zeitpunkte(), [0]);
  assert.deepEqual(fertig, [{ um: 200, text: '✓ Wieder verbunden', mitLeiste: true }]);
});

test('NeuerStand bei verbundenem Strom: ein Lauf ohne Leiste, mit Neuversuch nach einem Fehlschlag', async () => {
  const { uhr, port, verlauf, fertig, lauf } = laufAnlegen({ antworten: ['fehler', { nachrichten: [n(4), n(5)], neuer: false }] });

  lauf.neuerStand();
  await uhr.laufenBis(2000);

  assert.deepEqual(port.zeitpunkte(), [0, 1000]);
  assert.deepEqual(port.abrufe.map((abruf) => abruf.nach), [t(3), t(3)]);
  assert.deepEqual(zeiten(verlauf.folge), [1, 2, 3, 4, 5].map(t));
  assert.deepEqual(fertig, [{ um: 1000, text: '✓ Wieder verbunden · 2 nachgeladen', mitLeiste: false }]);
});

test('zu viel verpasst (immer neuer) → neu öffnen und „· neueste geladen“, nichts nachgeholt eingefügt', async () => {
  const voll = (block) => ({ nachrichten: [{ ...n(4), zeit: `2026-10-28T15:0${block + 1}:00.000+01:00` }], neuer: true });
  const { uhr, verlauf, fertig, lauf } = laufAnlegen({ antworten: [0, 1, 2, 3, 4].map(voll) });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(100);

  assert.deepEqual(verlauf.neuGeoeffnet, ['allgemein']);
  assert.deepEqual(zeiten(verlauf.folge), [1, 2, 3].map(t));
  assert.deepEqual(fertig, [{ um: 0, text: '✓ Wieder verbunden · neueste geladen', mitLeiste: true }]);
});

test('ohne offenen Channel kein Abruf, sofort „✓ Wieder verbunden“', async () => {
  const uhr = gestellteUhr();
  const port = falscherPort(uhr, []);
  const geladen = falschesLaden(uhr, []);
  const fertig = [];
  const lauf = nachholLaufAnlegen({
    stand: () => ({ channel: null, geladen: false, gescheitert: false, juengste: null, neuesterImFenster: true }),
    abrufen: port.abrufen,
    laden: geladen.laden,
    annehmen: () => 0,
    neuOeffnen: () => {},
    fertig: (ergebnis, mitLeiste) => fertig.push({ um: uhr.jetzt(), text: wiederVerbundenText(ergebnis), mitLeiste }),
    zeitgeber: uhr.zeitgeber,
  });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(100);

  assert.deepEqual(port.abrufe, []);
  assert.deepEqual(geladen.versuche, []);
  assert.deepEqual(fertig, [{ um: 0, text: '✓ Wieder verbunden', mitLeiste: true }]);
});

// R00025 F0084 „Geltender Zeitpunkt“ (B0397): scheitert ein Versuch, meldet der Lauf den Zeitpunkt des nächsten — Ende
// des gescheiterten Abrufs plus Rückzug; ein gelungener Lauf meldet keinen mehr.
test('gescheitert meldet den Zeitpunkt des nächsten Nachhol-Versuchs', async () => {
  const uhr = gestellteUhr();
  const port = falscherPort(uhr, ['fehler', 'fehler', { nachrichten: [n(4)], neuer: false }]);
  const verlauf = verlaufErsatz([n(1), n(2), n(3)]);
  const naechste = [];
  const lauf = nachholLaufAnlegen({
    stand: verlauf.stand,
    abrufen: port.abrufen,
    annehmen: verlauf.annehmen,
    neuOeffnen: () => {},
    fertig: () => naechste.push('fertig'),
    zeitgeber: uhr.zeitgeber,
    jetzt: uhr.jetzt,
    gescheitert: (naechsterVersuchUm) => naechste.push(naechsterVersuchUm),
  });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(10000);

  assert.deepEqual(naechste, [1000, 3000, 'fertig']);
});

// R00025 F0084 „Nur Nachgeholtes gezählt“ (B0490; UX-Vorprüfung #36 V9, wörtlich: „Lücke 3; während des Abrufs 2 live,
// eine davon auch im Block → ‚· 3 nachgeladen‘. n zählt nur Nachrichten, die erst durch das Nachholen kamen; Mutante
// ‚live mitgezählt‘ rot.“) Rechenbeispiel: bekannt 1–3; Lücke 4, 5, 6; während des Abrufs live 7 und 8; der Block bringt
// 4, 5, 6 und 7 → Folge 1–8 je einmal, „· 3 nachgeladen“.
test('Nur Nachgeholtes gezählt: live während des Abrufs Angenommenes zählt nicht, auch wenn es im Block steht', async () => {
  let blockLiefern = null;
  const block = new Promise((liefern) => { blockLiefern = liefern; });
  const { uhr, verlauf, fertig, lauf } = laufAnlegen({ antworten: [block] });

  lauf.getrennt();
  lauf.wiederverbunden();
  verlauf.live(n(7));
  verlauf.live(n(8));
  blockLiefern({ nachrichten: [n(4), n(5), n(6), n(7)], neuer: false });
  await uhr.laufenBis(1000);

  assert.deepEqual(zeiten(verlauf.folge), [1, 2, 3, 4, 5, 6, 7, 8].map(t));
  assert.deepEqual(fertig, [{ um: 0, text: '✓ Wieder verbunden · 3 nachgeladen', mitLeiste: true }]);
});

// Laden-Lauf (R00035, ADR-0037 Ergänzung „Karte #61“, 7g–7i; „Erste Tests“ wörtlich): ist der offene Channel bei der
// Rückkehr nie geladen, öffnet der Lauf ihn neu — sofort, mit Zeitgrenze 10 s und Rückzug.

// Falsches Laden: beantwortet die Versuche der Reihe nach aus `antworten` — 'geladen', 'fehler', 'abgebrochen',
// 'haengt' (bis das Signal abbricht, dann 'fehler' wie verlauf.oeffnen) oder ein Promise darauf; ohne weitere Antwort
// scheitert es. Merkt sich Zeit, Channel und Signal.
function falschesLaden(uhr, antworten) {
  const versuche = [];
  const laden = async (channel, signal) => {
    const geplant = antworten[versuche.length];
    versuche.push({ um: uhr.jetzt(), channel, signal });
    const antwort = await geplant;
    if (antwort === 'haengt') {
      return new Promise((fertig) => signal.addEventListener('abort', () => fertig({ art: 'fehler' })));
    }
    if (antwort === undefined) {
      return { art: 'fehler' };
    }
    return { art: antwort };
  };
  return { laden, versuche, zeitpunkte: () => versuche.map((versuch) => versuch.um) };
}

function ladenLaufAnlegen({ antworten, gescheitert = true }) {
  const uhr = gestellteUhr();
  const port = falscherPort(uhr, []);
  const geladen = falschesLaden(uhr, antworten);
  const verlauf = { stand: { channel: 'a', geladen: false, gescheitert, juengste: null, neuesterImFenster: true } };
  const fertig = [];
  const naechste = [];
  const lauf = nachholLaufAnlegen({
    stand: () => verlauf.stand,
    abrufen: port.abrufen,
    laden: geladen.laden,
    annehmen: () => 0,
    neuOeffnen: () => {},
    fertig: (ergebnis, mitLeiste) => fertig.push({ um: uhr.jetzt(), ergebnis, mitLeiste }),
    zeitgeber: uhr.zeitgeber,
    jetzt: uhr.jetzt,
    gescheitert: (naechsterVersuchUm) => naechste.push(naechsterVersuchUm),
  });
  return { uhr, port, geladen, verlauf, fertig, naechste, lauf };
}

// Mutante „`wiederverbunden` meldet ohne Laden fertig“ (der Stand vor #61) → rot.
test('Auslöser gescheitert: wiederverbunden lädt sofort, nach dem Fehlschlag bei 1 000 erneut, dann genau einmal fertig', async () => {
  let ersterVersuch = null;
  const erster = new Promise((liefern) => { ersterVersuch = liefern; });
  const { uhr, port, geladen, fertig, naechste, lauf } = ladenLaufAnlegen({ antworten: [erster, 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  const sofort = { versuche: geladen.versuche.map((versuch) => [versuch.um, versuch.channel]), fertig: [...fertig] };
  ersterVersuch('fehler');
  await uhr.laufenBis(999);
  const vorDemZweiten = geladen.zeitpunkte();
  await uhr.laufenBis(30000);

  assert.deepEqual(sofort, { versuche: [[0, 'a']], fertig: [] });
  assert.deepEqual(port.abrufe, []);
  assert.deepEqual(naechste, [1000]);
  assert.deepEqual(vorDemZweiten, [0]);
  assert.deepEqual(geladen.zeitpunkte(), [0, 1000]);
  assert.deepEqual(fertig, [{ um: 1000, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
});

// Mutante „Auslöser nur bei `gescheitert`“ → rot.
test('Auslöser unterwegs: erster Abruf noch nicht zurück (nicht geladen, nicht gescheitert) → wiederverbunden lädt sofort', async () => {
  const { uhr, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['geladen'], gescheitert: false });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(100);

  assert.deepEqual(geladen.zeitpunkte(), [0]);
  assert.deepEqual(fertig, [{ um: 0, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
});

// Mutante „abgebrochen zählt als Fehlschlag“ → rot.
test('abgebrochen beendet: kein Zeitgeber, kein fertig, nach 10 s kein weiterer Versuch', async () => {
  const { uhr, geladen, fertig, naechste, lauf } = ladenLaufAnlegen({ antworten: ['abgebrochen', 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(10000);

  assert.deepEqual(geladen.zeitpunkte(), [0]);
  assert.deepEqual(fertig, []);
  assert.deepEqual(naechste, []);
});

test('Zeitgrenze 10 s: ein hängender Versuch wird abgebrochen, zählt als Fehlschlag, der nächste folgt 1 s danach', async () => {
  const { uhr, geladen, fertig, naechste, lauf } = ladenLaufAnlegen({ antworten: ['haengt', 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(9999);
  const vorDerGrenze = geladen.versuche[0].signal.aborted;
  await uhr.laufenBis(20000);

  assert.equal(vorDerGrenze, false);
  assert.equal(geladen.versuche[0].signal.aborted, true);
  assert.deepEqual(naechste, [11000]);
  assert.deepEqual(geladen.zeitpunkte(), [0, 11000]);
  assert.deepEqual(fertig, [{ um: 11000, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
});

test('Rückzug und Channelwechsel: Fehlschlag bei 0, Wechsel bei 500 → kein Versuch bei 1 000', async () => {
  const { uhr, geladen, verlauf, fertig, lauf } = ladenLaufAnlegen({ antworten: ['fehler', 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(500);
  verlauf.stand = { ...verlauf.stand, channel: 'b' };
  lauf.channelGewechselt();
  await uhr.laufenBis(30000);

  assert.deepEqual(geladen.zeitpunkte(), [0]);
  assert.deepEqual(fertig, [{ um: 500, ergebnis: { art: 'nachgeholt', zahl: 0 }, mitLeiste: true }]);
});

test('jetztVersuchen bei 300 → Versuch bei 300, keiner bei 1 000', async () => {
  const { uhr, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['fehler', 'fehler', 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(300);
  const laedtVorher = lauf.laedt();
  lauf.jetztVersuchen();
  await uhr.laufenBis(1999);

  assert.equal(laedtVorher, true);
  assert.deepEqual(geladen.zeitpunkte(), [0, 300]);
  await uhr.laufenBis(30000);
  assert.deepEqual(geladen.zeitpunkte(), [0, 300, 2300]);
  assert.deepEqual(fertig, [{ um: 2300, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
  assert.equal(lauf.laedt(), false);
});

test('jetztVersuchen während eines laufenden Versuchs öffnet kein zweites Mal', async () => {
  let versuchLiefern = null;
  const erster = new Promise((liefern) => { versuchLiefern = liefern; });
  const { uhr, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: [erster] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(200);
  lauf.jetztVersuchen();
  versuchLiefern('geladen');
  await uhr.laufenBis(5000);

  assert.deepEqual(geladen.zeitpunkte(), [0]);
  assert.deepEqual(fertig, [{ um: 200, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
});

test('ohne Laden-Lauf: laedt ist falsch, jetztVersuchen tut nichts', async () => {
  const { uhr, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['geladen'] });

  const laedt = lauf.laedt();
  lauf.jetztVersuchen();
  await uhr.laufenBis(5000);

  assert.equal(laedt, false);
  assert.deepEqual(geladen.versuche, []);
  assert.deepEqual(fertig, []);
});

test('Trennung beendet den Laden-Lauf; nach dem nächsten Wiederverbinden lädt er erneut, ohne Cursor', async () => {
  const { uhr, port, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['haengt', 'geladen'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(400);
  lauf.getrennt();
  await uhr.laufenBis(3000);
  const nachDerTrennung = geladen.zeitpunkte();
  lauf.wiederverbunden();
  await uhr.laufenBis(5000);

  assert.equal(geladen.versuche[0].signal.aborted, true);
  assert.deepEqual(nachDerTrennung, [0]);
  assert.deepEqual(geladen.zeitpunkte(), [0, 3000]);
  assert.deepEqual(port.abrufe, []);
  assert.deepEqual(fertig, [{ um: 3000, ergebnis: { art: 'geladen' }, mitLeiste: true }]);
});

test('NeuerStand bei gescheitert: Laden-Lauf, fertig ohne Leiste', async () => {
  const { uhr, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['fehler', 'geladen'] });

  lauf.neuerStand();
  await uhr.laufenBis(2000);

  assert.deepEqual(geladen.zeitpunkte(), [0, 1000]);
  assert.deepEqual(fertig, [{ um: 1000, ergebnis: { art: 'geladen' }, mitLeiste: false }]);
});

test('NeuerStand bei nicht geladen und nicht gescheitert (erster Abruf unterwegs): nichts', async () => {
  const { uhr, port, geladen, fertig, lauf } = ladenLaufAnlegen({ antworten: ['geladen'], gescheitert: false });

  lauf.neuerStand();
  await uhr.laufenBis(5000);

  assert.deepEqual(geladen.versuche, []);
  assert.deepEqual(port.abrufe, []);
  assert.deepEqual(fertig, []);
});

// „`getrennt` merkt sich für ihn keinen [Cursor]“ (7g): wird der Channel während der Trennung doch geladen (neue Wahl
// bei verbundenem HTTP), holt das Wiederverbinden ab seiner jüngsten Zeit nach, nicht ab einem leeren Cursor.
test('Trennung während eines Laden-Laufs merkt keinen Cursor: später geladen, holt das Wiederverbinden ab der jüngsten Zeit nach', async () => {
  const { uhr, port, verlauf, lauf } = ladenLaufAnlegen({ antworten: ['haengt'] });

  lauf.getrennt();
  lauf.wiederverbunden();
  await uhr.laufenBis(200);
  lauf.getrennt();
  verlauf.stand = { channel: 'a', geladen: true, gescheitert: false, juengste: t(3), neuesterImFenster: true };
  lauf.wiederverbunden();
  await uhr.laufenBis(300);

  assert.deepEqual(port.abrufe.map((abruf) => abruf.nach), [t(3)]);
});
