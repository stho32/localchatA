// Unit-Tests des Neu-Lesens nach einer Lücke (R00041 F0134 „lädt die Blöcke seines Fensters neu“, ADR-0048 Regel 11;
// Nacharbeit #68 B1; node --test): gelesen wird das ganze Fenster vom Ende bis zu seinem Anfang — auch acht Blöcke und
// ein live gewachsener jüngster Block —, nicht weiter; Fehlschlag und Wechsel beenden das Lesen still.
import test from 'node:test';
import assert from 'node:assert/strict';
import { fensterNeuLesen } from '../../web/live-verlauf.js';

const BLOCK = 100;
const zeitVon = (nummer) => new Date(Date.UTC(2026, 9, 28, 12, 0, 0) + nummer * 1000).toISOString().replace('Z', '+00:00');

// Ein Channel mit `anzahl` Nachrichten (eine je Sekunde) und ein Abruf wie die API: `vor` exklusiv, höchstens 100,
// `aelter`, wenn davor noch welche liegen. Jeder Cursor wird mitgeschrieben.
function channelMit(anzahl) {
  const nachrichten = Array.from({ length: anzahl }, (_, nummer) => ({ zeit: zeitVon(nummer), text: `n${nummer}` }));
  const cursor = [];
  const abrufen = async ({ vor }) => {
    cursor.push(vor);
    const davor = nachrichten.filter((nachricht) => Date.parse(nachricht.zeit) < Date.parse(vor));
    const block = davor.slice(-BLOCK);
    return { art: 'block', block: { nachrichten: block, aelter: davor.length > block.length, neuer: true } };
  };
  return { nachrichten, cursor, abrufen };
}

async function neuLesen(channel, { von, bis, gilt = () => true }) {
  const uebernommen = new Set();
  await fensterNeuLesen({
    abrufen: channel.abrufen,
    anfang: channel.nachrichten[von].zeit,
    juengste: channel.nachrichten[bis].zeit,
    uebernehmen: (nachricht) => uebernommen.add(nachricht.text),
    gilt,
  });
  return uebernommen;
}

function fehlendeIm(uebernommen, channel, von, bis) {
  return channel.nachrichten.slice(von, bis + 1).map((nachricht) => nachricht.text).filter((text) => !uebernommen.has(text));
}

// Abnahme-Probe B1 (a): Fenster aus acht Blöcken (800), die älteste und die jüngste Nachricht des Fensters.
test('acht Blöcke im Fenster: jede Nachricht vom Anfang bis zum Ende wird gelesen, die älteste eingeschlossen', async () => {
  const channel = channelMit(1000);

  const uebernommen = await neuLesen(channel, { von: 200, bis: 999 });

  assert.deepEqual(fehlendeIm(uebernommen, channel, 200, 999), []);
});

// Abnahme-Probe B1 (b): fünf Blöcke plus live gewachsener jüngster Block (100 + 250 live).
test('jüngster Block live auf 350 gewachsen, dazu vier Blöcke: das ganze Fenster wird gelesen', async () => {
  const channel = channelMit(800);

  const uebernommen = await neuLesen(channel, { von: 50, bis: 799 });

  assert.deepEqual(fehlendeIm(uebernommen, channel, 50, 799), []);
});

// Abnahme-Probe B1 (c): nicht über den Anfang des Fensters hinaus — ein Fenster aus einem Block in einem Channel mit
// 600 liest genau einen Block, eines aus acht Blöcken genau acht.
test('gelesen wird nur bis zum Anfang des Fensters: so viele Abrufe, wie das Fenster Blöcke hat', async () => {
  const einBlock = channelMit(600);
  const achtBloecke = channelMit(1000);

  await neuLesen(einBlock, { von: 500, bis: 599 });
  await neuLesen(achtBloecke, { von: 200, bis: 999 });

  assert.equal(einBlock.cursor.length, 1);
  assert.equal(achtBloecke.cursor.length, 8);
  assert.equal(einBlock.cursor[0], '2026-10-28T12:09:59.001+00:00');
});

// Abnahme-Probe B1 (d): Fehlschlag und Wechsel beenden das Lesen ohne Ausnahme; ein Abruf ohne Fortschritt auch.
test('Fehlschlag, Wechsel des Channels und ein Block ohne Fortschritt beenden das Lesen still', async () => {
  const channel = channelMit(1000);
  let abrufe = 0;
  const fehlschlag = { ...channel, abrufen: async () => { abrufe += 1; return { art: 'fehler', status: 0 }; } };
  let gueltig = true;
  const gewechselt = { ...channel, abrufen: async (cursor) => { gueltig = false; return channel.abrufen(cursor); } };
  const stehend = { ...channel, abrufen: async () => { abrufe += 1; return { art: 'block', block: { nachrichten: channel.nachrichten.slice(-BLOCK), aelter: true } }; } };

  const nachFehlschlag = await neuLesen(fehlschlag, { von: 0, bis: 999 });
  const nachWechsel = await neuLesen(gewechselt, { von: 0, bis: 999, gilt: () => gueltig });
  await neuLesen(stehend, { von: 0, bis: 999 });

  assert.equal(nachFehlschlag.size, 0);
  assert.equal(nachWechsel.size, 0);
  assert.equal(abrufe, 1 + 2);
});
