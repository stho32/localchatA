# ADR-0004: Logformat — Kopfzeile und eingerückter Text

**Status**: Akzeptiert
**Datum**: 2026-10-27
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #1 I0001)
**Kontext-Anforderung**: — (WBS I0002, I0017)
**Erprobt durch**: Wegwerf-Probe im Scratchpad: Schreiben und Lesen von 100.000 Sätzen (53 MB, 5 % mit Codeblöcken) in diesem Format, Laden 0,43 s (siehe ADR-0006)

## Kontext

Jede Nachricht steht mit Absender und Zeit im Log ihres Channels; Nachrichten enthalten Markdown, also beliebige Zeilen, Leerzeilen, Codeblöcke; das Log soll ohne localchatA lesbar sein (Vision). Gefordert ist Zeilensicherheit: keine Nachricht darf das Ende einer anderen vortäuschen oder sie zerreißen. Daneben braucht ein Satz Metadaten (Antwort-Bezug, Anhänge mit Größe und Typ, bei Bildern Breite × Höhe für reservierten Platz laut Designsystem).

## Optionen

### Option A: JSON Lines — ein JSON-Objekt je Zeile
- Vorteile: zeilensicher durch Konstruktion; erweiterbar; `serde_json`; `jq` liest es.
- Nachteile: Markdown erscheint als eine Zeile mit `\n`-Folgen — mit `less` kaum lesbar; Ziel Q5 nur mit Werkzeug erfüllt.

### Option B: Kopfzeile + eingerückter Text (Einrückungsregel wie bei E-Mail-Kopfzeilen oder YAML-Blöcken)
````text
@ 2026-10-27T14:07:12.345+01:00 claude@10.0.0.5
+ antwort: 2026-10-27T14:05:01.002+01:00
+ anhang: anhaenge/2026-10-27/140712345-1-bericht.pdf 2457600 application/pdf
  Der Scan ist durch. **Zwei Befunde:**

  ```rust
  @ auch das hier ist nur Text
  ```

````
- Vorteile: mit `less`/`cat` lesbar wie ein Chat-Protokoll; zeilensicher, weil jede Textzeile mit zwei Leerzeichen beginnt und nur `@ ` in Spalte 1 einen Satz beginnt; Metadaten als `+ schluessel: wert` erweiterbar.
- Nachteile: eigene kleine Grammatik (Parser selbst schreiben und testen); Text um zwei Zeichen je Zeile größer; Hand-Editoren, die Leerzeichen am Zeilenende entfernen, sind harmlos, Editoren, die Einrückung ändern, nicht.

### Option C: Kopfzeile mit Bytelänge, dann Text roh (`@ <zeit> <von> <bytes>`)
- Vorteile: Text bleibt unverändert; Abschneiden durch Absturz erkennbar.
- Nachteile: von Hand nicht schreib- oder reparierbar; eine falsche Länge verdirbt den Rest der Datei.

### Option D: Markdown-Datei mit Überschrift je Nachricht (`### 14:07 claude@…`)
- Vorteile: am schönsten lesbar, jeder Markdown-Betrachter zeigt es.
- Nachteile: nicht zeilensicher — eine Nachricht mit `### ` sieht aus wie eine neue.

## Entscheidung

**Gewählt: Option B**

Sie erfüllt Lesbarkeit (Q5) ohne Werkzeug und ist zeilensicher; die Probe zeigt, dass sie beim Laden nicht bremst. Grammatik:

- Satzbeginn: Zeile beginnt mit `@ `; Felder durch ein Leerzeichen: Zeit (RFC 3339, Millisekunden, Versatz — zugleich die ID, ADR-0005), Kennung `name@herkunft` (ohne Leerzeichen).
- Metazeilen direkt danach: `+ <schluessel>: <wert>`; unbekannte Schlüssel werden gelesen und erhalten (vorwärtsverträglich).
- Textzeilen: zwei Leerzeichen + Zeile; leere Textzeile = leere Zeile; `\r\n` wird `\n`; abschließende Leerzeilen der Nachricht entfallen; Tabs bleiben.
- Ein Satz endet vor der nächsten `@ `-Zeile oder am Dateiende; der Schreiber setzt nach jedem Satz eine Leerzeile.
- Eine Zeile, die weder `@ `, `+ `, zwei Leerzeichen noch leer ist, ist **defekt**: sie wird beim Laden übersprungen, mit Datei und Zeile gemeldet (stderr, Health-Check), nie still geschluckt.

In Kauf genommen: eine eigene Grammatik — dafür ist [A00007 Logformat](../Architektur/A00007-logformat.md) ein reiner, vollständig unit-getesteter Baustein.

## Konsequenzen

**Positiv:**
- Ein Mensch liest den Verlauf eines Tages mit `less`; `grep -B3` findet Absender und Zeit zu einem Treffer.

**Negativ / Inkauf zu nehmen:**
- Ein Absturz mitten im `write_all` eines Satzes kann einen verkürzten letzten Satz hinterlassen, der nicht als verkürzt erkennbar ist (nur bei Stromausfall/Kernel-Absturz realistisch, ein Satz ist ein Schreibaufruf).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erwähnungen werden nicht gespeichert, sondern aus dem Text gelesen (`@name` als Wort) — Regel für den `planer` bei I0013.

## Quellen / Bezuege

- [A00007 Logformat](../Architektur/A00007-logformat.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md); ADR-0003, ADR-0005, ADR-0006
- Vision, Zielbild „Das Log ist das Gedächtnis“, „Markdown in Nachrichten“; Designsystem README „Leistung als Gestaltung“ (Bildmaße aus Metadaten)

## Ergänzungen

- **2026-10-28 (Vorlauf „Betriebsprotokoll“) — stderr wird Betriebsprotokoll:** Regel „defekte Zeile … gemeldet (stderr, Health-Check)“: `WARNING laden` je Zeile mit `datei` und `zeile`, je Datei höchstens 10 einzeln, dann eine Sammelzeile ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regeln 2 und 3; Format [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), stderr spiegelt ab Warning). Die übrigen Regeln dieser ADR gelten unverändert.
- **2026-10-29 (Karte #9 I0010, Anhänge; [ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) akzeptiert) — Metazeile `+ anhang:` und Satz ohne Text:** Das Beispiel „Pfad, Bytes, Typ“ wird zur festen Grammatik `+ anhang: <id> <bytes> <art> <breite>x<hoehe>|- <datei> <anzeigename>` — eine Zeile je Anhang, in Sendefolge, nach `+ art:` ([ADR-0046](0046-anhaenge-ablegen-eingang-bindung-beim-senden-tag-der-nachricht-quote-und-eigenes-konto.md) Regel 7). Eine `+ anhang:`-Zeile, die der Grammatik nicht folgt, ist **kein** Defekt im Sinn dieser ADR (Metazeilen bleiben erhalten), der Anhang fehlt aber im Anhangverzeichnis; `WARNING laden` mit Datei und Zeile. Neu ist ein **Satz ohne Textzeile**: mit mindestens einer `+ anhang:`-Zeile ist er gültig und trägt leeren Text (Betreiber B5, 2026-10-29) — die Grammatik oben erlaubt ihn schon (Metazeilen, dann der nächste Satz); erster Test (Unit, Logformat): Kopfzeile, `+ art: mensch`, `+ anhang: …`, Leerzeile, nächster Satz → zwei Sätze, der erste mit Text `""` und einem Anhang. Die übrigen Regeln gelten unverändert.
