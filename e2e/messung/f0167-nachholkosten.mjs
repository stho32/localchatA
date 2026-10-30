'use strict';
// Messung F0167 „Nachholkosten“ (R00049 US-13; ADR-0054 QW6 zweiter Teil; B0744). Messwert im Entwicklertest, kein
// Gate; Release-Build, nicht nachts.
//
//   node e2e/messung/f0167-nachholkosten.mjs <localchatA> <verlauf-erzeugen> <arbeitsordner>
//
// Eine frische Instanz auf einem freien Port mit 100.000 Nachrichten ohne Erwähnung in # allgemein; claude ruft
// 50 × warten?bis=0&erwaehnung mit dem jeweils zurückgegebenen seit (erwartet: 49 × weitere, dann frist), während ein
// offener Ereignisstrom weiter liefert; danach ein zweiter Lauf ohne Pause, den das Nachholkonto bremst (429).
//
// Die Dauer eines Aufrufs ist am Client gemessen (Loopback, Ende der Antwort) — eine obere Schranke der Server-Zeit.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { kennwerte, wartezeitInMs } from './last-auswertung.mjs';
import { anfragen, beitreten, gesundheit, starten, stromOeffnen, warten } from './last-instanz.mjs';

const VOR_ALLEM = '2000-01-01T00:00:00.000Z';

const [programm, erzeuger, arbeitsordner] = process.argv.slice(2);
const ergebnis = await nachholkosten({ programm, erzeuger, ordner: path.join(arbeitsordner, 'nachholkosten') });
console.log(JSON.stringify({ szenario: 'nachholkosten', ...ergebnis }));

// B0744 (US-13): 49 × weitere, der fünfzigste frist; jeder Aufruf < 100 ms; der Strom liefert in < 1 s.
async function nachholkosten({ programm, erzeuger, ordner }) {
  const vorbereiten = () => execFileSync(erzeuger, [ordner, 'allgemein', '100000', '1'], { stdio: 'ignore' });
  return mitInstanz({ programm, ordner, vorbereiten }, async (instanz) => {
    const claude = await beitreten(instanz.adresse, 'claude', '127.0.0.1');
    const anna = await beitreten(instanz.adresse, 'anna', '127.0.0.3', 'mensch');
    const leser = await beitreten(instanz.adresse, 'leser', '127.0.0.3', 'mensch');
    const strom = stromOeffnen(instanz.adresse, leser, '127.0.0.3');
    await strom.offen;
    const vorher = await gesundheit(instanz.adresse);

    let laeuft = true;
    const imStrom = aktivitaetImStrom(instanz.adresse, anna, strom, () => laeuft);
    const erster = await fuenfzigMalNachholen(instanz.adresse, claude);
    const nachErstem = await gesundheit(instanz.adresse);
    const zweiter = await fuenfzigMalNachholen(instanz.adresse, claude);
    laeuft = false;
    const strombefund = await imStrom;
    strom.schliessen();

    const letzte = letzteZeit(path.join(ordner, 'channels', 'allgemein'));
    return {
      letzte_nachricht: letzte,
      erster_seit_ab_letzter: Date.parse(erster.bericht.letztes_seit) >= Date.parse(letzte),
      erster: erster.bericht,
      zweiter_ohne_pause: zweiter.bericht,
      strom: strombefund,
      nachholen_saetze_erster_lauf: nachErstem.nachholen.saetze - vorher.nachholen.saetze,
      nachholen: nachErstem.nachholen,
    };
  });
}

// Die Zeit der letzten Kopfzeile `@ <zeit> <kennung>` der erzeugten Tagesdateien.
function letzteZeit(channelordner) {
  const zeiten = fs.readdirSync(channelordner)
    .filter((datei) => datei.endsWith('.log'))
    .flatMap((datei) => fs.readFileSync(path.join(channelordner, datei), 'utf8').split('\n'))
    .filter((zeile) => zeile.startsWith('@ '))
    .map((zeile) => zeile.split(' ')[1]);
  return zeiten.sort((a, b) => Date.parse(a) - Date.parse(b))[zeiten.length - 1];
}

// Ruft 50 × warten?bis=0&erwaehnung mit dem jeweils zurückgegebenen seit (429 nachholen: Retry-After abwarten und
// denselben Aufruf wiederholen); liefert Folge der Enden, Dauern der 200 und Zahl der 429.
async function fuenfzigMalNachholen(adresse, token) {
  let seit = VOR_ALLEM;
  const enden = [];
  const dauern = [];
  const gebremst = { anzahl: 0, retry_after: [] };
  let treffer = 0;
  while (enden.length < 50) {
    const antwort = await anfragen(adresse, { pfad: `api/warten?bis=0&erwaehnung&seit=${encodeURIComponent(seit)}`, token });
    if (antwort.status === 429) {
      gebremst.anzahl += 1;
      gebremst.retry_after.push(antwort.retryAfter);
      await warten(wartezeitInMs(antwort.retryAfter));
      continue;
    }
    if (antwort.status !== 200) {
      throw new Error(`warten: ${antwort.status} ${JSON.stringify(antwort.daten)}`);
    }
    enden.push(antwort.daten.ende);
    dauern.push(antwort.dauerInMs);
    treffer += antwort.daten.nachrichten.length;
    seit = antwort.daten.seit;
  }
  const weitere = enden.filter((ende) => ende === 'weitere').length;
  return {
    bericht: {
      aufrufe: enden.length,
      weitere,
      letztes_ende: enden[enden.length - 1],
      erste_49_weitere: enden.slice(0, 49).every((ende) => ende === 'weitere'),
      treffer,
      letztes_seit: seit,
      dauer_ms: kennwerte(dauern),
      gebremst: { anzahl: gebremst.anzahl, retry_after: [...new Set(gebremst.retry_after)] },
    },
  };
}

// Während das Nachholen läuft: anna setzt alle 250 ms abwechselnd `tippt` und `denkt_nach` in # technik — ein
// Ereignis, das keine Nachricht ist und das Nachholen nicht verlängert; gemessen vom Absenden bis zur Ankunft.
async function aktivitaetImStrom(adresse, anna, strom, laeuft) {
  const dauern = [];
  const status = {};
  let nummer = 0;
  while (laeuft()) {
    const zustand = ['tippt', 'denkt_nach'][nummer % 2];
    nummer += 1;
    const ab = strom.stand();
    const beginn = performance.now();
    const antwort = await anfragen(adresse, {
      methode: 'PUT', pfad: 'api/channels/technik/aktivitaet', token: anna, herkunft: '127.0.0.3', koerper: { zustand },
    });
    status[antwort.status] = (status[antwort.status] || 0) + 1;
    if (antwort.status < 300) {
      const angekommen = await strom.naechstes(ab, (ereignis) => ereignis.art === 'aktivitaet' && ereignis.zustand === zustand, 5_000);
      dauern.push(angekommen - beginn);
    }
    await warten(250);
  }
  return { ...kennwerte(dauern), status };
}

async function mitInstanz({ programm, ordner, vorbereiten }, messen) {
  fs.rmSync(ordner, { recursive: true, force: true });
  for (const channel of ['allgemein', 'technik']) {
    fs.mkdirSync(path.join(ordner, 'channels', channel), { recursive: true });
  }
  vorbereiten();
  const instanz = await starten(programm, ordner);
  try {
    return await messen(instanz);
  } finally {
    instanz.prozess.kill('SIGTERM');
    fs.rmSync(ordner, { recursive: true, force: true });
  }
}
