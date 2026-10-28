# ADR-0036: Schreibkonto gegen Flut — Token-Eimer je Kennung und je Herkunft, Kosten nach Größe, 429 mit Wartezeit; zuerst für das Senden

**Status**: Akzeptiert
**Datum**: 2026-10-28
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #8 I0009)
**Kontext-Anforderung**: — (WBS I0009; Empfehlung für I0018 und künftige Schreibrouten unter „Folgeentscheidungen“)
**Erprobt durch**: — (gerechnet, nicht gemessen; die Grenzen sind Konstanten mit Messwert im Entwicklertest, siehe Kriterien; Vorbild für „Platz bleibt belegt, bis eine Frist abläuft“: [ADR-0018](0018-verlassen-sofort-weg-nie-beim-schliessen.md), Ergänzung 2a, gebaut in `src/anwesenheit/`)

## Kontext

- Lehre #5 (Security-Review, CWE-770): eine Grenze, die ein billiger Gegenzug aufhebt, ist keine Grenze; `beitreten`/`verlassen` in Schleife erzeugte 2.456 Wechsel/s und fror einen Zuschauer-Tab 28,7 s ein. Lehre #6: schreibende Routen brauchen eine Begrenzung gegen Flut; jeder Beitritt ist frei.
- WBS I0018 (Offene Fragen, M3): ein Agent kann die Frist in Schleife zwischen 7 und 3650 wechseln; „ob eine Rate je Herkunft kommt, ist eine Richtungsfrage an den Betreiber“; [ADR-0031](0031-frist-setzen-put-api-channels-name-frist-jeder-mit-sitzung-bestaetigung-beim-verkuerzen.md) Pre-Mortem 2: „eine Rate je Herkunft wäre eine eigene Interaction, wie für Senden“.
- Senden ist der Hauptfall: jede Nachricht wird dauerhaft geschrieben (Platte, Index im Speicher) und geht an **jeden** Browser ([ADR-0028](0028-nachrichten-live-als-drittes-thema-ordnung-ueber-die-nachrichtenzeit.md) Regel 4); ein Block mit vielen großen Nachrichten verlangsamt das Öffnen ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md), Probe).
- Vision: „Mensch und KI sind gleichberechtigt“; keine Rechte. Herkunft = IP der Gegenstelle ([ADR-0012](0012-identitaet-und-sitzung.md)); auf einem Rechner teilen sich Betreiber und lokale Agenten `127.0.0.1`. Sitzungen: höchstens 64 je Herkunft, 512 gesamt; verlassene zählen bis zum Ende ihrer Gnadenfrist (ADR-0018, 2a).
- [ADR-0021](0021-channel-anlegen-jeder-mit-sitzung-grenzen-je-herkunft-und-gesamt.md) Regel 6: Channel anlegen braucht keine Rate, solange es kein Löschen gibt — der Bestand (256) deckelt die Ereignisse.
- Szenarien mit Maß (Qualitätsziel „Live überall“ ohne eingefrorene Sichten):
  - **S1 Mensch**: tippt höchstens alle paar Sekunden, selten 5 Nachrichten in Folge — darf nie gebremst werden.
  - **S2 Agent berichtet**: 20 Befunde als 20 Nachrichten in einem Zug, danach Fortschritt etwa einmal je Sekunde — darf nicht gebremst werden.
  - **S3 Factory auf einem Rechner**: Betreiber plus 8 Agenten auf `127.0.0.1` — ein entlaufener Agent darf die anderen höchstens kurz bremsen, den Menschen nie dauerhaft.
  - **S4 Flut**: ein Agent in einer Schleife oder ein Skript im LAN — die Sichten aller anderen bleiben bedienbar; die Platte füllt sich nicht in Minuten.

## Optionen

### Option A: Token-Eimer je Kennung **und** je Herkunft; Kosten je Nachricht nach Größe
- Vorteile: Ausbrüche (S2) erlaubt, Dauerlast begrenzt; der Eimer je Kennung schützt die anderen Teilnehmer derselben Herkunft vor einem einzelnen Ausreißer (S3), der Eimer je Herkunft deckelt, was 64 Sitzungen einer Adresse zusammen dürfen (S4) — und ist gegen den Gegenzug „verlassen, neu beitreten“ fest, weil die Herkunft bleibt; Kosten nach Größe begrenzen Ereignisse **und** Bytes mit einem Zähler; mit gestellter Uhr exakt testbar.
- Nachteile: zwei Eimer und eine Kostenformel; Teilnehmer einer Herkunft teilen sich deren Eimer — zieht ein Schwarm Agenten ihn leer, wartet auch der Mensch derselben Adresse (Sekunden).

### Option B: Fester Zähler je Herkunft und Minute (n Nachrichten je Kalenderminute)
- Vorteile: am einfachsten zu erklären und zu bauen.
- Nachteile: an der Minutengrenze das Doppelte in Sekunden; ein Ausreißer sperrt alle seiner Herkunft bis zur nächsten Minute (S3 verletzt); keine Rücksicht auf Größe.

### Option C: Gleitendes Protokoll je Herkunft (Zeitpunkte der letzten n Nachrichten)
- Vorteile: exakt „höchstens n in jedem Fenster von t Sekunden“.
- Nachteile: Speicher je Herkunft wächst mit n; gleiche Schwäche wie B bei S3; Größe unberücksichtigt.

### Option D: Nur je Kennung
- Vorteile: Fairness zwischen Mensch und Agent derselben Adresse ist vollständig.
- Nachteile: 64 Sitzungen einer Adresse ergeben das 64-Fache; neue Kennungen (neuer Name, nach `verlassen`) beginnen mit vollem Eimer — der billige Gegenzug aus Lehre #5.

### Option E: Gesamtgrenze über alle Absender
- Vorteile: deckelt auch Angreifer mit vielen IPv6-Adressen.
- Nachteile: ein einziger Angreifer sperrt **alle** — aus einer Flut wird ein Stillstand; das Gegenteil von S1.

| | S1 Mensch | S2 Agent | S3 Factory | S4 Flut | Schwäche |
|---|---|---|---|---|---|
| A | nie gebremst | Ausbruch frei | Ausreißer gebremst, andere kurz | gedeckelt je Herkunft | Herkunft geteilt |
| B | nie | Minutenrand | alle gesperrt bis Minutenende | gedeckelt, doppelt am Rand | grob |
| C | nie | ja | alle gesperrt | gedeckelt | Speicher, grob |
| D | nie | ja | fair | 64-fach, Gegenzug | umgehbar |
| E | gesperrt bei Flut | gesperrt bei Flut | alle gesperrt | gedeckelt | Stillstand |

## Entscheidung

**Gewählt: Option A**

Es überwiegt, dass nur die Kombination beider Eimer S3 und S4 zugleich trägt: der Eimer je Kennung bremst den Ausreißer, bevor er die Herkunft leert, der Eimer je Herkunft hält, was kein neuer Name aufheben kann. In Kauf genommen: Mensch und Agenten einer Adresse teilen den Eimer der Herkunft — ein Schwarm, der gleichzeitig ausbricht, lässt auch den Menschen Sekunden warten; Angreifer mit vielen IPv6-Adressen umgehen den Eimer je Herkunft (wie bei ADR-0021); ein Neustart füllt alle Eimer.

Regeln:

1. **Baustein** [Schreibkonto](../Architektur/A00014-schreibkonto.md): einziger Halter der Eimer, nur im Speicher; Uhr über den Provider ([X00007](../Architektur/Aspekte/X00007-zeit.md)). Schnittstelle `abbuchen(kennung, herkunft, kosten) -> Ok | Gebremst { grenze, wieder_in }` — prüft **beide** Eimer und bucht nur, wenn beide reichen (unter einer Sperre, alles oder nichts).
2. **Eimer** (Konstanten):

   | Eimer | Vorrat (höchstens) | Nachschub | Dauerlast höchstens |
   |---|---|---|---|
   | je Kennung (`name@herkunft`) | 30 Einheiten | 1 Einheit je Sekunde | 1 kleine Nachricht/s |
   | je Herkunft (IP) | 120 Einheiten | 8 Einheiten je Sekunde | 8 kleine Nachrichten/s, 32 KiB/s |

   Nachschub = (jetzt − zuletzt) × Rate, höchstens bis zum Vorrat; springt die Uhr zurück, gibt es keinen Nachschub und keinen Abzug (`zuletzt := jetzt`).
3. **Kosten Senden**: ⌈Bytes des Texts / 4.096⌉, mindestens 1 — also 1 bis 4 Einheiten. Gebucht wird in zwei Schritten ([ADR-0034](0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) Regel 3): **1 Einheit vorab**, nach der Sitzungsprüfung und vor dem Lesen des Körpers — sie deckt Lesen und Parse und wird auch bei 4xx nicht erstattet ([ADR-0010](0010-eigene-annahmeschleife-hyper-http1.md), Ergänzung Regel 3); der **Rest** (Kosten − 1) nach allen 4xx-Prüfungen, vor dem Schreiben. Scheitert das Schreiben (500), wird nicht erstattet.
4. **Antwort** 429 `{fehler, grenze: "kennung" | "herkunft", wieder_in_ms}` mit Kopf `Retry-After: <⌈wieder_in_ms / 1000⌉, mindestens 1>`. `wieder_in_ms` = Zeit, bis **beide** Eimer die Kosten tragen. Text: „Zu viele Nachrichten von `claude@10.0.0.5` — wieder möglich in 3 s.“ bzw. „Zu viele Nachrichten von `10.0.0.5` (alle Teilnehmer dieser Adresse zusammen) — wieder möglich in 3 s.“ `grenze` nennt den Eimer, der länger braucht.
5. **Kein Gegenzug**: der Eimer je Kennung hängt an der Kennung, nicht am Token — wer `verlassen` und mit demselben Namen neu beitritt, findet seinen Eimer wieder; ein neuer Name bekommt einen vollen Eimer je Kennung, aber nie einen vollen Eimer je Herkunft.
6. **Speicher begrenzt**: volle Eimer werden entfernt (beim Zugriff; zusätzlich ein Durchgang, wenn mehr als 1.024 Einträge bestehen). Ein nicht voller Eimer gehört zu einem Absender der letzten 30 s; Absender brauchen eine Sitzung (höchstens 512 gesamt, verlassene bis zum Ende der Gnadenfrist mitgezählt) — die Tabelle bleibt in derselben Größenordnung.
7. **Betrieb**: `/api/gesundheit` nennt `gebremst` (Zahl der 429 seit dem Start); jede Kennung, die gebremst wird, bekommt **eine** stderr-Zeile je Episode (`claude@10.0.0.5 gebremst (herkunft) — Flut?`), nicht eine je 429.
8. **Geltung**: dieses ADR bucht das Senden in Channels (I0009) und gilt unverändert für Direktnachrichten (I0012). Weitere Routen buchen erst, wenn eine eigene Entscheidung es sagt (Folgeentscheidungen).

Rechenbeispiele (für Abnahme und Anleitung I0019): S2 — 20 kleine Nachrichten in einem Zug kosten 20 von 30, danach eine je Sekunde ohne Bremse. Ein Agent, der 10 Texte zu 16 KiB nacheinander schickt, bucht 40 > 30: die ersten 7 gehen, die 8. wartet 2 s. S3 — acht Agenten brechen gleichzeitig mit je 30 aus: 240 > 120, die Herkunft bremst nach 120 Einheiten für etwa 15 s; danach dürfen sie zusammen 8 Einheiten/s. Solange die übrigen Teilnehmer einer Adresse zusammen unter dem Nachschub der Herkunft bleiben (bis zu sieben Agenten im Dauertakt von 1/s), wird der Mensch nie gebremst; senden sie mehr, teilt er sich den Nachschub mit ihnen und wartet Sekunden. S4 — eine Herkunft schreibt dauerhaft höchstens 8 Nachrichten/s bzw. 32 KiB/s (≈ 2,8 GB/Tag im schlimmsten Fall); jede Sicht bekommt von ihr höchstens 8 Ereignisse/s (Lehre #5: 2.456/s froren einen Tab).

## Konsequenzen

**Positiv:**
- Kein Teilnehmer kann die Sichten der anderen mit Nachrichten einfrieren; Menschen bemerken die Grenze im Normalbetrieb nie.
- Agenten bekommen eine maschinenlesbare Wartezeit (`Retry-After`) statt eines Abbruchs; das Werkzeug `lc senden` (I0019) wartet sie ab.
- Der Mechanismus ist rein und mit gestellter Uhr exakt prüfbar; Kosten je Route sind eine Zahl.

**Negativ / Inkauf zu nehmen:**
- Mensch und Agenten auf einem Rechner teilen den Eimer der Herkunft.
- Die Platte ist nur verlangsamt geschützt (≈ 2,8 GB/Tag je Herkunft), nicht gedeckelt; eine volle Platte meldet ADR-0011 als 500.
- Viele IPv6-Adressen eines Geräts umgehen den Eimer je Herkunft.

**Pre-Mortem (ein Monat nach dem Start gescheitert, weil …):**
1. … der Betreiber im Gespräch mit seinen Agenten ständig 429 sieht. Früh-Indikator: `gebremst` > 0 im Health-Check ohne Flut; stderr-Zeilen mit `herkunft` bei `127.0.0.1`. Gegenmaßnahme: Messwert K-M1 im Entwicklertest mit einer echten Factory-Sitzung; die Konstanten sind eine Zeile — anheben ist ein Commit, keine neue Entscheidung.
2. … ein Agent die 429 nicht versteht und in enger Schleife wiederholt. Früh-Indikator: viele 429 derselben Kennung je Sekunde. Gegenmaßnahme: jede Anfrage mit Sitzung kostet vorab 1 Einheit (Regel 3), also wird auch eine Schleife aus 4xx gebremst; 4xx kostet keinen Schreibvorgang und kein Ereignis; die Anleitung (I0019) und `lc senden` halten `Retry-After` ein; die Verbindungsgrenzen (ADR-0010) begrenzen die Last der Anfragen selbst.
3. … der Eimer je Kennung nach `verlassen` + neuem Namen voll ist und ein Skript so die Kennungsgrenze umgeht. Früh-Indikator: viele kurzlebige Kennungen einer Herkunft. Gegenmaßnahme: Regel 5 — der Eimer je Herkunft bleibt; Sitzungen je Herkunft sind auf 64 je Gnadenfrist begrenzt (ADR-0018, 2a).

**Folgeentscheidungen / Aufgaben:**
- [ ] Erster Unit-Test (Schreibkonto, rein, gestellte Uhr): Kennung `a@10.0.0.1` bucht 30 × 1 → ok, die 31. → `Gebremst { kennung, 1000 ms }`; nach 999 ms → gebremst, nach 1.000 ms → ok (Mutanten Vorrat 29/31 und Nachschub 0,9/1,1 je s werden rot). Herkunft: vier Kennungen derselben IP buchen je 30 → die 121. Einheit gebremst mit `herkunft`, `wieder_in` 125 ms (Mutanten Vorrat 119/121 und Nachschub 7/9 je s werden rot); eine Kennung einer anderen IP → ok. Alles-oder-nichts: Kennung voll, Herkunft leer → gebremst, und der Kennungseimer bleibt voll. Uhr zurück um 1 h → kein Nachschub, kein Fehler. Kosten: 4.096 Bytes → 1, 4.097 → 2, 16.384 → 4. Zwei Schritte: Vorab 1 gelingt, Rest 3 scheitert → 429, die Vorab-Einheit bleibt verbraucht.
- [ ] Erster Integrationstest (echtes Programm): 60 kleine Nachrichten einer Kennung so schnell wie möglich → Zahl der 201 liegt in [30, 30 + ⌈Dauer in s⌉ + 1], jede 429 trägt `Retry-After` ≥ 1 und `grenze: "kennung"`, ihr Satz steht nicht im Log und kein Ereignis wurde gesendet.
- [ ] **Empfehlung Frist (Richtungsfrage aus I0018 an den Betreiber)**: `PUT /api/channels/{name}/frist` bucht nach allen 4xx-Prüfungen pauschal **10 Einheiten** (auch die 409-Vorschau und gleiche Werte — ein Mensch braucht für Verkürzen zwei Aufrufe = 20 ≤ 30). Damit sinkt die Wechsel-Flut M3 auf höchstens 3 Wechsel in Folge, dann einen je 10 s je Kennung bzw. 0,8/s je Herkunft. Bau nach I0009 als eigene kleine Anforderung (Vorschlag im Bericht).
- [ ] **Keine Buchung** für `POST /api/channels` (Bestand 256 deckelt, ADR-0021 Regel 6 — kommt Löschen, entscheidet die Ablösung neu) und für `beitreten`/`verlassen` (Gnadenfrist deckelt, ADR-0018 2a). Anhänge (I0010) entscheiden ihre Kosten selbst (Empfehlung: nach Bytes, eigener Vorrat).

## Quellen / Bezuege

- [A00014 Schreibkonto](../Architektur/A00014-schreibkonto.md), [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00010 Anwesenheit](../Architektur/A00010-anwesenheit.md); Aspekte [X00007 Zeit](../Architektur/Aspekte/X00007-zeit.md), [X00004 Fehlerbehandlung](../Architektur/Aspekte/X00004-fehlerbehandlung.md), [X00002 Benutzerrechte](../Architektur/Aspekte/X00002-benutzerrechte.md)
- ADR-0010, ADR-0012, ADR-0018 (Ergänzung 2a), ADR-0021, ADR-0028, ADR-0031, ADR-0034; WBS I0018 Offene Fragen (M3)
- Token-Eimer als Verfahren: RFC 2697 (Single Rate Three Color Marker) beschreibt Eimer mit Rate und Höchstfüllung — aus dem Gedächtnis, in diesem Auftrag nicht geöffnet; `Retry-After`: RFC 9110 Abschnitt 10.2.3, 429: RFC 6585 Abschnitt 4 — aus dem Gedächtnis; CWE-770

## Ergänzungen

- **2026-10-28 (Vorlauf „Betriebsprotokoll“) — stderr wird Betriebsprotokoll:** Regel 7 „eine stderr-Zeile je Episode“: `WARNING schreibkonto`-Episode je Kennung über die Episoden-Mechanik des Protokolls; `gebremst` im Health-Check bleibt, dazu zählt `warnings` ([ADR-0042](0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) Regeln 2 und 3; Format [ADR-0040](0040-betriebsprotokoll-datei-je-lokalem-tag-unter-logs-eine-zeile-je-eintrag.md), stderr spiegelt ab Warning). Die übrigen Regeln dieser ADR gelten unverändert.
