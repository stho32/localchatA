// Unit-Tests der Gelöscht-Folge im Verlauf ohne Browser (R00030 B0796; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { geloeschtFolgeAnlegen, fokusNameVonGeloeschtem, idsJeNachricht } from '../../web/geloescht-folge.js';

const anh = (id, zustand = 'vorhanden') => ({ id, name: `${id}.png`, bytes: 100, art: 'image/png', zustand });
const nachricht = (zeit, ...anhaenge) => ({ zeit, text: '', anhaenge });

test('Fokusname: nennt den Namen des gerade gelöschten Anhangs mit Fokus, sonst null', () => {
  const n = nachricht('z1', anh('a'), anh('b'), anh('c', 'geloescht'));

  assert.equal(fokusNameVonGeloeschtem(n, new Set(['a', 'b']), (id) => id === 'b'), 'b.png');
  assert.equal(fokusNameVonGeloeschtem(n, new Set(['a']), (id) => id === 'b'), null, 'Fokus in einem Anhang, der bleibt');
  assert.equal(fokusNameVonGeloeschtem(n, new Set(['c']), () => true), null, 'schon gelöscht zählt nicht');
  assert.equal(fokusNameVonGeloeschtem(n, new Set(['a']), () => false), null);
});

test('idsJeNachricht: gruppiert nach Satzzeit der bekannten Nachricht, Unbekanntes fällt weg', () => {
  const bekannt = (zeit) => (zeit === 'z1' || zeit === 'z1b' ? { nachricht: { zeit: 'z1' } } : null);

  const karte = idsJeNachricht([{ id: 'a', zeit: 'z1' }, { id: 'b', zeit: 'z1b' }, { id: 'x', zeit: 'z9' }], bekannt);

  assert.deepEqual([...karte.keys()], ['z1']);
  assert.deepEqual([...karte.get('z1')], ['a', 'b']);
});

test('Vermerk: nachrichtenFiltern zeigt vermerkte Anhänge als gelöscht, ohne Vermerk dieselbe Liste', () => {
  const folge = geloeschtFolgeAnlegen({});
  const liste = [nachricht('z1', anh('a'), anh('b'))];
  assert.equal(folge.nachrichtenFiltern(liste), liste);

  folge.vermerken([{ id: 'a', zeit: 'z1' }]);
  const gefiltert = folge.nachrichtenFiltern(liste);

  assert.deepEqual(gefiltert[0].anhaenge.map((anhang) => anhang.zustand), ['geloescht', 'vorhanden']);
  assert.equal(liste[0].anhaenge[0].zustand, 'vorhanden');
});

function folgeMit({ gezeichnet = true, fokus = null } = {}) {
  const ereignisse = [];
  const stand = { nachricht: nachricht('z1', anh('a'), anh('b')) };
  const folge = geloeschtFolgeAnlegen({
    bekannteNachricht: (zeit) => (zeit === 'z1' ? { nachricht: stand.nachricht } : null),
    ersetzen: (bekannt, neu) => { ereignisse.push(['ersetzt', neu.anhaenge.map((anhang) => anhang.zustand)]); return gezeichnet; },
    hatFokus: (zeit, id) => id === fokus,
    ansagen: (text) => ereignisse.push(['ansage', text]),
    gehalten: (arbeit) => { ereignisse.push(['anker']); arbeit(); },
  });
  return { folge, ereignisse };
}

test('ersetzen: Fokus im gelöschten Anhang → einmal ersetzt und einmal angesagt, in einem Anker', () => {
  const { folge, ereignisse } = folgeMit({ fokus: 'a' });

  folge.ersetzen([{ id: 'a', zeit: 'z1' }]);

  assert.deepEqual(ereignisse, [['anker'], ['ersetzt', ['geloescht', 'vorhanden']], ['ansage', 'Anhang gelöscht: a.png']]);
});

test('ersetzen: Fokus woanders → ersetzt, keine Ansage', () => {
  const { folge, ereignisse } = folgeMit({ fokus: 'b' });

  folge.ersetzen([{ id: 'a', zeit: 'z1' }]);

  assert.deepEqual(ereignisse.filter(([art]) => art !== 'anker'), [['ersetzt', ['geloescht', 'vorhanden']]]);
});

test('ersetzen: nur gemerkt (Bearbeiten-Feld offen) → keine Ansage; unbekannte Nachricht → nichts', () => {
  const gemerkt = folgeMit({ gezeichnet: false, fokus: 'a' });
  gemerkt.folge.ersetzen([{ id: 'a', zeit: 'z1' }]);
  assert.equal(gemerkt.ereignisse.some(([art]) => art === 'ansage'), false);

  const fremd = folgeMit({ fokus: 'a' });
  fremd.folge.ersetzen([{ id: 'a', zeit: 'z9' }]);
  assert.equal(fremd.ereignisse.some(([art]) => art === 'ersetzt'), false);
});

test('ersetzen: bereits gelöschter Anhang ändert nichts', () => {
  const { folge, ereignisse } = folgeMit({ fokus: 'a' });
  folge.ersetzen([{ id: 'a', zeit: 'z1' }]);
  ereignisse.length = 0;

  const bereits = geloeschtFolgeAnlegen({
    bekannteNachricht: () => ({ nachricht: nachricht('z1', anh('a', 'geloescht')) }),
    ersetzen: () => { ereignisse.push('ersetzt'); return true; },
    hatFokus: () => true,
    ansagen: () => ereignisse.push('ansage'),
    gehalten: (arbeit) => arbeit(),
  });
  bereits.ersetzen([{ id: 'a', zeit: 'z1' }]);

  assert.deepEqual(ereignisse, []);
});
