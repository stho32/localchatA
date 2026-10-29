---
id: X00002
name: Benutzerrechte
art: Aspekt
geltung: Makro
befund: Einheitlich
hausantwort:
betrifft: [A00004, A00011, A00010]
herkunft: dialog
quellen:
  - src/http/
  - src/live/
---

# Benutzerrechte

## Hausantwort

Keine Vorlage zugeordnet.

## Systemantwort

Es gibt keine Rechte (Vision: „keine Benutzerverwaltung und keine Rechte“). Einzige Sichtbarkeitsregel: ein Direktgespräch sehen nur seine beiden Beteiligten — in Abruf, Suche, SSE und Warten. Geprüft wird an der Systemgrenze ([A00004](../A00004-http-schnittstelle.md)) bzw. im Filter des [Live-Kanals](../A00011-live-kanal.md), nie in Index oder Ablage. Durchsetzung: Integrationstest je Weg (Abruf, Suche, SSE, Warten), dass ein Dritter eine Direktnachricht nicht erhält.

Ohne Rechte bleibt die Abwehr fremder **Ursprünge**: Wirkungen nur aus dem eigenen Ursprung oder ohne Browser (Origin-/`Sec-Fetch-Site`-Prüfung), nur unter erlaubten Host-Namen (gegen DNS-Rebinding), Sicherheitsheader auf jeder Antwort ([ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md)). Gegen Namensbesetzung: Namensregel ohne Verwechslungen ([ADR-0013](../../ADRs/0013-namensregel-als-erlaubnisliste.md)), höchstens 64 Sitzungen je Herkunft ([ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md)). Durchsetzung: Angriffsmatrix aus ADR-0014 über das echte Programm, Mutationsprobe je Wächter.

## Abweichungen

## Offene Fragen

- Designsystem-Frage 10 (wer sieht welche Channels) — fachlich, `planer`.

## Notizen / Quellen

Sichtbarkeit ist kein Schutz gegen Absicht: Dateien sind lesbar, Tokens sind nicht an Personen gebunden, nur an die Herkunft ([ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md)); wer im LAN direkt per IP spricht, wird durch den Grenzschutz nicht abgehalten — gewollt, es gibt keine Anmeldung.

- **Ergänzung 2026-10-28 (Karte #4, I0005):** Die Teilnehmerliste sieht jeder mit Sitzung vollständig — alle lebenden Sitzungen mit Kennung (also Herkunft), Art und Status ([ADR-0016](../../ADRs/0016-anwesenheitsstatus-aus-haltern-mit-nachlauf-liste-als-stand-plus-wechsel.md), Regel 6); es gibt keine Sichtbarkeitsregel für Anwesenheit. Die Herkunft ist ohnehin Teil jeder Kennung (Vision). Ohne Sitzung gibt es keine Liste (`GET /api/teilnehmer` und der Strom verlangen sie). Halter und Lebenszeichen sind Wirkungen eines `GET`, entstehen aber nur mit eigenem Header — Lesart von Regel 3 in der Ergänzung zu [ADR-0014](../../ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md).
- **Ergänzung 2026-10-28 (Karte #12, I0018):** Die Frist eines Channels ändert jeder mit Sitzung — auch das Verkürzen, das Verlauf endgültig löscht ([ADR-0031](../../ADRs/0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md)). Statt eines Rechts: Grenzschutz wie jede schreibende Route, Bestätigung (409 `bestaetigen` mit Zahlen, erst `bestaetigt: true` löscht) und eine stderr-Zeile mit der Kennung des Änderers. Durchsetzung: Integrationstest „Verkürzen ohne `bestaetigt` löscht nichts“ und Mutationsprobe des Bestätigungs-Wächters.
- **Ergänzung 2026-10-28 (R00015, F0056):** Gegen Namensbesetzung gelten ab F0056 höchstens **128** Sitzungen je Herkunft (statt 64), 512 gesamt ([ADR-0012](../../ADRs/0012-identitaet-und-sitzung.md), Ergänzung R00015); die Grenze schützt nur, solange Agenten ihre Sitzung wiederverwenden (Wiederkehr) statt je Aufruf beizutreten.
- **Ergänzung 2026-10-28 (I0018 im Betriebsprotokoll):** Die Spur einer Friständerung mit der Kennung des Änderers (Ergänzung Karte #12) ist ab dem Bau von I0018 ein `INFORMATION frist`-Eintrag in `logs/<tag>.log` statt einer stderr-Zeile ([ADR-0030](../../ADRs/0030-entfallen-beim-start-beim-tageswechsel-und-beim-verkuerzen-loeschen-unter-der-gespraechssperre.md), Ergänzung „wie I0018 an das Betriebsprotokoll anbindet“). Die Spur hängt damit an der Protokollstufe: mit `--protokoll-stufe warning` oder höher fehlt sie — Standard bleibt `information`.
- **Ergänzung 2026-10-29 (Karte #9, I0010 — vorgeschlagen):** Anhänge öffnen **ohne Sitzung**: die Anhang-ID (128 Bit Zufall) ist die Berechtigung, `GET /anhaenge/{id}` prüft keine Kennung ([ADR-0047](../../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Frage 2, Z1). Für Channels ändert das nichts an der Sichtbarkeit (jeder im LAN kann beitreten und lesen); für Direktgespräche (I0012) heißt „nur die Beteiligten sehen es“ bei Anhängen „nur wer den Link kennt“ — die ID erscheint nur auf den gefilterten Wegen, ein weitergegebener Link öffnet aber. Richtungsfrage B4 an den Betreiber; bei „nein“ Kurzzeit-Ticket für Direktgespräche (Option Z3). Durchsetzung: der bestehende Integrationstest je Weg „ein Dritter erhält keine Direktnachricht“ wächst um „… und keine Anhang-ID daraus“; die ID steht nie im Betriebsprotokoll.
- **Ergänzung 2026-10-29 (Karte #9, I0010 — entschieden nach den Antworten des Betreibers B1–B5; ADR-0046/0047 Akzeptiert):** Die Richtungsfrage B4 ist entschieden: **„wer den Link hat, öffnet (auch Direktgespräche), bis der Tag entfällt“** (Betreiber, 2026-10-29). Für Direktgespräche (I0012) heißt „nur die Beteiligten sehen es“ bei Anhängen damit ausdrücklich „nur die Beteiligten bekommen den Link; wer ihn hat, öffnet“; ein Kurzzeit-Ticket (Option Z3) ist nicht vorgesehen ([ADR-0047](../../ADRs/0047-anhaenge-ausliefern-route-ohne-sitzung-ueber-anhang-id-art-vom-server-inline-nur-gefahrloses-unter-csp-sandbox.md) Frage 2, Regel 2). Durchsetzung bleibt: der Integrationstest je Weg „ein Dritter erhält keine Direktnachricht“ wächst um „… und keine Anhang-ID daraus“ (Angriffsmatrix M13); die ID steht nie im Betriebsprotokoll (M11).
