// Live-Strom (B0091, ADR-0015): fetch auf /api/ereignisse mit dem Sitzungs-Header statt EventSource — das
// Token steht nie in einer URL. Eigener Leser (sse.js) und Wächter (verbindung.js): nach einem Abbruch neu
// verbinden (1 s, 2 s, 5 s, dann alle 10 s), 40 s ohne Byte gilt als Abbruch. Auf 401 meldet der Halter
// „abgelehnt“ und wartet auf ein neues Token. Ohne DOM, damit derselbe Code im SharedWorker läuft.
import { SITZUNGS_HEADER } from './api.js';
import { zerlegen } from './sse.js';
import { entscheiden } from './verbindung.js';

const NICHT_AUTORISIERT = 401;

// Worker oder Rückfall (B0095, ADR-0015 Regel 2 und 5): gibt es SharedWorker, teilen alle Tabs einen Strom im
// Worker; sonst hält dieser Tab ihn selbst. Beide Wege liefern dieselben Ereignisse und Zustände, auf
// 'abgelehnt' mit { beitreten }: ob dieser Tab still beitreten soll (im Worker genau einer).
export function verbinden({ token, version, beiEreignis, beiZustand }) {
  if (typeof globalThis.SharedWorker === 'undefined') {
    return halten({ token, beiEreignis, beiZustand: (zustand) => beiZustand(zustand, { beitreten: true }) });
  }
  const worker = new globalThis.SharedWorker(workerAdresse(version), { type: 'module', name: `localchatA ${version}` });
  const port = worker.port;
  port.onmessage = (nachricht) => {
    const daten = nachricht.data;
    if (daten.art === 'ereignis') {
      beiEreignis(daten.ereignis);
    }
    if (daten.art === 'zustand') {
      beiZustand(daten.zustand, { beitreten: daten.beitreten });
    }
  };
  worker.onerror = (fehler) => console.error(`localchatA: Fehler im Strom-Worker — ${fehler.message}`);
  const gehen = () => port.postMessage({ art: 'tschuess' });
  globalThis.addEventListener('pagehide', gehen);
  port.postMessage({ art: 'token', token });
  return {
    tokenSetzen: (neuesToken) => port.postMessage({ art: 'token', token: neuesToken }),
    beenden: () => {
      globalThis.removeEventListener('pagehide', gehen);
      gehen();
      port.close();
    },
  };
}

// Die Programmversion in der Adresse des Workers: nach einem Update läuft kein alter Worker weiter (ADR-0015).
export function workerAdresse(version) {
  return `/web/strom.js?v=${version}`;
}

// Zustände an beiZustand: 'verbunden' | 'unterbrochen' | 'abgelehnt' (mit dem abgelehnten Token).
export function halten({
  token,
  beiEreignis,
  beiZustand,
  holen = (pfad, optionen) => globalThis.fetch(pfad, optionen),
  schlafen = (ms) => new Promise((weiter) => setTimeout(weiter, ms)),
  jetzt = () => Date.now(),
  pruefAbstandMs = 5000,
}) {
  let aktuellesToken = token;
  let beendet = false;
  let laufenderAbruf = null;
  let neuesTokenMelden = () => {};

  async function laufen() {
    let fehlversuche = 0;
    while (!beendet) {
      await schlafen(entscheiden({ verbunden: false, letzteAktivitaetMs: 0, jetztMs: 0, fehlversuche }).wartenMs);
      if (!aktuellesToken) {
        await aufNeuesTokenWarten();
        fehlversuche = 0;
        continue;
      }
      const ergebnis = await einmalVerbinden(aktuellesToken);
      if (beendet) {
        return;
      }
      if (ergebnis === 'abgelehnt') {
        beiZustand('abgelehnt', aktuellesToken);
        aktuellesToken = null;
        continue;
      }
      if (ergebnis === 'gelesen') {
        fehlversuche = 1;
      } else {
        fehlversuche += 1;
      }
      beiZustand('unterbrochen');
    }
  }

  // 'abgelehnt' (401), 'gescheitert' (kein Strom zustande gekommen) oder 'gelesen' (Strom lief und endete).
  async function einmalVerbinden(tokenFuerDenAbruf) {
    laufenderAbruf = new AbortController();
    let antwort;
    try {
      antwort = await holen('/api/ereignisse', {
        headers: { [SITZUNGS_HEADER]: tokenFuerDenAbruf },
        signal: laufenderAbruf.signal,
        cache: 'no-store',
      });
    } catch (ausnahme) {
      return 'gescheitert';
    }
    if (antwort.status === NICHT_AUTORISIERT) {
      return 'abgelehnt';
    }
    if (!antwort.ok || antwort.body === null) {
      return 'gescheitert';
    }
    beiZustand('verbunden');
    await lesen(antwort.body, laufenderAbruf);
    return 'gelesen';
  }

  async function lesen(body, abruf) {
    const leser = body.pipeThrough(new TextDecoderStream()).getReader();
    let letzteAktivitaetMs = jetzt();
    const waechter = setInterval(() => {
      const entscheidung = entscheiden({ verbunden: true, letzteAktivitaetMs, jetztMs: jetzt(), fehlversuche: 0 });
      if (entscheidung.aktion === 'neu') {
        abruf.abort();
      }
    }, pruefAbstandMs);
    let rest = '';
    try {
      for (;;) {
        const { value: stueck, done: zuEnde } = await leser.read();
        if (zuEnde) {
          return;
        }
        letzteAktivitaetMs = jetzt();
        const zerlegt = zerlegen(rest, stueck);
        rest = zerlegt.rest;
        zerlegt.ereignisse.filter((ereignis) => ereignis.event === '').forEach(weiterreichen);
      }
    } catch (abgebrochen) {
      return;
    } finally {
      clearInterval(waechter);
    }
  }

  // Ein Ereignis, das kein JSON ist, ist ein Fehler des Servers — sichtbar in der Konsole, der Strom läuft weiter.
  function weiterreichen(ereignis) {
    let daten;
    try {
      daten = JSON.parse(ereignis.data);
    } catch (ausnahme) {
      console.warn(`localchatA: Ereignis ist kein JSON — ${ereignis.data.slice(0, 80)}`);
      return;
    }
    beiEreignis(daten);
  }

  function aufNeuesTokenWarten() {
    return new Promise((gesetzt) => {
      neuesTokenMelden = gesetzt;
    });
  }

  laufen();
  return {
    tokenSetzen(neuesToken) {
      const anderesToken = neuesToken !== aktuellesToken;
      aktuellesToken = neuesToken;
      neuesTokenMelden();
      if (anderesToken && laufenderAbruf !== null) {
        laufenderAbruf.abort();
      }
    },
    beenden() {
      beendet = true;
      neuesTokenMelden();
      if (laufenderAbruf !== null) {
        laufenderAbruf.abort();
      }
    },
  };
}
