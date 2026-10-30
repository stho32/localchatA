// Unit-Tests „Lage des Aktionsmenüs auf Touch-Geräten“ (R00046 I0042 B0701, UX K1; node --test): das Menü liegt links
// neben dem Schalter „⋯“, beginnt in dessen Höhe und wird in den sichtbaren Bereich geschoben — bevorzugt in den des
// Verlaufs, sonst in den sichtbaren Teil des Fensters (visualViewport).
import test from 'node:test';
import assert from 'node:assert/strict';
import { menueLage, MENUE_RAND, schalterGanzSichtbar } from '../../web/menuelage.js';

const bereich = (oben, unten) => ({ oben, unten, links: 0, rechts: 375 });
const schalter = (oben) => ({ oben, unten: oben + 44, links: 331, rechts: 375 });
const menue = (hoehe) => ({ breite: 120, hoehe });

test('Platz genug: das Menü beginnt in der Höhe des Schalters, rechts bündig an dessen linker Kante', () => {
  const lage = menueLage({ schalter: schalter(100), menue: menue(158), bereiche: [bereich(85, 600)] });

  assert.deepEqual(lage, { oben: 100, links: 331 - 120 });
});

test('unten zu knapp: das Menü rückt hoch, bis seine Unterkante den Rand lässt', () => {
  const lage = menueLage({ schalter: schalter(540), menue: menue(158), bereiche: [bereich(85, 585)] });

  assert.equal(lage.oben, 585 - MENUE_RAND - 158);
});

test('mitten in einem Verlauf, der kaum höher ist als das Menü, liegt das Menü vollständig darin', () => {
  const verlauf = bereich(85, 261);
  for (const oben of [93, 137, 181, 225]) {
    const lage = menueLage({ schalter: schalter(oben), menue: menue(158), bereiche: [verlauf] });

    assert.ok(lage.oben >= verlauf.oben + MENUE_RAND, `oben ${oben}`);
    assert.ok(lage.oben + 158 <= verlauf.unten - MENUE_RAND, `oben ${oben}`);
  }
});

test('der Verlauf ist niedriger als das Menü: das Menü nimmt den sichtbaren Teil des Fensters', () => {
  const lage = menueLage({ schalter: schalter(120), menue: menue(158), bereiche: [bereich(85, 200), bereich(0, 367)] });

  assert.ok(lage.oben >= MENUE_RAND);
  assert.ok(lage.oben + 158 <= 367 - MENUE_RAND);
});

test('links nicht aus dem Fenster: ein breites Menü bleibt mit dem Rand im Bereich', () => {
  const lage = menueLage({ schalter: { oben: 10, unten: 54, links: 100, rechts: 144 }, menue: { breite: 200, hoehe: 50 }, bereiche: [bereich(0, 600)] });

  assert.equal(lage.links, MENUE_RAND);
});

// Nacharbeit #78 (UX S1): scrollt der Verlauf von selbst, bleibt das Menü offen, solange sein Schalter ganz im sichtbaren
// Teil des Verlaufs steht.
test('der Schalter ganz im Bereich, auch bündig an oberem und unterem Rand: sichtbar', () => {
  for (const oben of [85, 300, 585 - 44]) {
    assert.equal(schalterGanzSichtbar(schalter(oben), bereich(85, 585)), true, `oben ${oben}`);
  }
});

test('der Schalter ein Pixel über oder unter dem Bereich, oder ganz draußen: nicht sichtbar', () => {
  for (const oben of [84, 585 - 43, -200, 700]) {
    assert.equal(schalterGanzSichtbar(schalter(oben), bereich(85, 585)), false, `oben ${oben}`);
  }
});
