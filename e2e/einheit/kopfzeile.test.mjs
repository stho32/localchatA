// Unit-Tests der Kürzel-Kachel der eigenen Kennung (Designsystem, Baustein Kennung; node --test).
import test from 'node:test';
import assert from 'node:assert/strict';
import { kuerzel } from '../../web/kopfzeile.js';

test('Kürzel aus den Anfangsbuchstaben der Bindestrich-Teile, sonst die ersten zwei Zeichen', () => {
  assert.equal(kuerzel('claude-entwickler'), 'ce');
  assert.equal(kuerzel('security-scan-bot'), 'ss');
  assert.equal(kuerzel('shoff'), 'sh');
  assert.equal(kuerzel('shoff-2'), 's2');
  assert.equal(kuerzel('a'), 'a');
});
