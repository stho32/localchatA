// Unit-Tests des SSE-Lesers (R00004 F0013, ADR-0015 Regel 6; node --test): Teilstücke aus dem fetch-Strom
// werden zu Ereignissen { event, data } und einem Rest.
import test from 'node:test';
import assert from 'node:assert/strict';
import { zerlegen } from '../../web/sse.js';

function alleStuecke(stuecke) {
  let rest = '';
  const ereignisse = [];
  for (const stueck of stuecke) {
    const ergebnis = zerlegen(rest, stueck);
    ereignisse.push(...ergebnis.ereignisse);
    rest = ergebnis.rest;
  }
  return { ereignisse, rest };
}

test('ein Ereignis über zwei Teilstücke kommt erst mit der Leerzeile, einmal', () => {
  const erstes = zerlegen('', 'data: {"art":"teil');
  const zweites = zerlegen(erstes.rest, 'nehmer"}\n\n');

  assert.deepEqual(erstes.ereignisse, []);
  assert.deepEqual(zweites.ereignisse, [{ event: '', data: '{"art":"teilnehmer"}' }]);
  assert.equal(zweites.rest, '');
});

test('\\r\\n-Zeilenenden, auch zwischen zwei Teilstücken getrennt', () => {
  const { ereignisse, rest } = alleStuecke(['event: herzschlag\r\ndata: {}\r', '\n\r\ndata: 1\r\n\r\n']);

  assert.deepEqual(ereignisse, [{ event: 'herzschlag', data: '{}' }, { event: '', data: '1' }]);
  assert.equal(rest, '');
});

test('Kommentarzeilen werden übergangen, ein Block nur aus Kommentar ergibt kein Ereignis', () => {
  const { ereignisse } = alleStuecke([': x\n\n: ping\ndata: a\n\n']);

  assert.deepEqual(ereignisse, [{ event: '', data: 'a' }]);
});

test('zwei Ereignisse in einem Stück, der angefangene dritte bleibt Rest', () => {
  const { ereignisse, rest } = zerlegen('', 'data: 1\n\ndata: 2\n\ndata: 3');

  assert.deepEqual(ereignisse, [{ event: '', data: '1' }, { event: '', data: '2' }]);
  assert.equal(rest, 'data: 3');
});

test('mehrere data-Zeilen werden mit Zeilenumbruch verbunden, ohne Leerzeichen nach dem Doppelpunkt geht es auch', () => {
  const { ereignisse } = zerlegen('', 'data:a\ndata: b\n\n');

  assert.deepEqual(ereignisse, [{ event: '', data: 'a\nb' }]);
});
