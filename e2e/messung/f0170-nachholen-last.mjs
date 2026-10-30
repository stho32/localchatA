'use strict';
// Lastprobe F0170 „Nachholen bleibt begrenzt“ (R00049, B0759; ADR-0054 Ergänzung „Nachholen von warten …“,
// Abnahme-Probe des security-reviewer und Z1–Z5) und F0171 „Nachholen zählt nach Aufwand“ (B0763; ADR-0054 Ergänzung
// „Kosten-Einheit des Nachholens“: Z5 neu, Z6, Z6b, Z2 auf Kosten, Z4 reihum). Messwert im Entwicklertest, kein Gate;
// Release-Build, nicht nachts.
//
//   node e2e/messung/f0170-nachholen-last.mjs <localchatA> <verlauf-erzeugen> <arbeitsordner> [szenario …]
//
// Szenarien: kontrolle, verlauf200, probe, z1, z5 (2.000 Sätze zu 16 KiB), z5-32k (2.000 Sätze zu 32 KiB aus
// Zeilen), z5-bearbeitet (2.000 kurze Sätze, jeder auf 16 KiB bearbeitet), z5-geloescht (2.000 Sätze zu 16 KiB, alle
// bis auf den letzten gelöscht), z5-dicht (2.000 Sätze „@a “ × 5.461), z6 (64 Herkünfte), z6b (6 Herkünfte × 16
// Sitzungen, Retry-After beachtet), k3, k4 (ohne Angabe alle). Die Z5-Reihen nennen dazu µs je gebuchter
// Kosteneinheit (Nacharbeit security-reviewer #17 Befund 1, Probe 4: Median ≤ ≈ 8 µs).
// Je Szenario eine frische Instanz auf einem freien Port mit 100.000 Nachrichten in 10 Channels. Die 64 Schleifen laufen in einem eigenen Worker, damit ihre
// Last auf dem Client die Messung des Beobachters nicht verfälscht. Beobachter und Sender kommen von 127.0.0.3,
// weil eine Gegenstelle höchstens 64 Verbindungen hält und die 64 Schleifen 127.0.0.1 ausschöpfen.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { fileURLToPath } from 'node:url';
import { kennwerte, wartezeitInMs } from './last-auswertung.mjs';
import { anfragen, beitreten, gesundheit, starten, warten } from './last-instanz.mjs';

const ALTES_SEIT = '2000-01-01T00:00:00.000Z';
const NACHHOLEN = `api/warten?bis=0&seit=${ALTES_SEIT}&erwaehnung`;
const VERLAUF_50 = 'api/channels/allgemein/nachrichten?n=50';
const VERLAUF_200 = 'api/channels/allgemein/nachrichten?n=200';
const Z5_LAEUFE = 10;

if (isMainThread) {
  await hauptprogramm(process.argv.slice(2));
} else {
  parentPort.postMessage(await schleifenFahren(workerData));
}

async function hauptprogramm([programm, erzeuger, arbeitsordner, ...gewaehlt]) {
  const alle = {
    kontrolle, verlauf200, probe, z1, z5, 'z5-32k': z5Zeilen, 'z5-bearbeitet': z5Bearbeitet, 'z5-geloescht': z5Geloescht, 'z5-dicht': z5Dicht, z6, z6b, k3, k4,
  };
  let namen = Object.keys(alle);
  if (gewaehlt.length > 0) {
    namen = gewaehlt;
  }
  for (const name of namen) {
    const umgebung = { programm, erzeuger, ordner: path.join(arbeitsordner, name) };
    const ergebnis = await alle[name](umgebung);
    console.log(JSON.stringify({ szenario: name, ...ergebnis }));
  }
}

// Kontrolle ohne Schleifen: Verlauf-Median unter 5 ms.
async function kontrolle(umgebung) {
  return mitInstanz(umgebung, async (instanz) => {
    const beobachter = await beitreten(instanz.adresse, 'beobachter', '127.0.0.3');
    return { verlauf: await beobachten(instanz.adresse, beobachter) };
  });
}

// Gleiche Last als Verlauf-Schleife n=200: Verlauf p90 unter 100 ms.
async function verlauf200(umgebung) {
  return unterLast(umgebung, { pfad: VERLAUF_200, retryAfterBeachten: true });
}

// Die Abnahme-Probe wörtlich (Schleifen beachten Retry-After) mit Z2, Z3 und Z4 im selben Lauf.
async function probe(umgebung) {
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: true });
}

// Z6 (F0171): 64 Herkünfte (127.0.2.1–127.0.2.64) mit je 1 Sitzung und je 4 Schleifen = 256 gleichzeitige
// Nachhol-Aufrufe, Retry-After ignoriert, 8 s; der Beobachter von 127.0.0.3 misst den Verlauf.
async function z6(umgebung) {
  const herkuenfte = Array.from({ length: 64 }, (_, index) => `127.0.2.${index + 1}`);
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: false, herkuenfte, sitzungenJeHerkunft: 1, dauerInMs: 8_000, ohneZustellung: true });
}

// Z6b (F0171, Variante des security-reviewer): 6 Herkünfte × 16 Sitzungen × 4 Schleifen = 384 Verbindungen,
// Retry-After beachtet, 8 s.
async function z6b(umgebung) {
  const herkuenfte = Array.from({ length: 6 }, (_, index) => `127.0.1.${index + 1}`);
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: true, herkuenfte, sitzungenJeHerkunft: 16, dauerInMs: 8_000, ohneZustellung: true });
}

// K3 (Integration mit laufender Uhr, F0170/F0171): schwere Aufrufe einer Kennung ohne Pause — die ersten 50 sind 200,
// spätestens beim 80. kommt 429 `nachholen` von `kennung` mit wieder_in_ms in 1…100.
async function k3(umgebung) {
  return mitInstanz(umgebung, async (instanz) => {
    const agent = await beitreten(instanz.adresse, 'claude', '127.0.0.1');
    const folge = await schwereAufrufe(instanz.adresse, [{ token: agent, herkunft: '127.0.0.1' }], 100);
    return { erstes_429: folge.erstes429, von: folge.von, wieder_in_ms: folge.wiederInMs, ok_davor: folge.okDavor };
  });
}

// K4 (Integration mit laufender Uhr): 16 Kennungen von 127.0.0.1 rufen reihum schwer ohne Pause — kein 429 von
// `kennung`, spätestens beim 600. Aufruf 429 `nachholen` von `herkunft`.
async function k4(umgebung) {
  return mitInstanz(umgebung, async (instanz) => {
    const sitzungen = [];
    for (let nummer = 0; nummer < 16; nummer += 1) {
      sitzungen.push({ token: await beitreten(instanz.adresse, `k${nummer}`, '127.0.0.1'), herkunft: '127.0.0.1' });
    }
    const folge = await schwereAufrufe(instanz.adresse, sitzungen, 700);
    return { erstes_429: folge.erstes429, von: folge.von, wieder_in_ms: folge.wiederInMs, ok_davor: folge.okDavor };
  });
}

// Ruft reihum schwer (altes seit, erwaehnung, kein Treffer) bis zum ersten 429 oder `hoechstens` Aufrufen.
async function schwereAufrufe(adresse, sitzungen, hoechstens) {
  for (let nummer = 1; nummer <= hoechstens; nummer += 1) {
    const { token, herkunft } = sitzungen[(nummer - 1) % sitzungen.length];
    const antwort = await anfragen(adresse, { pfad: NACHHOLEN, token, herkunft });
    if (antwort.status === 429) {
      return { erstes429: nummer, von: antwort.daten.von, wiederInMs: antwort.daten.wieder_in_ms, okDavor: nummer - 1 };
    }
  }
  return { erstes429: null, okDavor: hoechstens };
}

// Z1: die Schleifen ignorieren Retry-After und rufen sofort wieder.
async function z1(umgebung) {
  return unterLast(umgebung, { pfad: NACHHOLEN, retryAfterBeachten: false });
}

// Z5 neu (F0171): 2.000 Sätze zu je 16 KiB, die Erwähnung im letzten (Namenssuche gebaut) — ≈ 18 Aufrufe mit dem
// jeweils zurückgegebenen seit bis zum Treffer; je Aufruf der Median über zehn Läufe ≤ 20 ms, die Folge lückenlos
// ohne Doppel, der letzte Aufruf liefert den Treffer.
async function z5(umgebung) {
  return z5Reihe(umgebung, grosseSaetzeSchreiben);
}

// Dieselbe Probe mit 2.000 Sätzen zu je 32 KiB, jeder Satz aus vielen kurzen Zeilen.
async function z5Zeilen(umgebung) {
  return z5Reihe(umgebung, saetzeAusZeilenSchreiben);
}

// Nacharbeit security-reviewer #17 Befund 1: 2.000 kurze Sätze, jeder auf 16 KiB bearbeitet (die Bearbeitung des
// letzten erwähnt claude) — kostet wie z5, je Einheit gleich schnell.
async function z5Bearbeitet(umgebung) {
  return z5Reihe(umgebung, bearbeiteteSaetzeSchreiben);
}

// Nacharbeit security-reviewer #17 Befund 1: 2.000 Sätze zu 16 KiB, alle bis auf den letzten gelöscht — gelöschte
// kosten nach dem Original, das gelesen wird.
async function z5Geloescht(umgebung) {
  return z5Reihe(umgebung, geloeschteSaetzeSchreiben);
}

// Nacharbeit security-reviewer #17 Befund 1 und 2: 2.000 Sätze „@a “ × 5.461 (16 KiB erwähnungsdicht).
async function z5Dicht(umgebung) {
  return z5Reihe(umgebung, (ordner, erzeuger) => saetzeSchreiben(ordner, erzeuger, `  ${'@a '.repeat(5_461)}`));
}

async function z5Reihe(umgebung, vorbereiten) {
  return mitInstanz({ ...umgebung, vorbereiten }, async (instanz) => {
    const agent = await beitreten(instanz.adresse, 'claude', '127.0.0.1');
    const laeufe = [];
    for (let lauf = 0; lauf < Z5_LAEUFE; lauf += 1) {
      laeufe.push(await reiheBisTreffer(instanz.adresse, agent));
    }
    const stand = await gesundheit(instanz.adresse);
    const aufrufe = Math.max(...laeufe.map((eintrag) => eintrag.dauern.length));
    const medianeJeAufruf = Array.from({ length: aufrufe }, (_, index) => kennwerte(laeufe.map((eintrag) => eintrag.dauern[index])).median);
    const jeEinheit = kennwerte(laeufe.flatMap((eintrag) => eintrag.usJeEinheit));
    return {
      us_je_kosteneinheit: { median: jeEinheit.median, p90: jeEinheit.p90, max: jeEinheit.max },
      kosten_je_lauf: [...new Set(laeufe.map((eintrag) => eintrag.kosten))],
      aufrufe_je_lauf: [...new Set(laeufe.map((eintrag) => eintrag.dauern.length))],
      median_je_aufruf_ms: medianeJeAufruf,
      groesster_median_ms: Math.max(...medianeJeAufruf),
      lueckenlos_ohne_doppel: laeufe.every((eintrag) => eintrag.lueckenlos),
      letzter_liefert_treffer: laeufe.every((eintrag) => eintrag.treffer === 1 && eintrag.letztesEnde === 'treffer'),
      bremsen_429: laeufe.reduce((summe, eintrag) => summe + eintrag.gebremst, 0),
      nachholen: stand.nachholen,
      nachrichten: stand.nachrichten,
      defekte_zeilen: stand.defekte_zeilen,
    };
  });
}

// Ein Lauf: Aufrufe mit dem jeweils zurückgegebenen seit, bis ein anderes Ende als `weitere` kommt (429: Retry-After
// abwarten und denselben Aufruf wiederholen, nicht gemessen). Lückenlos: jedes seit liegt hinter dem vorigen. Je
// Aufruf die gebuchten Kosten aus der Gesundheit davor und danach und daraus µs je Kosteneinheit (ab 100 Kosten, damit
// der feste Anteil eines Aufrufs das Maß nicht verzerrt).
async function reiheBisTreffer(adresse, token) {
  let seit = ALTES_SEIT;
  const dauern = [];
  const usJeEinheit = [];
  let kosten = 0;
  let treffer = 0;
  let gebremst = 0;
  let lueckenlos = true;
  let letztesEnde;
  while (letztesEnde === undefined || letztesEnde === 'weitere') {
    const kostenVorher = (await gesundheit(adresse)).nachholen.kosten;
    const antwort = await anfragen(adresse, { pfad: `api/warten?bis=0&erwaehnung&seit=${encodeURIComponent(seit)}`, token });
    if (antwort.status === 429) {
      gebremst += 1;
      await warten(wartezeitInMs(antwort.retryAfter));
      continue;
    }
    if (antwort.status !== 200) {
      throw new Error(`warten: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
    }
    dauern.push(antwort.dauerInMs);
    const gebucht = (await gesundheit(adresse)).nachholen.kosten - kostenVorher;
    kosten += gebucht;
    if (gebucht >= 100) {
      usJeEinheit.push((antwort.dauerInMs * 1000) / gebucht);
    }
    treffer += antwort.daten.nachrichten.length;
    lueckenlos = lueckenlos && Date.parse(antwort.daten.seit) > Date.parse(seit);
    seit = antwort.daten.seit;
    letztesEnde = antwort.daten.ende;
  }
  return { dauern, usJeEinheit, kosten, treffer, gebremst, lueckenlos, letztesEnde };
}

// Last aus `herkuenfte` × `sitzungenJeHerkunft` Sitzungen mit je 4 Schleifen (Worker); daneben Beobachter (Verlauf),
// 8 Sender reihum (Z4) und — außer bei `ohneZustellung` — der Agent von 127.0.0.2 (Z3). Z2 ist die Gesundheit nach 8 s.
async function unterLast(umgebung, { pfad, retryAfterBeachten, herkuenfte = ['127.0.0.1'], sitzungenJeHerkunft = 16, dauerInMs = 20_000, ohneZustellung = false }) {
  return mitInstanz(umgebung, async (instanz) => {
    const sitzungen = [];
    for (const [stelle, herkunft] of herkuenfte.entries()) {
      for (let nummer = 0; nummer < sitzungenJeHerkunft; nummer += 1) {
        sitzungen.push({ token: await beitreten(instanz.adresse, `schleife${stelle}x${nummer}`, herkunft), herkunft });
      }
    }
    const beobachter = await beitreten(instanz.adresse, 'beobachter', '127.0.0.3');
    const sender = [];
    for (let nummer = 0; nummer < 8; nummer += 1) {
      sender.push(await beitreten(instanz.adresse, `sender${nummer}`, '127.0.0.3'));
    }
    const fernagent = await beitreten(instanz.adresse, 'fernagent', '127.0.0.2');
    const fernsender = await beitreten(instanz.adresse, 'fernsender', '127.0.0.2');
    const ende = Date.now() + dauerInMs;
    const schleifen = schleifenStarten({ adresse: instanz.adresse, sitzungen, pfad, retryAfterBeachten, ende });

    const zustellen = ohneZustellung ? Promise.resolve(null) : zwanzigMalZustellen(instanz.adresse, fernagent, fernsender, ende);
    const [verlauf, senden, zustellung, gesundheitNach8s] = await Promise.all([
      beobachten(instanz.adresse, beobachter),
      vierzigMalSenden(instanz.adresse, sender),
      zustellen,
      warten(8_000).then(() => gesundheit(instanz.adresse)),
    ]);
    return {
      verlauf,
      z2: gesundheitNach8s.nachholen,
      z3: zustellung,
      z4: senden,
      schleifen: await schleifen,
    };
  });
}

// Beobachter: 40 × Verlauf n=50 im Abstand von 50 ms.
async function beobachten(adresse, token) {
  const dauern = [];
  for (let nummer = 0; nummer < 40; nummer += 1) {
    const antwort = await anfragen(adresse, { pfad: VERLAUF_50, token, herkunft: '127.0.0.3' });
    dauern.push(antwort.dauerInMs);
    await warten(50);
  }
  return kennwerte(dauern);
}

// Z4: 40 Sendungen im Abstand von 200 ms, reihum von 8 Kennungen (je 5, weit unter dem Vorrat 30 des Schreibkontos,
// A00014); alle antworten 201 — gezählt je Status, gemessen bis zur 201.
async function vierzigMalSenden(adresse, tokens) {
  const dauern = [];
  const status = {};
  for (let nummer = 0; nummer < 40; nummer += 1) {
    const antwort = await anfragen(adresse, {
      methode: 'POST', pfad: 'api/channels/allgemein/nachrichten', token: tokens[nummer % tokens.length], herkunft: '127.0.0.3', koerper: { text: `z4-${nummer}` },
    });
    status[antwort.status] = (status[antwort.status] || 0) + 1;
    if (antwort.status === 201) {
      dauern.push(antwort.dauerInMs);
    }
    await warten(200);
  }
  return { ...kennwerte(dauern), status };
}

// Z3: ein Agent von 127.0.0.2 wartet in Schleife mit laufendem seit; ein Sender von dort sendet 20 Nachrichten in 20 s.
// Der Agent holt sich vor dem ersten Senden einen Stand, sonst verpasst „ab jetzt“ eine Nachricht, die vor seinem
// ersten Aufruf ankommt. Gemessen von der 201 bis zur Ankunft (kommt die Nachricht vor der 201 an, zählt 0). Der Agent
// wartet mit `erwaehnung` (B0742), jede Nachricht erwähnt ihn — so verlangt es Z3.
async function zwanzigMalZustellen(adresse, agent, sender, ende) {
  const zeit201 = new Map();
  const ankunft = new Map();
  const aufrufe = {};
  const anfang = await anfragen(adresse, { pfad: 'api/warten?bis=0&erwaehnung', token: agent, herkunft: '127.0.0.2' });
  const empfangen = (async () => {
    let seit = anfang.daten.seit;
    while (ankunft.size < 20 && Date.now() < ende + 5_000) {
      let seitTeil = '';
      if (seit) {
        seitTeil = `&seit=${encodeURIComponent(seit)}`;
      }
      const antwort = await anfragen(adresse, { pfad: `api/warten?bis=5&erwaehnung${seitTeil}`, token: agent, herkunft: '127.0.0.2' });
      const art = `${antwort.status} ${antwort.daten.ende || antwort.daten.grenze}`;
      aufrufe[art] = (aufrufe[art] || 0) + 1;
      if (antwort.status !== 200) {
        await warten(wartezeitInMs(antwort.retryAfter));
        continue;
      }
      seit = antwort.daten.seit;
      for (const eintrag of antwort.daten.nachrichten) {
        ankunft.set(eintrag.nachricht.text, antwort.empfangen);
      }
    }
  })();
  for (let nummer = 0; nummer < 20; nummer += 1) {
    const text = `@fernagent z3-${nummer}`;
    const antwort = await anfragen(adresse, {
      methode: 'POST', pfad: 'api/channels/z3/nachrichten', token: sender, herkunft: '127.0.0.2', koerper: { text },
    });
    if (antwort.status === 201) {
      zeit201.set(text, antwort.empfangen);
    }
    await warten(1_000);
  }
  await empfangen;
  const dauern = [...zeit201].map(([text, gesendet]) => zustelldauer(gesendet, ankunft.get(text)));
  const fehlend = [...zeit201.keys()].filter((text) => !ankunft.has(text));
  return { ...kennwerte(dauern), gesendet: zeit201.size, angekommen: ankunft.size, fehlend, aufrufe };
}

function zustelldauer(gesendet, angekommen) {
  if (angekommen === undefined) {
    return Infinity;
  }
  return Math.max(0, angekommen - gesendet);
}

function schleifenStarten(auftrag) {
  return new Promise((erledigt, gescheitert) => {
    const worker = new Worker(fileURLToPath(import.meta.url), { workerData: auftrag });
    worker.once('message', erledigt);
    worker.once('error', gescheitert);
  });
}

// Im Worker: je Sitzung 4 Schleifen, jede sequentiell mit eigener Verbindung von der Herkunft der Sitzung, bis `ende`.
async function schleifenFahren({ adresse, sitzungen, pfad, retryAfterBeachten, ende }) {
  const zaehler = { ok: 0, nachholen429: 0, warten429: 0, andere: 0 };
  const schleife = async ({ token, herkunft }) => {
    const agent = new http.Agent({ keepAlive: true, maxSockets: 1 });
    while (Date.now() < ende) {
      const antwort = await anfragen(adresse, { pfad, token, herkunft, agent });
      zaehlen(zaehler, antwort);
      if (antwort.status === 429 && retryAfterBeachten) {
        await warten(Math.min(wartezeitInMs(antwort.retryAfter), ende - Date.now()));
      }
    }
    agent.destroy();
  };
  await Promise.all(sitzungen.flatMap((token) => [schleife(token), schleife(token), schleife(token), schleife(token)]));
  return zaehler;
}

function zaehlen(zaehler, antwort) {
  if (antwort.status === 200) {
    zaehler.ok += 1;
  } else if (antwort.status === 429 && antwort.daten.grenze === 'nachholen') {
    zaehler.nachholen429 += 1;
  } else if (antwort.status === 429) {
    zaehler.warten429 += 1;
  } else {
    zaehler.andere += 1;
  }
}

async function mitInstanz({ programm, erzeuger, ordner, vorbereiten = hunderttausendNachrichten }, messen) {
  fs.rmSync(ordner, { recursive: true, force: true });
  fs.mkdirSync(path.join(ordner, 'channels', 'z3'), { recursive: true });
  vorbereiten(ordner, erzeuger);
  const instanz = await starten(programm, ordner);
  try {
    return await messen(instanz);
  } finally {
    await beenden(instanz.prozess);
    fs.rmSync(ordner, { recursive: true, force: true });
  }
}

// 100.000 Nachrichten in 10 Channels (allgemein, c1 … c9), je 10.000 über einen Tag.
function hunderttausendNachrichten(ordner, erzeuger) {
  const channels = ['allgemein', ...Array.from({ length: 9 }, (_, index) => `c${index + 1}`)];
  for (const channel of channels) {
    execFileSync(erzeuger, [ordner, channel, '10000', '1'], { stdio: 'ignore' });
  }
}

// Z5 (16 KiB): 2.000 Sätze, je mit 16.384 Byte ASCII-Text in einer Zeile; der letzte erwähnt claude.
function grosseSaetzeSchreiben(ordner, erzeuger) {
  const text = 'Befund Scan laeuft '.repeat(1024).slice(0, 16 * 1024);
  saetzeSchreiben(ordner, erzeuger, `  ${text}`);
}

// Z5 (32 KiB): jeder Satz aus Zeilen zu 31 Zeichen (Zeilenumbrüche im Text, je Zeile zwei Leerzeichen Einrückung),
// zusammen ≈ 32 KiB; der letzte erwähnt claude.
function saetzeAusZeilenSchreiben(ordner, erzeuger) {
  const zeile = `  ${'Befund Scan laeuft Schritt 0123'.slice(0, 31)}`;
  const zeilen = Math.floor((32 * 1024) / (zeile.length + 1));
  saetzeSchreiben(ordner, erzeuger, Array.from({ length: zeilen }, () => zeile).join('\n'));
}

// Schreibt 2.000 Sätze: die Kopfzeilen kommen vom Erzeuger, der Text (eingerückt) ist `rumpf`; im letzten Satz steht
// `@claude ` vor dem ersten Wort.
function saetzeSchreiben(ordner, erzeuger, rumpf) {
  const { kopfzeilen, schreiben } = kopfzeilenErzeugen(ordner, erzeuger, 2_000);
  schreiben(kopfzeilen.map((kopf, index) => `${kopf}\n${mitErwaehnungImLetzten(rumpf, index, kopfzeilen.length)}\n\n`));
}

// Nacharbeit #17 Befund 1: 2.000 Sätze „x“, danach je ein Bearbeitungssatz auf 16 KiB (später, gleicher Absender).
function bearbeiteteSaetzeSchreiben(ordner, erzeuger) {
  const rumpf = `  ${'Befund Scan laeuft '.repeat(1024).slice(0, 16 * 1024)}`;
  const { kopfzeilen, schreiben } = kopfzeilenErzeugen(ordner, erzeuger, 4_000);
  const originale = kopfzeilen.slice(0, 2_000);
  const originalsaetze = originale.map((kopf) => `${kopf}\n  x\n\n`);
  const bearbeitungen = originale.map((kopf, index) => {
    const text = mitErwaehnungImLetzten(rumpf, index, originale.length);
    return `${aenderungskopf(kopfzeilen[2_000 + index], kopf)}\n+ bearbeitet: ${kopfteile(kopf).zeit}\n${text}\n\n`;
  });
  schreiben([...originalsaetze, ...bearbeitungen]);
}

// Nacharbeit #17 Befund 1: 2.000 Sätze zu 16 KiB, danach je ein Löschsatz außer für den letzten (der erwähnt claude).
function geloeschteSaetzeSchreiben(ordner, erzeuger) {
  const rumpf = `  ${'Befund Scan laeuft '.repeat(1024).slice(0, 16 * 1024)}`;
  const { kopfzeilen, schreiben } = kopfzeilenErzeugen(ordner, erzeuger, 4_000);
  const originale = kopfzeilen.slice(0, 2_000);
  const originalsaetze = originale.map((kopf, index) => `${kopf}\n${mitErwaehnungImLetzten(rumpf, index, originale.length)}\n\n`);
  const loeschungen = originale.slice(0, -1)
    .map((kopf, index) => `${aenderungskopf(kopfzeilen[2_000 + index], kopf)}\n+ geloescht: ${kopfteile(kopf).zeit}\n\n`);
  schreiben([...originalsaetze, ...loeschungen]);
}

// `anzahl` Kopfzeilen vom Erzeuger (ein Tag, aufsteigend) und ein Schreiber für die Sätze in dieselbe Tagesdatei.
function kopfzeilenErzeugen(ordner, erzeuger, anzahl) {
  const muster = path.join(ordner, 'muster');
  execFileSync(erzeuger, [muster, 'allgemein', String(anzahl), '1'], { stdio: 'ignore' });
  const tagesdatei = fs.readdirSync(path.join(muster, 'channels', 'allgemein'))[0];
  const kopfzeilen = fs.readFileSync(path.join(muster, 'channels', 'allgemein', tagesdatei), 'utf8')
    .split('\n')
    .filter((zeile) => zeile.startsWith('@ '));
  fs.rmSync(muster, { recursive: true, force: true });
  const schreiben = (saetze) => {
    fs.mkdirSync(path.join(ordner, 'channels', 'allgemein'), { recursive: true });
    fs.writeFileSync(path.join(ordner, 'channels', 'allgemein', tagesdatei), saetze.join(''));
  };
  return { kopfzeilen, schreiben };
}

function mitErwaehnungImLetzten(rumpf, index, anzahl) {
  if (index === anzahl - 1) {
    return `  @claude ${rumpf.slice(2)}`;
  }
  return rumpf;
}

// Kopf eines Änderungssatzes: Zeit aus `spaeter`, Absender des Originals (nur der Absender darf ändern).
function aenderungskopf(spaeter, original) {
  return `@ ${kopfteile(spaeter).zeit} ${kopfteile(original).von}`;
}

function kopfteile(kopf) {
  const [, zeit, von] = kopf.split(' ');
  return { zeit, von };
}

// SIGTERM und warten, bis der Prozess weg ist — sonst schreibt er noch ins Protokoll, während der Ordner gelöscht wird.
function beenden(prozess) {
  return new Promise((erledigt) => {
    if (prozess.exitCode !== null) {
      erledigt();
      return;
    }
    prozess.once('exit', erledigt);
    prozess.kill('SIGTERM');
  });
}
