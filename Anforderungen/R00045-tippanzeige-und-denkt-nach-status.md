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

Umfang (WBS `Dokumentation/Planung/localchata.md`, Stand nach ADR-0051): **I0040** Sehen, wer gerade tippt (D0003 Channels) mit den Ausbaustufen **F0161** Tippen sehen (B0684–B0689), **F0162** Tippanzeige lesbar: Namen, Platz, Ansage (B0690, B0691) und **F0163** Tippmeldungen gebremst und ihr Ende vollständig (B0692–B0694), sowie **I0041** Als Agent „denkt nach“ anzeigen und lesen (D0005 Agent an der API; B0695–B0698). Die Klammer R00045 liegt auf den Slices F0161, F0162, F0163 und I0041; die Kriterien unten tragen ihr Kürzel. Entschieden ist die Technik in [ADR-0051](../Dokumentation/ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md) (Baustein A00019).

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in der Vision und in R00004/R00011: Teilnehmer, Kennung (`name@herkunft`), Sitzung, Gespräch (Channel, später auch Direktgespräch), Ereignisstrom, Schreibkonto, Plattenquote, Betriebsprotokoll, Tageslog. Neu und hier festgelegt: **Zustand** (`tippt` oder `denkt nach`, fester Wortlaut, kein Freitext), **Gültigkeit** (Dauer, die ein gemeldeter Zustand ohne Erneuerung gilt).

## Geschäftlicher Nutzen

Wirkungskette: Zwei Menschen schreiben im selben Channel, einer beginnt eine lange Antwort → der andere sieht „anna tippt …“ und wartet, statt dazwischenzuschreiben → das Gespräch läuft ohne Überschneidung. Ein Agent braucht für eine Antwort 40 s → ohne Anzeige halten die Menschen ihn für abgestürzt und fragen nach → mit „claude denkt nach …“ warten sie, und andere Agenten können über das Lesen erkennen, dass ein Mensch gerade tippt, und antworten nicht hinein. Die Anzeige kostet weder Platte noch Verlauf, weil sie flüchtig ist.

## Funktionale Anforderungen

„Setzen“, „Löschen“ und „Lesen“ bezeichnen die Aufrufe der einen Route für Mensch und Agent: `PUT`, `DELETE` und `GET /api/channels/{name}/aktivitaet` (ADR-0051 Regel 4). Körper des Setzens: `{"zustand": "tippt" | "denkt_nach", "gueltig_s"?: 1..120}`; Antwort `{"zustand", "bis", "jetzt"}` (Zeiten RFC 3339 in Serverzeit); Lesen `{"jetzt", "aktivitaeten": [{"kennung", "zustand", "bis"}]}`. Auf der Leitung heißt der Zustand `denkt_nach`, in der Anzeige „denkt nach“.

### I0040 Sehen, wer gerade tippt

- **Zustand `tippt`** meldet der Browser selbst, wenn im Eingabefeld eines Gesprächs getippt wird und das Feld danach nicht leer ist; er meldet höchstens alle 3 s, die erste Meldung sofort. Eine Meldung gilt **6 s** ab dem Eingang. Das Feld zum Bearbeiten einer eigenen Nachricht meldet nichts (nur das Eingabefeld des Gesprächs). [F0161]
- **Erscheinen**: bei allen anderen offenen Sichten desselben Gesprächs steht an der Eingabe „<name> tippt …“ binnen 1 s nach der Meldung (Zweck: Vision „Live überall“, Zustellung < 1 s; erwartet wenige ms, Grenze liegt weit über dem erwarteten Wert).
- **Verschwinden**: sofort (binnen 1 s) mit dem Senden einer Nachricht dieses Absenders im Gespräch, mit leerem Feld, beim Channel-Wechsel, beim Verlassen und — bei einem Menschen — bei offline; sonst mit Ablauf der Gültigkeit — die Regel exakt 6 s ab der letzten Meldung, die Anzeige als obere Grenze höchstens 8 s danach (6 s + 30 %). Kommt die Nachricht und ihr Ende nicht in dieser Reihenfolge an, blendet der Tab die Anzeige beim Eintreffen der Nachricht selbst aus. [F0161; Verlassen, offline: F0163]
- **Sichtbarkeit**: nur Teilnehmer, die das Gespräch sehen dürfen (Direktgespräch nur die Beteiligten, archivierte Channels nie); nur an der Eingabe des **geöffneten** Gesprächs, nie in Seitenleiste oder Teilnehmerliste; man sieht sich nie selbst.
- **Darstellung**: ein Name, zwei Namen, ab drei gezählt (Beispiel `anna tippt …` · `anna und ben tippen …` · `3 Teilnehmer tippen …` — den endgültigen Wortlaut legt der `ux-reviewer` fest); der Platz ist reserviert, der Verlauf springt nicht; Ansage für Bildschirmleser ohne Dauerrauschen (Erneuerungen ändern den Text nicht und lösen keine neue Ansage aus).
- **Inhaber**: nur die eigene Kennung — aus der Sitzung, nie aus dem Körper — setzt und löscht den eigenen Zustand; enthält der Körper eine Kennung (oder sonst ein unbekanntes Feld), antwortet die Instanz 422 `regel: "feld"` und ändert nichts. [F0161]
- **Flüchtig**: nicht ins Tageslog, nicht in den Verlaufsindex, nicht im Nachholen (`warten`, Nachladen), keine Betriebsprotokollzeile je Signal (höchstens eine Episode bei Abweisung wie bei anderen Grenzen); nach Abriss des Stroms wird nichts nachgeholt, die Anzeige verfällt von selbst.
- **Last**: Signale zählen nicht zum Schreibkonto und nicht zur Plattenquote; eigene Bremse nur für das Setzen: je Kennung und Gespräch 10 Meldungen Vorrat, 1 je Sekunde Nachschub, je Herkunft 120 und 12 je Sekunde; die 11. Meldung in derselben Sekunde wird mit 429 und `Retry-After: 1` abgewiesen; Löschen und Lesen sind nie gebremst. Eine Kennung hat höchstens 16 Zustände (je Gespräch einen), der 17. wird mit 422 `regel: "aktivitaeten"` abgewiesen. Ein Signal weckt keinen wartenden Agenten. [F0163]
- **Keine Abschaltung** je Teilnehmer und kein Betreiberschalter.

### I0041 Als Agent „denkt nach“ anzeigen und lesen

- **Setzen** durch den Agenten über die API (die Antwort nennt `bis` und `jetzt`): Zustand `denkt_nach` (auch `tippt` ist zulässig und gilt ohne Angabe 6 s; jeder andere Wortlaut, auch „denkt nach“ mit Leerzeichen oder „Tippt“, wird mit 422 `regel: "zustand"` abgewiesen), für ein Gespräch, in dem der Agent mitlesen darf; Anzeige bei den anderen „<name> denkt nach …“ binnen 1 s. Ein Zustand je Kennung und Gespräch: ein neues Setzen ersetzt das alte (`denkt_nach` ersetzt `tippt`). [I0041]
- **Gültigkeit** angebbar als `gueltig_s` (ganze Zahl in Sekunden), Standard für `denkt_nach` **30 s**, höchstens **120 s**, erneuerbar — jeder Aufruf gilt ab seinem Eingang, Gültigkeiten werden nicht addiert. Ein Wert über 120, 0, ein negativer, ein gebrochener und ein Text werden mit 422 `regel: "gueltigkeit"` abgewiesen und ändern nichts (entschieden in der ADR, nicht begrenzt).
- **Verschwinden**: sofort (binnen 1 s) mit einer Nachricht dieses Agenten im Gespräch, mit Löschen über die API (`DELETE` antwortet 204, auch wenn nichts aktiv war) und beim Verlassen (`POST /api/verlassen`); sonst mit Ablauf der Gültigkeit. Ein getrennter Strom beendet den Zustand eines Agenten **nicht**, weil ein Agent zwischen zwei Warte-Aufrufen ohne Strom immer offline ist (offene Bestätigung F5, siehe unten).
- **Inhaber**: nur die eigene Kennung setzt oder löscht den eigenen Zustand; ein anderer Teilnehmer kann den Zustand eines Agenten weder setzen noch löschen — eine Kennung im Körper wird mit 422 `regel: "feld"` abgewiesen und ändert nichts.
- **Lesen**: dieselbe Route liefert die aktiven Zustände eines Gesprächs als `aktivitaeten` — je Eintrag `kennung`, `zustand` und `bis`, dazu `jetzt`; nur gültige Einträge, nach Kennung sortiert; die eigene Kennung ist nicht enthalten (wie in der Anzeige); für ein Gespräch, das der Aufrufer nicht sehen darf, antwortet die Instanz wie für ein nicht vorhandenes (kein Orakel).
- **Flüchtig, ohne Wecken, ohne Schreibkonto**: wie bei `tippt`; `warten` (I0016) bleibt unberührt.
- **Bremse**: derselbe Mechanismus wie bei `tippt` (10 Vorrat, 1 je Sekunde je Kennung und Gespräch); ein Agent, der Setzen in Schleife aufruft, wird abgewiesen (429), ohne dass seine Nachrichten betroffen sind.

## Nicht-funktionale Anforderungen

- **Last** (Vision „Lastprofil Team-LAN“, PO Zeile 139): 20 Menschen tippen, 50 Agenten melden `denkt nach`, 30 Nachrichten/s. Rechenbeispiel: im ungünstigsten Fall 70 Teilnehmer × 1 Signal je 3 s ≈ 23 Signale/s zusätzlich zu 30 Nachrichten/s. Die Zustellung der Nachrichten bleibt unter 1 s (Messung wie Z10 in R00011: von der 201 bis zum Ereignis beim Leser), das Schreibkonto ist unberührt. Gerechnet in der ADR: Spitze ≈ 46 Signale/s, dauerhaft ≈ 21/s (mit den Enden durch Senden, leeres Feld und Wechsel).
- **Zeit**: Zweck jeder Grenze steht an ihrem Kriterium; Regel und Zustellung sind getrennt (Regel exakt 6 s bzw. Gültigkeit, Anzeige als obere Grenze 30 % darüber); die Uhr startet beim Eingang der Meldung bzw. der Antwort des Setzens.
- **Sicherheit**: Kennung nie vom Client; Zustand und Lesen folgen der Sichtbarkeit der Nachrichten (kein Orakel über fremde Gespräche, auch nicht über Lesen); Wortlaut fest, kein Freitext, also kein Einschleusen von Text in fremde Sichten; Gate `security-reviewer`.
- **Bedienbarkeit** (`ux-reviewer` legt fest): Wortlaut, Platz, Ansage für Bildschirmleser, Touch und schmale Fenster (375 px und 320 px, der Hinweis darf nicht umbrechen und den Verlauf nicht verschieben).

## Akzeptanzkriterien

Gemeinsam: Start mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem Wegwerf-Ordner mit dem Channel `allgemein`; Menschen `anna`, `ben`, `cora` in je einem Browser-Fenster, Agenten `claude` und `codex` über die API. Die Szenarien stehen in [R00045](user-stories/R00045.md).

### I0040 Sehen, wer gerade tippt

- [ ] **Erscheinen** [F0161] (E2E, zwei Browser): `anna` tippt in `# allgemein` → bei `ben` steht binnen 1 s nach dem ersten Tastendruck „anna tippt …“; bei `anna` steht nichts.
- [ ] **Verschwinden sofort** [F0161] (E2E): `anna` sendet die Nachricht → bei `ben` ist die Anzeige binnen 1 s weg und die Nachricht steht im Verlauf; `anna` tippt erneut und löscht das Feld → weg binnen 1 s; `anna` tippt und wechselt in `# technik` → bei `ben` in `# allgemein` weg binnen 1 s.
- [ ] **Verschwinden durch Ablauf** [F0161] (E2E, Rechenbeispiel): `anna` tippt zwei Zeichen (eine Meldung, t = 0 s) und hört auf → die Anzeige gilt bis t = 6 s (die Antwort auf das Setzen nennt den Ablaufzeitpunkt) und ist bei `ben` spätestens bei t = 8 s weg; nie vor Ablauf der Gültigkeit, gemessen über den genannten Zeitpunkt, nicht über die Ankunft.
- [ ] **Takt der Meldungen** [F0161] (E2E, Netzbeobachtung): `anna` tippt 10 s ohne Pause → höchstens ⌈10 / 3⌉ + 1 = 5 Meldungen; ein leeres Feld löst keine Meldung aus; beim Bearbeiten einer eigenen Nachricht (anderes Feld) geht keine Meldung hinaus.
- [ ] **Sichtbarkeit** [F0161; Direktgespräch und Archiv nachgezogen mit I0012 bzw. I0022] (E2E, Integration): Direktgespräch `anna`–`ben`, `cora` erhält kein Ereignis und beim Setzen die Antwort für ein nicht vorhandenes Gespräch (das Lesen für `cora` prüft I0041) (gilt, sobald I0012 grün; bis dahin Unit-Test der Sichtbarkeitsregel); archivierter Channel: keine Anzeige, Setzen und Lesen abgewiesen (gilt, sobald I0022 grün); Seitenleiste und Teilnehmerliste zeigen nie etwas.
- [ ] **Inhaber** [F0161] (Integration): `cora` setzt mit der Kennung `anna@127.0.0.1` im Körper → 422 `regel: "feld"` und keine Wirkung (Lesen von `ben` leer); `cora` setzt ohne Kennung → der Zustand gilt nur für `cora`; `anna` setzt; `cora` löscht → nur ihr eigener Zustand ist weg, das Lesen von `ben` nennt nur `anna`.
- [ ] **Mehrere Namen** [F0162] (E2E): `anna` tippt → „anna tippt …“; `ben` kommt dazu → ein Satz mit beiden Namen; `cora` kommt dazu → „3 Teilnehmer tippen …“ (Wortlaut laut `ux-reviewer`); jeder sieht sich selbst nie.
- [ ] **Platz reserviert** [F0162] (E2E): die Position der letzten Nachricht im Verlauf ist vor, während und nach der Anzeige gleich (gleiche Pixelzeile, Toleranz 0 px); bei 375 px Breite ebenso.
- [ ] **Bildschirmleser** [F0162] (E2E): während 12 s ununterbrochenen Tippens ändert sich der Text der Ansage-Region genau zweimal (Erscheinen, Verschwinden) — Erneuerungen ändern ihn nicht.
- [ ] **Flüchtig** [F0161] (Integration, E2E): während `anna` tippt und danach sind `sha256sum` über alle Dateien unter `channels/` unverändert; `ben` lädt neu → keine Anzeige und kein Eintrag im Verlauf; der Strom von `ben` wird getrennt und neu aufgebaut → nichts wird nachgeholt; im Betriebsprotokoll steht je Signal keine Zeile.
- [ ] **Kein Wecken** [I0041 B0698] (Integration, fahrbar, sobald I0016 gebaut ist; bis dahin trägt es die Bauweise: `warten` abonniert den Signalbus nie): `codex` wartet über `warten`; `anna` tippt 10 s → `warten` kehrt nicht zurück (läuft in seine Zeitgrenze); eine Nachricht von `anna` weckt wie bisher.
- [ ] **Schreibkonto und Quote** [F0163] (Integration): 60 Meldungen von `anna` in 30 s, danach sendet sie 30 Nachrichten in einem Zug → alle 201; `belegt` in `/api/gesundheit` ist unverändert.
- [ ] **Bremse** [F0163] (Integration, Rechenbeispiel): 11 Meldungen von `anna` in `# allgemein` in unter 1 s → zehnmal 200, einmal 429 `grenze: "gespraech"` mit `Retry-After: 1`; die Anzeige bei `ben` bleibt bestehen; nach 1 s geht Setzen wieder (200); danach sendet `anna` → 201; im Betriebsprotokoll steht genau eine Beginn-Zeile `WARNING aktivitaet`; zehn weitere Meldungen in `# technik` gehen durch (anderes Gespräch). 121 Meldungen einer Adresse über 12 Kennungen × 10 Gespräche in einer Sekunde → die 121. mit `grenze: "herkunft"`.
- [ ] **Verlassen und offline eines Menschen** [F0163] (Integration): `anna` (Mensch, mit Strom) tippt, der Strom wird geschlossen → bei `ben` ist die Anzeige binnen 2 s weg; `anna` verlässt (`POST /api/verlassen`) → weg binnen 1 s.
- [ ] **Keine Abschaltung** [F0162] (E2E): die Oberfläche bietet weder je Teilnehmer noch für die Instanz einen Schalter dafür.

### I0041 Als Agent „denkt nach“ anzeigen und lesen

- [ ] **Setzen und Anzeige** [I0041] (Integration, E2E): `claude` setzt `denkt_nach` ohne Gültigkeit → bei `anna` und `ben` steht binnen 1 s „claude denkt nach …“; die Antwort nennt `bis` = `jetzt` + 30 s, Lesen nennt den Eintrag mit demselben `bis`; setzt `claude` `tippt` ohne Angabe, gilt es 6 s.
- [ ] **Nachricht beendet den Zustand** [F0161 für den Mechanismus, I0041 für den Agenten] (E2E): `claude` sendet im Gespräch eine Nachricht → die Anzeige ist binnen 1 s weg und die Nachricht steht im Verlauf; eine Nachricht von `claude` in `# technik` beendet den Zustand in `# allgemein` nicht.
- [ ] **Ablauf** [I0041] (Integration, Rechenbeispiel, gestellte Uhr wo möglich): ohne Erneuerung gilt der Zustand 30 s; Erneuerung bei t = 20 s gilt bis t = 50 s (nicht bis 60 s); Gültigkeit 10 angegeben → bis t = 10 s; 120 → bis t = 120 s; 121 und 500 → 422 `regel: "gueltigkeit"` ohne Wirkung (nicht begrenzt); Uhr rückwärts (Eintrag mit `bis` − `jetzt` größer als die Gültigkeit) → gilt als abgelaufen; Anzeige spätestens 30 % nach Ablauf weg (Standard: 39 s).
- [ ] **Löschen** [F0161 Route, I0041 Agent] (Integration): `claude` löscht → 204, Anzeige binnen 1 s weg, Lesen leer; zweites Löschen antwortet ebenfalls 204.
- [ ] **Inhaber** [I0041] (Integration, Abnahmepunkt 4): `codex` setzt mit der Kennung `claude@127.0.0.1` im Körper → 422 `regel: "feld"`, keine Wirkung; `codex` löscht → nur ein eigener Zustand ist betroffen; der Zustand von `claude` bleibt unverändert sichtbar und lesbar.
- [ ] **Lesen** [I0041] (Integration): `anna` tippt und `claude` denkt nach → `codex` liest beide Einträge (`aktivitaeten` mit `kennung`, `zustand`, `bis`, dazu `jetzt`, nach Kennung sortiert); `claude` liest nur `anna`; ein abgelaufener Eintrag fehlt; Lesen für ein Gespräch, das der Aufrufer nicht sehen darf, antwortet wie für ein nicht vorhandenes — auch für `cora` im Direktgespräch `anna`–`ben` (gilt, sobald I0012 grün); ohne Sitzung 401.
- [ ] **Fehlerfälle** [Zustandswortlaut, `Origin`, Channel: F0161; Gültigkeit: I0041] (Integration): Zustand `"schreibt"`, `"denkt nach"` (mit Leerzeichen), `"Tippt"` → 422 `regel: "zustand"`; `gueltig_s` 0, −1, 1.5, `"30"`, 121 → 422 `regel: "gueltigkeit"`; fremder `Origin` bei `PUT`/`DELETE` → 403; unbekannter Channel → 404; ohne Sitzung → 401; jeweils keine Wirkung (Lesen leer).
- [ ] **Flüchtig, kein Wecken** [I0041] (Integration): Tageslog und Verlauf nach dem Neuladen unverändert (`sha256sum`), kein Eintrag im Nachholen, keine Betriebsprotokollzeile je Signal; ein wartender Agent wird nicht geweckt.
- [ ] **Verlassen, aber nicht offline** [I0041] (Integration, Abnahmepunkt aus P7; Regel F5 offen beim `product-owner`): `claude` tritt mit `art: agent` bei, hat keinen Strom und setzt `denkt_nach` → Lesen von `ben` nennt `claude` auch nach 5 s; `claude` verlässt (`POST /api/verlassen`) → Zustand binnen 1 s weg; ohne Erneuerung endet er mit Ablauf der Gültigkeit.
- [ ] **Last** [I0041 B0698] (Gate, Lastprofil Team-LAN): 20 Menschen tippen, 50 Agenten melden `denkt nach` (Erneuerung alle 20 s), 30 Nachrichten/s über 60 s → Zustellung jeder Nachricht unter 1 s (Maximum, wie Z10 gemessen); Schreibkonto der Senderkennungen unberührt (kein 429 durch Signale); keine Episode `live` und kein `aktivitaet_luecke` bei einem zügigen Leser; im Bericht stehen Leerlauf-p90, p50, p90 und max.

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

Entschieden in [ADR-0051](../Dokumentation/ADRs/0051-aktivitaet-tippt-und-denkt-nach-fluechtig-im-speicher-eigener-signalbus-eine-route-je-gespraech.md): Baustein `src/aktivitaet/` (A00019) mit Tabelle (Kennung, Gespräch) → Zustand und Ablauf im Speicher, eigener Signalbus neben dem Ereignisbus (Nachrichten gehen im Strom vor), Route `PUT`/`DELETE`/`GET /api/channels/{name}/aktivitaet`, Ablauf im Client aus `bis` und `jetzt`, eigene Bremse neben dem Schreibkonto.

## Abhängigkeiten

- Abhängig von: R00011 (Senden, Eingabefeld, Strom), R00004 (Anwesenheit), I0014 Als Agent beitreten (WBS, noch ohne Requirement).
- Nachgezogen, sobald gebaut: I0012 Direktnachricht senden (Sichtbarkeit im Direktgespräch), I0022 Channel archivieren (archivierte Channels).
- Blockiert: nichts.

## Offene Fragen

Aus den Entscheidungen des product-owner nicht beantwortet, vom planer mit Empfehlung gesetzt (der `architekt` bzw. `ux-reviewer` bestätigt oder ändert). Stand nach ADR-0051: Fragen 1–5 sind **entschieden** (1 abweisen mit 422 `gueltigkeit`; 2 ein Zustand je Kennung und Gespräch, ein neues Setzen ersetzt das alte; 3 Erneuerung ab Eingang, nicht addiert; 4 Lesen ohne eigene Kennung; 5 Antwort und Lesen nennen `bis` und `jetzt`), 6 und 7 liegen beim `ux-reviewer`, F5 beim `product-owner`:

1. **Größerer Wert als 120 s** — begrenzen oder abweisen: Empfehlung abweisen (422), `architekt` (Abnahmepunkt 3 lässt beides zu).
2. **Ein Zustand je Kennung und Gespräch**, ein neuer ersetzt den alten (`denkt nach` ersetzt `tippt`) — Annahme des planer, `architekt` bestätigt.
3. **Erneuerung** gilt ab Eingang, keine Addition — Annahme des planer.
4. **Lesen ohne eigene Kennung** — Annahme des planer, passend zu „man sieht sich nie selbst“.
5. **Die Antwort auf das Setzen und die Lese-Antwort nennen den Ablaufzeitpunkt** — braucht der Test für „nie vor Ablauf weg“; Feldnamen `architekt`.
6. **Anzeigename**: Name vor dem `@` wie im Verlauf — `ux-reviewer`.
7. **Beide Zustände gleichzeitig** (ein Mensch tippt, ein Agent denkt nach) — eine oder zwei Zeilen: `ux-reviewer`.
8. **F5 — Bestätigung durch den `product-owner`, nicht entschieden**: beendet `offline` (Strom getrennt, Gnadenfrist abgelaufen) auch den Zustand eines **Agenten**? Gebaut nach der Empfehlung der ADR: nein — für Agenten gilt `weg` (Verlassen), Löschen, eine Nachricht oder der Ablauf, weil ein Agent zwischen zwei Warte-Aufrufen ohne Strom immer offline ist und sonst jedes „denkt nach“ nach einer Sekunde verschwände. Folge: ein Mensch, der per API mit `art: agent` beitritt, „tippt“ nach einem Verbindungsabriss bis zum Ablauf seiner 6 s weiter. Betrifft F0163 (B0694) und I0041 (B0695).

## Warum löst diese Anforderung das Problem?

Das Problem ist die Stille zwischen Absicht und Nachricht: wer wartet, weiß nicht, ob ein Gegenüber schreibt, nachdenkt oder fehlt. Die Anzeige beantwortet genau diese Frage, ohne Gesprächsinhalt zu speichern: Eingabe im Browser bzw. Setzen durch den Agenten → flüchtiger Zustand mit Ablauf → Ereignis im Live-Strom → Anzeige bei den Mitlesenden → Ende mit Nachricht, Löschen oder Ablauf. Dass sie wirklich nichts anderes tut, belegen die Kriterien „flüchtig“, „kein Wecken“ und „Last“.

## Notizen

- Optionen: Die Richtung ist vom product-owner entschieden (Zeilen 135–141, Optionen dort); verworfen wurden Freitext im Status, Abschaltung je Teilnehmer und Wecken durch Signale (jeweils später additiv möglich).
- Referenzen (Verhalten, nichts übernommen): Slack `assistant.threads.setStatus` (https://docs.slack.dev/reference/methods/assistant.threads.setStatus, abgerufen 2026-10-30) — „is thinking…“, endet mit der Antwort, leerem Status oder nach 2 Minuten; Discord Trigger Typing Indicator (https://docs.discord.com/developers/resources/channel, abgerufen 2026-10-30) — verfällt nach 10 s.
- Karte #77: laufende Karten bauen die alte Linie nicht; neuer Umfang.
