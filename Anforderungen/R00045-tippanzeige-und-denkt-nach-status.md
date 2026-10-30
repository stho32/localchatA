---
id: R00045
status: Neu
datum: 2026-10-30
---

# R00045: Wer tippt oder nachdenkt, ist sichtbar

## Beschreibung

Wer in einem Gespräch gerade tippt, wird den anderen Teilnehmern, die dieses Gespräch geöffnet haben, an der Eingabe angezeigt („anna tippt …“). Ein Agent kann auf ähnliche Weise über die API einblenden, dass er gerade nachdenkt („claude denkt nach …“), und die aktiven Zustände eines Gesprächs lesen. Die Anzeige ist flüchtig: Sie wird nicht gespeichert, verfällt von selbst und weckt niemanden.

Zahlt ein auf: [Vision](R00000-vision.md) — „Wer tippt oder nachdenkt, ist sichtbar“ (neuer Zielbild-Punkt, 2026-10-30), dazu „Wer da ist, ist sichtbar“, „Live überall“, „Mensch und KI sind gleichberechtigt“ und die Leitplanke „Lastprofil Team-LAN“.

Herkunft: Karte #77 — Auftraggeber 2026-10-30: „Wenn ein Benutzer gerade tippt, dann sollte dieser Status üblich angezeigt werden. Bei AI Agents soll es auf ähnliche Weise möglich sein, dass sie einblenden können, dass sie "gerade nachdenken".“; entschieden vom product-owner in `AI-Team/po-entscheidungen.md` Zeilen 133–141.

Umfang (WBS `Dokumentation/Planung/localchata.md`): **I0040** Sehen, wer gerade tippt (D0003 Channels) und **I0041** Als Agent „denkt nach“ anzeigen und lesen (D0005 Agent an der API). Features und Bubbles gibt es noch nicht (Rolling Wave); sie kommen nach der ADR des `architekt`, danach geht die Klammer auf die Slices über.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in der Vision und in R00004/R00011: Teilnehmer, Kennung (`name@herkunft`), Sitzung, Gespräch (Channel, später auch Direktgespräch), Ereignisstrom, Schreibkonto, Plattenquote, Betriebsprotokoll, Tageslog. Neu und hier festgelegt: **Zustand** (`tippt` oder `denkt nach`, fester Wortlaut, kein Freitext), **Gültigkeit** (Dauer, die ein gemeldeter Zustand ohne Erneuerung gilt).

## Geschäftlicher Nutzen

Wirkungskette: Zwei Menschen schreiben im selben Channel, einer beginnt eine lange Antwort → der andere sieht „anna tippt …“ und wartet, statt dazwischenzuschreiben → das Gespräch läuft ohne Überschneidung. Ein Agent braucht für eine Antwort 40 s → ohne Anzeige halten die Menschen ihn für abgestürzt und fragen nach → mit „claude denkt nach …“ warten sie, und andere Agenten können über das Lesen erkennen, dass ein Mensch gerade tippt, und antworten nicht hinein. Die Anzeige kostet weder Platte noch Verlauf, weil sie flüchtig ist.

## Funktionale Anforderungen

Route, Pfade, Körperform, Ereignisart und Feldnamen legt die ADR fest; die Kriterien nennen die Wirkung. „Setzen“, „Löschen“ und „Lesen“ bezeichnen die Aufrufe der einen Route für Mensch und Agent.

### I0040 Sehen, wer gerade tippt

- **Zustand `tippt`** meldet der Browser selbst, wenn im Eingabefeld eines Gesprächs getippt wird und das Feld danach nicht leer ist; er meldet höchstens alle 3 s, die erste Meldung sofort. Eine Meldung gilt **6 s** ab dem Eingang.
- **Erscheinen**: bei allen anderen offenen Sichten desselben Gesprächs steht an der Eingabe „<name> tippt …“ binnen 1 s nach der Meldung (Zweck: Vision „Live überall“, Zustellung < 1 s; erwartet wenige ms, Grenze liegt weit über dem erwarteten Wert).
- **Verschwinden**: sofort (binnen 1 s) mit dem Senden einer Nachricht dieses Absenders im Gespräch, mit leerem Feld, beim Channel-Wechsel, beim Verlassen und bei offline; sonst mit Ablauf der Gültigkeit — die Regel exakt 6 s ab der letzten Meldung, die Anzeige als obere Grenze höchstens 8 s danach (6 s + 30 %).
- **Sichtbarkeit**: nur Teilnehmer, die das Gespräch sehen dürfen (Direktgespräch nur die Beteiligten, archivierte Channels nie); nur an der Eingabe des **geöffneten** Gesprächs, nie in Seitenleiste oder Teilnehmerliste; man sieht sich nie selbst.
- **Darstellung**: ein Name, zwei Namen, ab drei gezählt (Beispiel `anna tippt …` · `anna und ben tippen …` · `3 Teilnehmer tippen …` — den endgültigen Wortlaut legt der `ux-reviewer` fest); der Platz ist reserviert, der Verlauf springt nicht; Ansage für Bildschirmleser ohne Dauerrauschen (Erneuerungen ändern den Text nicht und lösen keine neue Ansage aus).
- **Inhaber**: nur die eigene Kennung — aus der Sitzung, nie aus dem Körper — setzt und löscht den eigenen Zustand.
- **Flüchtig**: nicht ins Tageslog, nicht in den Verlaufsindex, nicht im Nachholen (`warten`, Nachladen), keine Betriebsprotokollzeile je Signal (höchstens eine Episode bei Abweisung wie bei anderen Grenzen); nach Abriss des Stroms wird nichts nachgeholt, die Anzeige verfällt von selbst.
- **Last**: Signale zählen nicht zum Schreibkonto und nicht zur Plattenquote; eigene Bremse je Kennung und Gespräch (Zahl legt der `architekt` fest); ein Signal weckt keinen wartenden Agenten.
- **Keine Abschaltung** je Teilnehmer und kein Betreiberschalter.

### I0041 Als Agent „denkt nach“ anzeigen und lesen

- **Setzen** durch den Agenten über die API (die Antwort nennt den Ablaufzeitpunkt): Zustand `denkt nach` (auch `tippt` ist zulässig; ein anderer Wortlaut wird abgewiesen), für ein Gespräch, in dem der Agent mitlesen darf; Anzeige bei den anderen „<name> denkt nach …“ binnen 1 s.
- **Gültigkeit** angebbar in Sekunden, Standard **30 s**, höchstens **120 s**, erneuerbar — jeder Aufruf gilt ab seinem Eingang, Gültigkeiten werden nicht addiert. Ein größerer Wert wird nie länger als 120 s wirksam (begrenzt oder abgewiesen — der `architekt` legt fest; Empfehlung des planer: abweisen mit 422, damit der Agent es merkt).
- **Verschwinden**: sofort (binnen 1 s) mit einer Nachricht dieses Agenten im Gespräch, mit Löschen über die API, beim Verlassen und bei offline; sonst mit Ablauf der Gültigkeit.
- **Inhaber**: nur die eigene Kennung setzt oder löscht den eigenen Zustand; ein anderer Teilnehmer kann den Zustand eines Agenten weder setzen noch löschen, auch nicht mit dessen Kennung im Körper.
- **Lesen**: dieselbe Route liefert die aktiven Zustände eines Gesprächs — je Eintrag Kennung, Zustand und den Zeitpunkt, bis zu dem er gilt (Feldnamen laut ADR); die eigene Kennung ist nicht enthalten (wie in der Anzeige); für ein Gespräch, das der Aufrufer nicht sehen darf, antwortet die Instanz wie für ein nicht vorhandenes (kein Orakel).
- **Flüchtig, ohne Wecken, ohne Schreibkonto**: wie bei `tippt`; `warten` (I0016) bleibt unberührt.
- **Bremse**: derselbe Mechanismus wie bei `tippt`; ein Agent, der Setzen in Schleife aufruft, wird abgewiesen (429), ohne dass seine Nachrichten betroffen sind.

## Nicht-funktionale Anforderungen

- **Last** (Vision „Lastprofil Team-LAN“, PO Zeile 139): 20 Menschen tippen, 50 Agenten melden `denkt nach`, 30 Nachrichten/s. Rechenbeispiel: im ungünstigsten Fall 70 Teilnehmer × 1 Signal je 3 s ≈ 23 Signale/s zusätzlich zu 30 Nachrichten/s. Die Zustellung der Nachrichten bleibt unter 1 s (Messung wie Z10 in R00011: von der 201 bis zum Ereignis beim Leser), das Schreibkonto ist unberührt.
- **Zeit**: Zweck jeder Grenze steht an ihrem Kriterium; Regel und Zustellung sind getrennt (Regel exakt 6 s bzw. Gültigkeit, Anzeige als obere Grenze 30 % darüber); die Uhr startet beim Eingang der Meldung bzw. der Antwort des Setzens.
- **Sicherheit**: Kennung nie vom Client; Zustand und Lesen folgen der Sichtbarkeit der Nachrichten (kein Orakel über fremde Gespräche, auch nicht über Lesen); Wortlaut fest, kein Freitext, also kein Einschleusen von Text in fremde Sichten; Gate `security-reviewer`.
- **Bedienbarkeit** (`ux-reviewer` legt fest): Wortlaut, Platz, Ansage für Bildschirmleser, Touch und schmale Fenster (375 px und 320 px, der Hinweis darf nicht umbrechen und den Verlauf nicht verschieben).

## Akzeptanzkriterien

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner mit dem Channel `allgemein`; Menschen `anna`, `ben`, `cora` in je einem Browser-Fenster, Agenten `claude` und `codex` über die API. Die Szenarien stehen in [R00045](user-stories/R00045.md).

### I0040 Sehen, wer gerade tippt

- [ ] **Erscheinen** (E2E, zwei Browser): `anna` tippt in `# allgemein` → bei `ben` steht binnen 1 s nach dem ersten Tastendruck „anna tippt …“; bei `anna` steht nichts.
- [ ] **Verschwinden sofort** (E2E): `anna` sendet die Nachricht → bei `ben` ist die Anzeige binnen 1 s weg und die Nachricht steht im Verlauf; `anna` tippt erneut und löscht das Feld → weg binnen 1 s; `anna` tippt und wechselt in `# technik` → bei `ben` in `# allgemein` weg binnen 1 s.
- [ ] **Verschwinden durch Ablauf** (E2E, Rechenbeispiel): `anna` tippt zwei Zeichen (eine Meldung, t = 0 s) und hört auf → die Anzeige gilt bis t = 6 s (die Antwort auf das Setzen nennt den Ablaufzeitpunkt) und ist bei `ben` spätestens bei t = 8 s weg; nie vor Ablauf der Gültigkeit, gemessen über den genannten Zeitpunkt, nicht über die Ankunft.
- [ ] **Takt der Meldungen** (E2E, Netzbeobachtung): `anna` tippt 10 s ohne Pause → höchstens ⌈10 / 3⌉ + 1 = 5 Meldungen; ein leeres Feld löst keine Meldung aus.
- [ ] **Sichtbarkeit** (E2E, Integration): Direktgespräch `anna`–`ben`, `cora` erhält kein Ereignis und beim Setzen die Antwort für ein nicht vorhandenes Gespräch (das Lesen für `cora` prüft I0041) (gilt, sobald I0012 grün; bis dahin Unit-Test der Sichtbarkeitsregel); archivierter Channel: keine Anzeige, Setzen und Lesen abgewiesen (gilt, sobald I0022 grün); Seitenleiste und Teilnehmerliste zeigen nie etwas.
- [ ] **Inhaber** (Integration): `cora` setzt mit der Kennung `anna@127.0.0.1` im Körper → der Zustand gilt für `cora`, nicht für `anna`; `cora` löscht → nur ihr eigener Zustand ist weg, der von `anna` bleibt.
- [ ] **Mehrere Namen** (E2E): `anna` tippt → „anna tippt …“; `ben` kommt dazu → ein Satz mit beiden Namen; `cora` kommt dazu → „3 Teilnehmer tippen …“ (Wortlaut laut `ux-reviewer`); jeder sieht sich selbst nie.
- [ ] **Platz reserviert** (E2E): die Position der letzten Nachricht im Verlauf ist vor, während und nach der Anzeige gleich (gleiche Pixelzeile, Toleranz 0 px); bei 375 px Breite ebenso.
- [ ] **Bildschirmleser** (E2E): während 12 s ununterbrochenen Tippens ändert sich der Text der Ansage-Region genau zweimal (Erscheinen, Verschwinden) — Erneuerungen ändern ihn nicht.
- [ ] **Flüchtig** (Integration, E2E): während `anna` tippt und danach sind `sha256sum` über alle Dateien unter `channels/` unverändert; `ben` lädt neu → keine Anzeige und kein Eintrag im Verlauf; der Strom von `ben` wird getrennt und neu aufgebaut → nichts wird nachgeholt; im Betriebsprotokoll steht je Signal keine Zeile.
- [ ] **Kein Wecken** (Integration): `codex` wartet über `warten`; `anna` tippt 10 s → `warten` kehrt nicht zurück (läuft in seine Zeitgrenze); eine Nachricht von `anna` weckt wie bisher.
- [ ] **Schreibkonto und Quote** (Integration): 60 Meldungen von `anna` in 30 s, danach sendet sie 30 Nachrichten in einem Zug → alle 201; `belegt` in `/api/gesundheit` ist unverändert.
- [ ] **Bremse** (Integration): mehr Meldungen je Kennung und Gespräch, als die ADR zulässt, in einer Sekunde → Abweisung 429 mit `Retry-After`; die Anzeige bleibt für bereits Gemeldetes bestehen; danach geht Setzen wieder.
- [ ] **Keine Abschaltung** (E2E): die Oberfläche bietet weder je Teilnehmer noch für die Instanz einen Schalter dafür.

### I0041 Als Agent „denkt nach“ anzeigen und lesen

- [ ] **Setzen und Anzeige** (Integration, E2E): `claude` setzt `denkt nach` ohne Gültigkeit → bei `anna` und `ben` steht binnen 1 s „claude denkt nach …“; Lesen nennt den Eintrag mit einer Gültigkeit bis ≈ Eingang + 30 s.
- [ ] **Nachricht beendet den Zustand** (E2E): `claude` sendet im Gespräch eine Nachricht → die Anzeige ist binnen 1 s weg und die Nachricht steht im Verlauf; eine Nachricht von `claude` in `# technik` beendet den Zustand in `# allgemein` nicht.
- [ ] **Ablauf** (Integration, Rechenbeispiel, gestellte Uhr wo möglich): ohne Erneuerung gilt der Zustand 30 s; Erneuerung bei t = 20 s gilt bis t = 50 s (nicht bis 60 s); Gültigkeit 10 angegeben → bis t = 10 s; 120 → bis t = 120 s; 500 → nie länger als 120 s sichtbar (begrenzt, oder 422 mit `regel` und ohne Wirkung — laut ADR); Anzeige spätestens 30 % nach Ablauf weg (Standard: 39 s).
- [ ] **Löschen** (Integration): `claude` löscht → Anzeige binnen 1 s weg, Lesen leer; zweites Löschen ist harmlos (kein Fehler, den der Agent behandeln müsste — Form laut ADR).
- [ ] **Inhaber** (Integration, Abnahmepunkt 4): `codex` setzt oder löscht mit der Kennung `claude@127.0.0.1` im Körper → wirkt nur auf `codex`; der Zustand von `claude` bleibt unverändert sichtbar und lesbar.
- [ ] **Lesen** (Integration): `anna` tippt und `claude` denkt nach → `codex` liest beide Einträge mit Kennung, Zustand und Gültigkeit; `claude` liest nur `anna`; Lesen für ein Gespräch, das der Aufrufer nicht sehen darf, antwortet wie für ein nicht vorhandenes — auch für `cora` im Direktgespräch `anna`–`ben` (gilt, sobald I0012 grün); ohne Sitzung 401.
- [ ] **Fehlerfälle** (Integration): Wortlaut `schreibt` oder ein Freitext → 422 und keine Wirkung; Gültigkeit 0, negativ oder keine Zahl → 422; fremder `Origin` → 403; unbekannter Channel → 404 wie sonst.
- [ ] **Flüchtig, kein Wecken** (Integration): Tageslog und Verlauf nach dem Neuladen unverändert (`sha256sum`), kein Eintrag im Nachholen, keine Betriebsprotokollzeile je Signal; ein wartender Agent wird nicht geweckt.
- [ ] **Verlassen und offline** (Integration): `claude` verlässt (`POST /api/verlassen`) → Zustand binnen 1 s weg; Strom getrennt, bis die Gnadenfrist ablief → weg.
- [ ] **Last** (Gate, Lastprofil Team-LAN): 20 Menschen tippen, 50 Agenten melden `denkt nach` (Erneuerung alle 20 s), 30 Nachrichten/s über 60 s → Zustellung jeder Nachricht unter 1 s (Maximum, wie Z10 gemessen); Schreibkonto der Senderkennungen unberührt (kein 429 durch Signale); im Bericht stehen Leerlauf-p90, p50, p90 und max.

## Betroffene Verzeichnisstruktur

Folgt der ADR (`architekt`): Zustand im Speicher mit Ablauf neben der Anwesenheit (A00010), Ereignisart im Live-Kanal (A00011), Route in der HTTP-Schnittstelle (A00004), Anzeige in der Weboberfläche (A00005, `web/`), Tests spiegeln die Themenordner.

## Technische Überlegungen

### Ablauf (Vorplanung — Bubbles entstehen nach der ADR)

1. Zustand je (Kennung, Gespräch) im Speicher mit Ablaufzeitpunkt; ein Zustand je Kennung und Gespräch, ein neuer ersetzt den alten.
2. Route nimmt Setzen und Löschen der eigenen Kennung entgegen und liefert Lesen; Sichtbarkeit wie bei Nachrichten.
3. Ereignis im Live-Strom an die Sichten, die das Gespräch sehen dürfen; nach Abriss kein Nachholen.
4. Anzeige an der Eingabe; Browser meldet gedrosselt bei Eingabe.
5. Ende durch Senden, leeres Feld, Wechsel, Verlassen, offline oder Ablauf.

### Grobentwurf

Entfällt bis zur ADR (Rolling Wave).

## Abhängigkeiten

- Abhängig von: R00011 (Senden, Eingabefeld, Strom), R00004 (Anwesenheit), I0014 Als Agent beitreten (WBS, noch ohne Requirement).
- Nachgezogen, sobald gebaut: I0012 Direktnachricht senden (Sichtbarkeit im Direktgespräch), I0022 Channel archivieren (archivierte Channels).
- Blockiert: nichts.

## Offene Fragen

Aus den Entscheidungen des product-owner nicht beantwortet, vom planer mit Empfehlung gesetzt (der `architekt` bzw. `ux-reviewer` bestätigt oder ändert):

1. **Größerer Wert als 120 s** — begrenzen oder abweisen: Empfehlung abweisen (422), `architekt` (Abnahmepunkt 3 lässt beides zu).
2. **Ein Zustand je Kennung und Gespräch**, ein neuer ersetzt den alten (`denkt nach` ersetzt `tippt`) — Annahme des planer, `architekt` bestätigt.
3. **Erneuerung** gilt ab Eingang, keine Addition — Annahme des planer.
4. **Lesen ohne eigene Kennung** — Annahme des planer, passend zu „man sieht sich nie selbst“.
5. **Die Antwort auf das Setzen und die Lese-Antwort nennen den Ablaufzeitpunkt** — braucht der Test für „nie vor Ablauf weg“; Feldnamen `architekt`.
6. **Anzeigename**: Name vor dem `@` wie im Verlauf — `ux-reviewer`.
7. **Beide Zustände gleichzeitig** (ein Mensch tippt, ein Agent denkt nach) — eine oder zwei Zeilen: `ux-reviewer`.

## Warum löst diese Anforderung das Problem?

Das Problem ist die Stille zwischen Absicht und Nachricht: wer wartet, weiß nicht, ob ein Gegenüber schreibt, nachdenkt oder fehlt. Die Anzeige beantwortet genau diese Frage, ohne Gesprächsinhalt zu speichern: Eingabe im Browser bzw. Setzen durch den Agenten → flüchtiger Zustand mit Ablauf → Ereignis im Live-Strom → Anzeige bei den Mitlesenden → Ende mit Nachricht, Löschen oder Ablauf. Dass sie wirklich nichts anderes tut, belegen die Kriterien „flüchtig“, „kein Wecken“ und „Last“.

## Notizen

- Optionen: Die Richtung ist vom product-owner entschieden (Zeilen 135–141, Optionen dort); verworfen wurden Freitext im Status, Abschaltung je Teilnehmer und Wecken durch Signale (jeweils später additiv möglich).
- Referenzen (Verhalten, nichts übernommen): Slack `assistant.threads.setStatus` (https://docs.slack.dev/reference/methods/assistant.threads.setStatus, abgerufen 2026-10-30) — „is thinking…“, endet mit der Antwort, leerem Status oder nach 2 Minuten; Discord Trigger Typing Indicator (https://docs.discord.com/developers/resources/channel, abgerufen 2026-10-30) — verfällt nach 10 s.
- Karte #77: laufende Karten bauen die alte Linie nicht; neuer Umfang.
