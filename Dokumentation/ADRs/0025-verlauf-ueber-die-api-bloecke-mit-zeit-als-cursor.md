# ADR-0025: Verlauf über die API — `GET /api/channels/<name>/nachrichten` in Blöcken, Nachrichtenzeit als Cursor in beide Richtungen

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #7 I0008)
**Kontext-Anforderung**: — (WBS I0008; prägt I0009, I0010, I0011, I0012, I0016, I0017, I0018)
**Erprobt durch**: Wegwerf-Probe im Scratchpad (2026-10-28): HTTP-Attrappe mit 100.000 Nachrichten wie `tests/leistung.rs` (10–90 Wörter, jede zwanzigste mit 45-Zeilen-Codeblock), Blöcke zu 100 über `vor`/`nach` — Antwort 63 KB JSON je Block; im Browser 199 Blöcke rückwärts und 10 Sprünge an den oberen Rand ohne Lücke und ohne Doppel; Index-Kosten aus der Probe zu ADR-0006 (letzter Block zu 100 in 0,06 ms, `--release`). Probe danach gelöscht.

## Kontext

- Fertig-Kriterium I0008 (WBS Zeile 162): „Auswahl zeigt den Verlauf mit Absender und Zeit; bei 100.000 Nachrichten öffnet der Channel sofort und scrollt flüssig.“ Vision „Schnell, auch bei großem Verlauf“; Qualitätsziel Q1 in [A00001](../Architektur/A00001-localchata.md): neueste 100 in < 50 ms Server-Zeit, jeder ältere Block ebenso.
- Designsystem „Leistung als Gestaltung“: Blöcke zu 100, nach oben verankert nachladen; `VerlaufsMarken`: Tagestrenner, „Zum Neuesten“, Nachladen ab etwa zwei Bildschirmhöhen vor dem Rand; `1-tastatur.md`: `Pos1` an den Anfang, `Strg G` Sprung an ein Datum; `components/Nachricht`: der Verbraucher liefert Absender (Kennung), Zeit, Markdown.
- [ADR-0005](0005-programm-als-einziger-schreiber-und-reihenfolge.md): die Nachrichtenzeit ist ID und global streng steigend (`max(jetzt, letzte + 1 ms)`, beim Start fortgesetzt) — gebaut in `Gespraeche::zeit_vergeben` (`src/gespraeche/mod.rs`).
- [ADR-0006](0006-verlaufsindex-im-speicher.md), [A00009](../Architektur/A00009-verlaufsindex.md): je Gespräch eine `BTreeMap<Tag, {Rohtext, Satzanfänge}>` (`src/gespraeche/verlaufsindex.rs`); `block` ist geplant, aber nicht gebaut.
- [ADR-0003](0003-ablage-layout-im-ordner.md): eine Logdatei je Tag nach dem lokalen Datum des Servers; ab I0018 entfallen alte Tage, während jemand liest.
- [ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md): Metazeilen `+ schluessel: wert` kommen später (Antwort, Anhänge); unbekannte werden erhalten.
- [ADR-0020](0020-channelname-ist-ordnername-erlaubnisliste.md): der Channelname ist `[a-z0-9-]` — ohne Kodierung als Pfadsegment brauchbar. Lesende Routen brauchen die Sitzung (A00004, Ergänzung #3), keine Origin-Prüfung.

## Optionen

### Option A: Blöcke über die Nachrichtenzeit als Cursor — `vor=<zeit>` (ältere) oder `nach=<zeit>` (neuere), ohne Cursor die neuesten
- Vorteile: der Cursor ist die schon vorhandene, über Neustarts stabile ID; er muss nicht existieren (exklusive Grenze), überlebt also das Entfallen eines Tages und gilt für Browser und wartende Agenten gleich (ADR-0007 `seit`); Tageswechsel sind ein Weitergehen in der `BTreeMap`; beide Richtungen tragen ein Fenster, das unten abwirft ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)), und später den Sprung an ein Datum oder an die Ungelesen-Linie ohne neue Mechanik.
- Nachteile: keine Position „Nachricht 37.412 von 100.000“ — ein Scrollbalken über den ganzen Verlauf und ein Sprung „auf 40 %“ sind so nicht möglich; zwei Sätze mit gleicher Zeit (nur von Hand möglich) würden an einer Blockgrenze einer übersprungen.

### Option B: Position als Offset (`ab=<index>&n=100`, gezählt vom Neuesten oder vom Ältesten)
- Vorteile: Gesamtzahl und Position bekannt — erlaubt einen echten Scrollbalken über alle Nachrichten (vollständige Virtualisierung).
- Nachteile: Positionen wandern — vom Neuesten gezählt mit jeder neuen Nachricht, vom Ältesten gezählt mit jedem entfallenden Tag (I0018); Doppel und Lücken zwischen zwei Abrufen; braucht eine Präfixsumme über die Tage; für Agenten ohne Wert.

### Option C: Ein Tag je Abruf (`tag=2026-10-27`), passend zur Logdatei
- Vorteile: Tagestrenner und Logdatei fallen zusammen; Abruf ist ein Stück Rohtext; einfach zu erklären.
- Nachteile: ein Tag hat beliebig viele Nachrichten — 100.000 in sieben Tagen sind über 14.000 je Tag, rund 9 MB JSON; „öffnet sofort“ hängt dann am Tagesvolumen; leere Tage brauchen Extra-Abrufe.

### Option D: Seitenweise mit undurchsichtigem Token (`weiter=<token>`)
- Vorteile: Server frei, die Form später zu ändern; üblich in öffentlichen APIs.
- Nachteile: ein zweites Kennzeichen neben der Zeit, das Agenten nicht aus einer Nachricht bilden können; kein Sprung an ein Datum; für ein lokales Programm ohne Versionsdruck tote Flexibilität.

| # | „öffnet sofort“ (Q1) | über Tage / Entfallen | beide Richtungen, Datumssprung | Agenten (`seit`) | Schwäche |
|---|---|---|---|---|---|
| A | ja, 100 je Abruf | robust (exklusive Grenze) | ja | dieselbe Zeit | keine Position im Ganzen |
| B | ja | Doppel/Lücken | Position ja, Datum nein | nein | wandernde Positionen |
| C | nein bei vollen Tagen | natürlich | Datum ja | nein | Tagesvolumen unbegrenzt |
| D | ja | robust | nur vorwärts/rückwärts | nein | zweites Kennzeichen |

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass ein einziges Kennzeichen — die Nachrichtenzeit — Verlauf, Nachladen in beide Richtungen, späteren Datumssprung, Live-Abgleich ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md)) und das Warten der Agenten trägt, ohne dass sich etwas verschiebt, wenn Nachrichten kommen oder Tage entfallen. In Kauf genommen: es gibt keinen Scrollbalken über den ganzen Verlauf ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)), und Sätze mit gleicher Zeit (nur bei Handarbeit an den Logs, ADR-0005) können an einer Blockgrenze fehlen.

Regeln:

1. **Route** `GET /api/channels/{name}/nachrichten` mit Sitzung (Extraktor `Teilnehmer`; ohne 401). Dieselbe Ressource nimmt ab I0009 mit `POST` neue Nachrichten an. Unbekannter oder ungültiger Name → 404 `{fehler}` („`# name` gibt es nicht.“). Jeder mit Sitzung liest jeden Channel (keine Rechte, [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md); alle sehen alle Channels, Director 2026-10-28).
2. **Abfrage** — höchstens einer der Cursor:
   - ohne Cursor: die neuesten `n`;
   - `vor=<zeit>`: die `n` jüngsten mit Zeit **echt kleiner** als `zeit`;
   - `nach=<zeit>`: die `n` ältesten mit Zeit **echt größer** als `zeit` (der Anfang des Channels ist `nach` mit einer Zeit vor dem ältesten Tag);
   - `n`: ganze Zahl **1 bis 200**, Standard 100 (Block des Designsystems).
   Die Zeit ist RFC 3339 mit Versatz, verglichen als Zeitpunkt, nie als Zeichenkette ([X00007](../Architektur/Aspekte/X00007-zeit.md)); der Cursor muss zu keiner Nachricht gehören. Im Query ist `+` als `%2B` zu kodieren (`encodeURIComponent`) — ein unkodiertes `+` wird zum Leerzeichen und ergibt 422. Beide Cursor, ein nicht lesbarer Cursor, `n` außerhalb 1–200 oder keine Zahl → 422 `{fehler, regel}` mit `regel` = `cursor` bzw. `n`; abgewiesen, nicht umgeformt (Hausregel aus ADR-0013).
3. **Antwort** 200 `{channel, nachrichten, aelter, neuer}`:
   - `nachrichten` in Zeitfolge, **älteste zuerst** — in beiden Richtungen gleich, damit der Empfänger einen Block nur vorne oder hinten anfügt;
   - je Nachricht `{zeit, von, text}`: `zeit` wie im Log (RFC 3339 mit Millisekunden und dem Versatz des Servers) und zugleich die ID; `von` die Kennung `name@herkunft`; `text` der Markdown-Text ohne die Einrückung des Logs (`logformat::text`), ungültiges UTF-8 ersetzt (`from_utf8_lossy`);
   - `aelter` / `neuer`: ob es vor der ersten bzw. nach der letzten gelieferten Nachricht im Channel (in der Frist) noch eine gibt. Leerer Channel: `nachrichten: []`, beide `false`.
   - Später kommen Felder **hinzu**, nie wird eines umgedeutet: `antwort` (Zeit der zitierten Nachricht), `anhaenge` (I0010), weitere Metazeilen aus ADR-0004; der Empfänger übergeht unbekannte Felder. Ob die Art des Absenders (Mensch/Agent) mitkommt, entscheidet I0009 (Empfehlung: Metazeile `+ art:` beim Senden; ältere Nachrichten `null`).
4. **Tage**: der Index geht von dem Tag, in dem der Cursor liegt, über die `BTreeMap` zum vorigen bzw. nächsten vorhandenen Tag weiter, bis `n` erreicht oder die Frist erschöpft ist — ein Block kann über beliebig viele (auch leere) Tage reichen. In einem Tag sind die Satzanfänge nach Zeit geordnet (einziger Schreiber, steigende Zeit, ADR-0005); die Grenze findet eine binäre Suche über die Kopfzeiten (O(log n) Kopfzeilen lesen). Defekte Zeilen sind beim Laden übergangen (ADR-0004) und erscheinen nie. Tagestrenner bildet der Empfänger aus `zeit`.
5. **Sperren**: der Handler nimmt die Sperre des Verzeichnisses lesend nur zum Nachschlagen (`Arc<Mutex<Verlauf>>`), dann die Sperre des Gesprächs nur zum Herauskopieren der Sätze; JSON entsteht ohne Sperre. Die Arbeit läuft über `spawn_blocking` (A00004), weil `senden` die Gesprächssperre während eines Plattenschreibens hält (ADR-0005).
6. **Kein Stand, keine Nummer**: der Verlauf hat keinen `stand` wie Teilnehmer und Channels — die Nachrichtenzeit ordnet ihn ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md)).
7. **Direktgespräche** (I0012) bekommen dieselbe Abfrage unter eigener Ressource; die Form der Antwort bleibt.

## Konsequenzen

**Positiv:**
- Öffnen kostet einen Abruf zu 100 Nachrichten (≈ 63 KB) unabhängig von der Länge des Verlaufs; jedes Nachladen ebenso.
- Ein Agent liest mit `curl … /nachrichten?nach=<zeit>` nach, was seit seiner letzten Nachricht geschah — dieselbe Form wie der spätere Warte-Aufruf.
- `Pos1`, `Strg G` und die Ungelesen-Linie brauchen später nur einen Cursor, keine neue Route.

**Negativ / Inkauf zu nehmen:**
- Keine Gesamtposition; ein Sprung „an 40 %“ des Verlaufs ist nicht vorgesehen.
- `+` im Versatz muss kodiert werden — ein typischer Fehler bei Agenten mit `curl`; die 422-Antwort sagt es.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … beim Hochscrollen an einer Tagesgrenze Nachrichten fehlen oder doppelt stehen. Früh-Indikator: Unit-Test über drei Tage mit Block-Grenze genau am Tageswechsel. Gegenmaßnahme: exklusive Grenzen in beiden Richtungen (Regel 2), Test zuerst (unten).
2. … ein Agent den Cursor als Zeichenkette vergleicht und Nachrichten mit anderem Versatz (Sommer-/Winterzeit) falsch ordnet. Früh-Indikator: Test über die Zeitumstellung. Gegenmaßnahme: Server vergleicht als Zeitpunkt; die Anleitung für Agenten (I0019) nennt die Regel.
3. … der Abruf unter Last blockiert, weil `senden` die Gesprächssperre beim Plattenschreiben hält. Früh-Indikator: Antwortzeit des Verlaufs steigt beim gleichzeitigen Senden (Messung mit I0009). Gegenmaßnahme: Regel 5 (kurz kopieren, `spawn_blocking`); bei Bedarf Umbau zu `RwLock` je Verlauf innerhalb von A00009.

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Verlaufsindex, rein): drei Tage zu je 250 Sätzen; `block(Neueste, 100)` → die letzten 100 des jüngsten Tages, älteste zuerst, `aelter: true`, `neuer: false`; `block(Vor(zeit des 50. Satzes des jüngsten Tages), 100)` → 49 vom jüngsten und 51 vom mittleren Tag ohne Lücke; `block(Nach(zeit des letzten Satzes des ältesten Tages), 100)` → die ersten 100 des mittleren Tages; Cursor zwischen zwei Sätzen (kein Satz mit dieser Zeit) trifft die exklusive Grenze; `n` 1 und 200 → so viele, `Nach(sehr früh)` → die ersten `n` des Channels.
- [ ] Erster Integrationstest (echtes Programm, 100.000 Nachrichten in **einem** Channel über 7 Tage, Generator wie `tests/leistung.rs`): `GET …/nachrichten` → 100 Nachrichten, letzte = jüngster Satz, `aelter: true`; 1.000 Blöcke rückwärts über `vor` ergeben genau die 100.000 Zeiten streng steigend ohne Doppel; `n=0`, `n=201`, `n=abc`, `vor` und `nach` zugleich, `vor=2026-10-27T14:07:12.345+01:00` mit unkodiertem `+` → je 422 (Wertprüfung als Spanne: `n=1` und `n=200` → 200, Mutanten 199/201 und 0/2 werden rot); unbekannter Channel → 404; ohne Sitzung → 401.
- [ ] Leistung als Gate: siehe [ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md), Regel 7.

## Quellen / Bezuege

- [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00007](../Architektur/Aspekte/X00007-zeit.md)
- ADR-0003, ADR-0004, ADR-0005, ADR-0006, ADR-0007 (`seit`), ADR-0013 (abweisen statt umformen), ADR-0020
- Vision R00000 „Schnell, auch bei großem Verlauf“; WBS I0008; Designsystem `README.md` („Leistung als Gestaltung“), `components/VerlaufsMarken/README.md`, `components/Nachricht/README.md`, `1-tastatur.md`; `design-prompt.md` Abschnitt 5
