'use strict';
// Der Testrahmen jeder Spec (Karte #49): `test` schließt nach jedem Test alle Browser-Kontexte, die der Test selbst
// mit browser.newContext() geöffnet hat — Playwright schließt nur seinen eigenen `context`. Ein offener Kontext
// verbände seinen Tab mit dem alten Port neu und träte dort bei einer späteren Instanz still bei.
const base = require('@playwright/test');

const test = base.test.extend({
  // Als automatische Fixture zuerst aufgebaut und zuletzt abgebaut: Playwrights eigener `context` ist dann schon zu.
  kontexteEndenMitDemTest: [async ({ browser }, use) => {
    const offenVorher = browser.contexts().length;
    if (offenVorher > 0) {
      throw new Error(`Vor dem Test sind ${offenVorher} Browser-Kontexte offen — ein früherer Test hat sie hinterlassen.`);
    }
    await use();
    await Promise.all(browser.contexts().map((kontext) => kontext.close()));
  }, { auto: true }],
});

module.exports = { test, expect: base.expect };
