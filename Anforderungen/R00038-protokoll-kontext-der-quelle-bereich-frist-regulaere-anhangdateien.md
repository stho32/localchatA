---
id: R00038
status: Neu
datum: 2026-10-29
---

# R00038: Protokoll trägt den Kontext der Quelle, Pflege-Zeile im Bereich frist, Anhangdateien nur regulär öffnen

## Beschreibung

Drei kleine Härtungen aus den Reviews zu #33 und #9 (Karte #66):

- **Zwischen- und Ende-Zeile jeder Episode nennen nur, was für die ganze Episode gilt**: die Schlüssel der Quelle, die Zählungen und zusammengefasste Werte. Werte eines einzelnen Auftretens — die Kennung, die zuerst scheiterte, der Kanal und die Belegung der ersten Abweisung — stehen nur in der Beginn-Zeile. Das gilt jetzt für **alle** Episoden und auch für die Zwischenzeile „läuft weiter — n seit Beginn“; R00022 hatte vier Episoden und die Ende-Zeile geregelt.
- **Die Fehlerzeile „Anhänge eines Tags nicht entfernt“ steht im Bereich `frist`**, nicht in einem eigenen Bereich `pflege`.
- **Eine Anhangdatei wird nur gelesen oder gekürzt, wenn sie eine reguläre Datei ist**: liegt an ihrer Stelle eine FIFO, hängt kein Abruf und kein Löschen.

Zahlt ein auf: [Vision](R00000-vision.md) — „eine gemeinsame LAN-Instanz“ (die Betreiberin liest aus dem Protokoll, was im Betrieb geschieht, ohne falsche Spur; eine hängende Anfrage legt die Instanz nicht lahm).

Herkunft: Karte #66 — code-reviewer #33 Anmerkung 4, observability-reviewer #33, Befund des Auftrags aus Karte #9; entschieden vom architekt in [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) (Ergänzungen „Karte #66“ und „Katalogzeilen der Anhang-Pflege“), [ADR-0044](../Dokumentation/ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) (Ergänzung „Karte #66“) und [ADR-0046](../Dokumentation/ADRs/0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) (Ergänzungen „Karte #66 Teil C“ und „Teil D“).

Slices (WBS `Dokumentation/Planung/localchata.md`): F0129 Zwischen- und Ende-Zeile jeder Episode tragen den Kontext der Quelle (unter I0021 Betriebsprotokoll), F0130 Nicht entfernte Anhänge eines Tags stehen im Bereich frist (unter I0021), F0131 Anhangdateien nur öffnen, wenn sie regulär sind (unter I0010 Anhänge senden und öffnen).

Weiche (planer): F0129 ist reifer als R00022 (die Anforderung nennt die Zwischenzeile nicht und listet vier Episoden); für `quote`, „Reservierung verfallen“ und „Großer Anspruch“ gilt die allgemeine Regel von R00022 („die Ende-Zeile einer Episode trägt …“) schon, sie wurde dort nicht gebaut — hier ausdrücklich zugesagt. F0130 und F0131 sind in keiner Anforderung zugesagt.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00014 und R00022: Betriebsprotokoll, Bereich, Kontext, Episode (Beginn-Zeile, Zwischenzeile, Ende-Zeile), Quelle, Auftreten, Ruhe.

## Geschäftlicher Nutzen

Eine Ende-Zeile mit der Kennung des ersten Auftretens sagt der Betreiberin, ein einzelner Absender sei schuld, wo eine Datei, eine Platte oder eine Herkunft die Ursache ist; bei `WARNING quote` zeigt sie Channel und Belegung der ersten Abweisung für eine Episode über mehrere Channels. Die Zwischenzeile nach zehn Minuten hat denselben Fehler, und sie ist die Zeile, die eine lange Störung begleitet. Ein Bereich `pflege`, den der Katalog nicht kennt, entgeht `grep ' frist '`. Eine FIFO im Anhangbaum hält je Abruf einen Thread fest, ohne Spur im Protokoll. Alle drei sind klein, aber im Betrieb unsichtbar, bis jemand sie braucht.

## Funktionale Anforderungen

### F0129 Zwischen- und Ende-Zeile jeder Episode tragen den Kontext der Quelle (ADR-0042 Ergänzung „Karte #66“)

- Für **jede** Episode tragen Ende- und Zwischenzeile die Schlüssel der Quelle (und was aus ihr folgt), die Zählungen und ausdrücklich zusammengefasste Werte; Werte eines einzelnen Auftretens stehen nur in der Beginn-Zeile.
- Zwischenzeile und Ende-Zeile tragen **denselben Kontext** (aus einer gemeinsamen Funktion, mit dem letzten Auftreten und der größten Bytezahl, falls die Episode eine führt); sie unterscheiden sich im Text („läuft weiter — n seit Beginn“ gegen „Episode endet: n in d s …“) und darin, dass nur die Ende-Zeile die Aufteilung je Art, das Verwerfen und die Dauer nennt. Die Sammelepisode („weitere Quellen“) hat weiter keinen Kontext.
- Festlegung je Episode:

  | Episode | Quelle | Beginn | Ende und Zwischenzeile |
  |---|---|---|---|
  | `quote` Senden abgewiesen | Kennung | `kennung`, `channel`, `belegt`, `hoechstens` | `kennung`, `hoechstens` |
  | `anhang` Reservierung verfallen ohne Hochladen | Herkunft | `herkunft`, `kennung`, `bytes` | `herkunft` |
  | `anhang` Großer Anspruch | Kennung | `kennung`, `herkunft`, `bytes`, `hochladen_bis` | `kennung`, `herkunft` |
  | alle übrigen | wie bisher | wie bisher | Zwischenzeile = Ende-Kontext (`senden` ohne `kennung`, `körper` ohne `kennung`, `schreibkonto` ohne `grenze`, `zeit` mit `dauer_ms` des letzten) |

- Der Entwickler listet im Bericht **alle** Aufrufer von `Protokoll::episode` mit der Einordnung je Schlüssel (Quelle oder Einzelwert); jeder weitere Fund nach dieser Regel wird in derselben Karte behoben, nicht neu entschieden.

### F0130 Nicht entfernte Anhänge eines Tags stehen im Bereich frist (ADR-0042 Ergänzung „Katalogzeilen der Anhang-Pflege“)

- Die ERROR-Zeile „Anhänge eines Tags nicht entfernt — erneuter Versuch bei der nächsten Pflege“ steht im Bereich `frist` mit `channel`, `tag`, `grund` (Anlass), `ursache` (Fremdtext); der Bereich `pflege` entfällt. Die WARNING `laden` „Anhänge ohne Tagesdatei“ bleibt unverändert (`ordner`, `tag`, `anzahl`, `bytes`).

### F0131 Anhangdateien nur öffnen, wenn sie regulär sind (ADR-0046 Ergänzung „Karte #66 Teil D“)

- Ausliefern (Lesen) und Löschen (Kürzen) öffnen eine Anhangdatei ohne zu blockieren und weisen eine Datei, die keine reguläre ist (FIFO, Gerät, Socket), wie eine unlesbare ab: kein neuer Statuscode, der vorhandene Fehlerpfad gilt (Ausliefern: wie ein Lesefehler der Datei; Löschen: wie ein Kürzfehler, Wiederholung bei der nächsten Pflege). Gilt für jedes Öffnen einer Datei unter `anhaenge/<tag>/`, das nicht neu anlegt. Nur Unix.

## Nicht-funktionale Anforderungen

- **Sicherheit**: nie Nachrichtentext, Token oder Körper im Protokoll (R00014); die FIFO-Abwehr braucht keine neue Katalogzeile, der Fall erscheint als Fehlerzeile der jeweiligen Operation.
- **Zeitkriterien** nach `AI-Team/planer.md`: (1) „Abruf und Löschen einer FIFO enden binnen 2 s“ — Zweck: kein Thread hängt (vor der Härtung unbegrenzt), erwartet wenige Millisekunden (ein `open` mit `O_NONBLOCK` und ein `fstat`), 2 s bleiben weit über dem Erwarteten; die Uhr startet mit dem Senden der Anfrage. (2) „Unit auf eine FIFO ohne Gegenstelle: `Err` binnen 1 s“ — gleicher Zweck; die Uhr startet mit dem Aufruf der Funktion in einem eigenen Thread. (3) Die Zwischenzeile entsteht nach 10 Minuten und 1 Sekunde und wird nur als Unit mit gestellter Uhr geprobt.
- **Konsole**: die Härtungen berühren keine Oberfläche.

## Akzeptanzkriterien

Gemeinsam: Start, `S(<name>)` und Speicher-Senke wie in R00014 und R00022; „Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel.

### F0129 Zwischen- und Ende-Zeile jeder Episode tragen den Kontext der Quelle

- [ ] **Zwischenzeile des Schreibfehlers** (Unit, Speicher-Senke, gestellte Uhr; wörtlich aus ADR-0042): Schreibfehler-Episode von `alpha@10.0.0.9` und `beta@10.0.0.9` (gleiche Datei, gleicher Grund) über 10 Minuten und 1 Sekunde → genau eine Zeile „… läuft weiter — n seit Beginn“ mit `channel=`, `datei=`, `grund=` und **ohne** `kennung=`; die Beginn-Zeile trägt `kennung=alpha@10.0.0.9`. Heute rot (`kennung=alpha@10.0.0.9` steht in der Zwischenzeile).
- [ ] **Zwischenzeile der übrigen Episoden** (Unit, gleiche Anlage): `körper` ohne `kennung=`; `schreibkonto` ohne `grenze=`; für den Vorlauf trägt die Zwischenzeile `dauer_ms` des letzten Auftretens (Rechenbeispiel: 61.001 ms, dann 7.410.002 ms → `dauer_ms=7410002`).
- [ ] **Ende-Zeile `quote`** (Unit): `alpha@10.0.0.9` in `a` (`belegt=100`), danach in `b` (`belegt=200`), `hoechstens=1000` → Beginn mit `channel=a belegt=100`; Ende `… Episode endet: 2 in <d> s | kennung=alpha@10.0.0.9 hoechstens=1000` ohne `channel=` und `belegt=`.
- [ ] **Ende-Zeile „Reservierung verfallen“** (Unit): zwei verfallene Anmeldungen von `10.0.0.9` (Kennungen `alpha`, `beta`; `bytes` 5 und 6) → Ende nur `herkunft=10.0.0.9`; die DEBUG-Zeile „Anmeldung verfallen“ je Ablauf trägt weiter alle drei Schlüssel.
- [ ] **Ende-Zeile „Großer Anspruch“** (Unit): zwei Anmeldungen derselben Kennung (`bytes` 5 GiB und 6 GiB) → Ende `kennung=… herkunft=…` ohne `bytes=` und `hochladen_bis=`.
- [ ] **Großer Anspruch im echten Programm** (Integration, `LOCALCHATA_EPISODE_RUHE_MS=200`): zwei Anmeldungen derselben Kennung über der Grenze (`z(bytes) ≥ Q/4`) mit verschiedenen `bytes` → eine Beginn-Zeile mit `bytes=` der ersten, eine Ende-Zeile ohne `bytes=` und `hochladen_bis=`.
- [ ] **Mutanten rot**: „Kontext des ersten Auftretens“ in der Ende-Zeile; „Zwischenzeile nimmt `self.kontext`“; `kennung` bei `quote` mit `mit_nur_beginn` (Quelle fehlt im Ende); `herkunft` bei „Großer Anspruch“ mit `mit_nur_beginn` (Folge der Quelle fehlt).
- [ ] **Bestandsaufnahme**: der Bericht des Entwicklers nennt jeden Aufrufer von `Protokoll::episode` (Stand der Planung: 25 Stellen in 16 Dateien) mit Einordnung je Schlüssel; zu jeder behobenen weiteren Stelle gehört ein Test „zwei Auftreten verschiedener Werte“ mit roter Mutante.
- [ ] **Unverändert grün**: die Kriterien aus R00014, R00018, R00022 (Beginn-Zeile, Zählungen, Aufteilung je Art).

### F0130 Nicht entfernte Anhänge eines Tags stehen im Bereich frist

- [ ] **Bereich frist** (Integration; wörtlich aus ADR-0042): Verkürzen der Frist bei schreibgeschütztem `anhaenge/<tag>` → genau eine Zeile `ERROR frist Anhänge eines Tags nicht entfernt — erneuter Versuch bei der nächsten Pflege | channel=… tag=… grund=… ursache=…`; `grep ' pflege '` im Protokoll ist leer. Mutante „Bereich pflege“ → rot. Der Aufzählungswert `Pflege` ist aus dem Code entfernt.

### F0131 Anhangdateien nur öffnen, wenn sie regulär sind

- [ ] **Abruf einer FIFO** (Integration, echtes Programm, Wegwerf-Ordner, nur Unix; wörtlich aus ADR-0046): ein gebundener Anhang, dessen Datei vor dem Abruf durch `mkfifo` gleichen Namens ersetzt wird → `GET /anhaenge/<id>` antwortet **binnen 2 s** mit dem Fehlerstatus des Lesefehlers (nicht 200, kein Hängen); ein zweiter Abruf ebenso; `GET /api/gesundheit` bleibt erreichbar.
- [ ] **Löschen einer FIFO** (Integration): `POST /api/anhaenge/loeschen` für dieselbe ID endet binnen 2 s; die Antwort nennt den Fehler, `anhang_belegt` ist unverändert.
- [ ] **Unit `anhangpfade`** (Wegwerf-Ordner): `zum_lesen` und `kuerzen` auf eine FIFO ohne Gegenstelle → `Err` binnen 1 s (Test mit Thread und Zeitgrenze); auf eine reguläre Datei unverändert `Ok`.
- [ ] **Mutanten rot**: „ohne `NONBLOCK`“ → der Test läuft in die Zeitgrenze; „ohne `S_ISREG`“ → `zum_lesen` auf die FIFO liefert `Ok`.

## Betroffene Verzeichnisstruktur

- `src/protokoll/episoden.rs` (gemeinsame Funktion für Ende und Zwischenzeile), `src/http/senden.rs` (`quote_melden`), `src/start/mod.rs` (`verfall_melden`), `src/http/anhaenge/anmelden.rs` (`anspruch_melden`) und die übrigen Aufrufer aus der Bestandsaufnahme.
- `src/gespraeche/fristeintrag.rs`, `src/protokoll/zeile.rs` (Bereich).
- `src/gespraeche/anhangpfade.rs` (zwei Funktionen, ein Helfer), ggf. `src/gespraeche/anhangablage.rs`.
- Tests spiegeln die Themen: `tests/episoden.rs`, `tests/frist_protokoll.rs`, `tests/anhaenge_ausliefern.rs`, `tests/anhaenge_loeschen.rs`, Unit in den Modulen.
- `Dokumentation/Architektur/A00015-betriebsprotokoll.md`: Beschreibung der Zusammenfassung, falls dort die Zwischenzeile mit dem Beginn-Kontext steht.

## Technische Überlegungen

### Ablauf

1. **Zwischen- und Ende-Zeile** (B0588): Test „Zwischenzeile ohne Beginn-Werte“ zuerst rot; private Funktion `kontext_der_zusammenfassung()` (`fuer_ende(letzter)` plus größte Bytezahl) in `ende` und `zwischenzeile`.
2. **Quellen einordnen** (B0589, B0590): `quote_melden`: `channel`, `belegt` mit `mit_nur_beginn`; `verfall_melden`: `kennung`, `bytes`; `anspruch_melden`: `bytes`, `hochladen_bis`.
3. **Bestandsaufnahme** (B0591): alle Aufrufer einordnen, Funde beheben.
4. **Bereich** (B0592): `Bereich::Pflege` entfällt, `fristeintrag.rs` nimmt `Bereich::Frist`.
5. **Reguläre Dateien** (B0593): `OFlags::NONBLOCK` und `fstat` mit `S_ISREG` hinter gemeinsamem Helfer; Nicht-Datei → `io::Error` (`InvalidInput`, „keine reguläre Datei“).

Reihenfolge: 1 vor 2 bis 3; 3 zuletzt; 4 und 5 unabhängig; alle erst nach dem Merge von #9 Teil B (gleiche Dateien).

### Grobentwurf

Keine neuen Typen von außen sichtbar; Option D der ADR (Typ erzwingt die Einordnung durch getrennte Felder `quelle` und `einzeln`) ist Wiedervorlage, wenn ein weiterer Fund dieser Klasse auftritt.

## Aufwand (Zählung)

Gezählt aus der WBS (6 Bubbles); die Zahlen sind Richtwerte, ungemessen für diese Bubble-Typen.

- F0129: 4 Bubbles · B0588 2 h, B0589 0,4 h, B0590 0,8 h, B0591 unklar 2–4 h (wie viele Funde die 25 Aufrufer ergeben)
- F0130: 1 Bubble · 0,4 h
- F0131: 1 Bubble · 2 h
- Summe 7,6 bis 9,6 h bei sechs Bubbles in drei Slices.

## Warum löst diese Anforderung das Problem? (Pflicht)

- Die gemeinsame Funktion lässt die Zwischenzeile den Fehler nicht neu erzeugen, weil sie denselben Kontext nutzt wie die Ende-Zeile; deshalb steht keine Kennung des ersten Auftretens mehr in einer Zeile, die für die ganze Episode spricht.
- Die Einordnung je Episode und die Bestandsaufnahme der Aufrufer schließen die drei bekannten Fälle und suchen weitere, bevor ein Review sie findet.
- Bereich `frist` führt die Fehlerzeile in dasselbe Suchwort wie alles, was das Entfallen tut.
- Das Öffnen ohne Blockieren, mit Prüfung am gehaltenen Deskriptor, nimmt der FIFO die Wirkung: kein Thread hängt, und es gibt keine Rennbedingung zwischen Prüfen und Lesen.

## Offene Fragen

- Keine. Das Muster „vergessen einzuordnen“ bleibt möglich, bis Option D kommt (in Kauf genommen, ADR-0042).

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00014 und R00022.

## Notizen

- **Zusagen aus R00014, R00018 und R00022 bleiben**; Tests, die `kennung=`, `channel=`, `belegt=`, `bytes=` oder `hochladen_bis=` in einer Ende-Zeile erwarten, ziehen mit.
- **Katalogzeilen nachtragen** (Architekt, nach dem Bau): die Zeilen der drei Episoden in ADR-0042 um „Ende:“ ergänzen; dazu gehört kein Auftrag an den Entwickler.
- Verworfen (Optionen des architekt): Beginn-Wert als Absicht belassen und nur die Zwischenzeile korrigieren (Ende von `quote` bliebe falsch); Ende- und Zwischenzeile ganz ohne Kontext (wer wäre nicht mehr zu sehen); Typ erzwingt die Einordnung (Umbau von 25 Stellen für drei Fehler, Wiedervorlage); Bereich `pflege` behalten (ein Suchwort mehr, bricht die Form von `frist`); benannte Grenze statt Härtung für FIFOs; `spawn_blocking` mit Frist um das Öffnen (der Thread bliebe hängen).
