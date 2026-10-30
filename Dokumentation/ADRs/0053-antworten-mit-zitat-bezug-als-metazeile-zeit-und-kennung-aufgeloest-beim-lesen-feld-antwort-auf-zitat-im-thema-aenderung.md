# ADR-0053: Antworten mit Zitat — Bezug als Metazeile `+ antwort_auf: <zeit> <kennung>` am Satz der Antwort, aufgelöst beim Lesen aus dem Index (lebender Verweis), Feld `antwort_auf` beim Senden und in jeder gelieferten Nachricht, `zitat` im Thema `aenderung`, Sprung über den Verlauf selbst, Wecken über die Kennung im Bezug

**Status**: Akzeptiert
**Datum**: 2026-10-30
**Entscheider**: Claude (autonom, architekt, Auftrag des Directors, Karte #79); Richtung vom Auftraggeber und product-owner (PO-Entscheidungen Zeilen 142, 143, 145–151)
**Kontext-Anforderung**: [R00047](../../Anforderungen/R00047-mit-zitat-antworten-api-und-browser.md) (WBS I0043 mit F0157, F0158; I0044 mit F0159, F0160)
**Erprobt durch**: — (gerechnet, nicht gemessen; überträgt Gebautes: Metazeilen und Vorwärtsverträglichkeit ADR-0004, Nachschlagen einer Nachricht über ihre Zeit `Verlauf::nachricht` und Fassung je Nachricht `src/gespraeche/verlaufsindex.rs` (ADR-0048 Regel 3), Prüfung unter Gesprächs- und Schreibsperre `src/gespraeche/mod.rs` `senden_mit`, Sendekörper mit Erlaubnisliste der Felder `src/http/senden.rs` `sendekoerper_lesen`, Thema `aenderung` ADR-0048 Regel 11, Blöcke über die Zeit als Cursor ADR-0025. Messung: Probe P9 in F0157.)

## Kontext

- **Auftrag** (PO 143, 146–150; Vision-Zielbild „Nachrichten kopieren und mit Zitat beantworten“): eine Antwort ist eine **gewöhnliche neue Nachricht** am Ende des Verlaufs mit Bezug auf **genau eine** frühere Nachricht **desselben Gesprächs**; keine Threads, keine Verschachtelung, kein Zähler am Original. Das Zitat ist ein **lebender Verweis**: bearbeitet → neuer Textanfang, gelöscht → „Nachricht gelöscht“ ohne Text, entfallen → „ältere Nachricht, nicht mehr vorhanden“. **Im Log nur der Bezug (Zeit und Kennung des Originals), nie der zitierte Text** (PO 148). Unbekannter, fremder oder gelöschter Bezug → Abweisung mit deutschem Text, nichts gespeichert; Mensch und Agent gleich (PO 149). Wecken als Ausbaustufe; ist es am Weckmechanismus teuer, Kommentar statt Bau (PO 150). Klick auf das Zitat springt zum Original und lädt nach „wie der Sprung aus der Suche“ (PO 147) — die Suche (I0011) ist nicht gebaut.
- **Offene Fragen des planer** (R00047 „Offene Fragen“ 1–9; Kommentar auf Karte #79): Länge des Textanfangs, Route und Feldnamen, Status der Abweisung, Erkennen von `entfallen`, Archiv, abwesende Teilnehmer, Löschen der Antwort, Kosten des Weckens, Sprung ohne I0011.
- **Bestand**:
  - Satz = Kopfzeile `@ <zeit> <kennung>`, Metazeilen `+ schluessel: wert` (unbekannte Schlüssel werden gelesen und erhalten — vorwärtsverträglich), Text eingerückt ([ADR-0004](0004-logformat-kopfzeile-und-eingerueckter-text.md)). `logformat::satzart` erkennt einen Änderungssatz **nur** an `+ bearbeitet:` und `+ geloescht:`; jede andere Metazeile lässt den Satz eine Nachricht bleiben (`src/gespraeche/logformat.rs:78–116`). Text kann keine Metazeile vortäuschen (eingerückt, M14 in ADR-0048).
  - Die Nachrichtenzeit ist ID, im Ordner eindeutig und global streng steigend unter **einer** Schreibsperre ([ADR-0035](0035-nachrichtenzeit-eine-schreibsperre-ueber-alle-gespraeche.md)); `Verlauf::nachricht(&zeit)` findet eine Nachricht mit binärer Suche im Tag ihres Datums (`src/gespraeche/verlaufsindex.rs:113`, `fundort`), samt Fassung (`fassung`, `aenderungssatz`).
  - Senden prüft unter Gesprächs- **und** Schreibsperre: Wiederholschlüssel → Anhänge nehmen → Zeit → Quote → Satz anhängen → Index → Veröffentlichen (`src/gespraeche/mod.rs` `senden_mit`). Löschen setzt die Fassung `Geloescht` unter denselben Sperren (ADR-0048 Regel 9 a), Entfallen läuft unter der Gesprächssperre (ADR-0030).
  - Der Sendekörper erlaubt heute nur `text` und `anhaenge`, jedes weitere Feld ist 422 `feld` (`src/http/senden.rs:506–527`) — ein älteres Programm weist `antwort_auf` also laut ab, statt es still zu verlieren.
  - Antwortform einer Nachricht ([ADR-0025](0025-verlauf-ueber-die-api-bloecke-mit-zeit-als-cursor.md) Regel 3 mit Ergänzungen): `{zeit, von, art, text, anhaenge, zustand, geaendert}`, Felder kommen hinzu, keines wird umgedeutet, jedes ist immer da (`null` statt fehlend). Byte-Budget je Block 256 KiB ([ADR-0039](0039-grosse-antworten-byte-budget-je-block-werkbank-und-abrissgrenze-als-sendefrist.md)).
  - Fenster im Tab: höchstens 5 Blöcke à 100, Nachladen über `vor`/`nach`, Anker von Hand ([ADR-0026](0026-verlauf-im-browser-als-fenster-aus-hoechstens-fuenf-bloecken.md)); ADR-0026 nennt „an eine Zeit springen“ als Folge für Live und Suche, gebaut ist kein Sprung.
  - `warten` (I0016) ist nicht gebaut; laut [ADR-0007](0007-live-kanal-sse-fuer-browser-long-poll-fuer-agenten.md) holt es zuerst aus dem Index nach `seit` nach und wartet dann auf den Bus, mit Filtern `erwaehnung`, `direkt`, `channel`.
- **Lastprofil Team-LAN** (`AI-Team/umgebung.md`): 30 Nachrichten/s Spitze, 100.000 je Channel, Zustellung < 1 s, Channel öffnen < 500 ms.

### Qualitätsziele mit Szenario

| # | Ziel | Szenario mit Maß |
|---|---|---|
| QZ1 | Kein Zitattext auf der Platte | Original mit 500 Zeichen und Marke `ZITATMARKE-4711`, Antwort darauf → Marke genau einmal in der Tagesdatei; Satz der Antwort ist um höchstens `+ antwort_auf: ` + Zeit (29) + Leerzeichen + Kennung + Zeilenende größer als ohne Bezug (bei `claude@127.0.0.1`: 15 + 29 + 1 + 16 + 1 = 62 Bytes) |
| QZ2 | Lebend und live | Original bearbeitet → jedes Zitat darauf in jedem offenen Tab < 1 s nach der 200 mit neuem Textanfang; gelöscht → „Nachricht gelöscht“ < 1 s; Lesen über die API liefert sofort den neuen Zustand; das Log der Antwort bleibt byte-gleich |
| QZ3 | Öffnen bleibt schnell | 100.000 Nachrichten, jede zweite eine Antwort auf eine zufällige ältere → neueste 100 weiter < 50 ms Server-Zeit (Q1), Start < 2 s (Q6) |
| QZ4 | Sprung ohne Hängen | Channel mit 100.000 Nachrichten, Original = älteste → nach dem Klick binnen 1 s sichtbar und hervorgehoben, kein Long Task > 200 ms |
| QZ5 | Kein Orakel, nie geglaubt | ein Bezug wird nur im Index **des Gesprächs** nachgeschlagen, in das gesendet wird; Zeit aus einem anderen Gespräch antwortet wie eine nie dagewesene; Kennung des Originals kommt aus dem Index, nie vom Client; ein gelöschter Text und eine frühere Fassung erscheinen in keinem Zitat |
| QZ6 | Lesbar ohne localchatA | `less` auf die Tagesdatei zeigt an der Antwort, auf welche Nachricht (Zeit, Absender) sie antwortet |

## Optionen

### Gesamtschnitt — wo lebt der Bezug?

| | Form | Stärke | Schwäche |
|---|---|---|---|
| **V1 (gewählt)** | **Metazeile am Satz der Antwort** `+ antwort_auf: <zeit> <kennung>` (Zeit und Kennung des Originals, aus dem Index geschrieben); **aufgelöst beim Lesen** über die Zeit im Index des Gesprächs (Zustand, Textanfang aus der aktuellen Fassung) | ein Satz, ein `write_all` mit Rückbau — atomar wie jede Nachricht; nichts Neues im Index (das Nachschlagen gibt es schon); der Verweis lebt von selbst, weil er beim Lesen aufgelöst wird; ein älteres Programm liest die Antwort als gewöhnliche Nachricht (unbekannte Metazeile erhalten); `less` zeigt Bezug und Absender | je Antwort im Block ein Nachschlagen O(log n); das Zitat entfallener Originale kennt nur noch Zeit und Kennung |
| V2 | Zitat als Kopie im Satz (Textanfang in einer Metazeile oder im Text) | Lesen ohne Nachschlagen; das Zitat überlebt das Original | zitierter Text auf der Platte und doppelt an der Quote, eine frühere Fassung bliebe sichtbar — gegen PO 148 und Vision „frühere Fassungen sieht niemand“ — verworfen |
| V3 | eigener Bezugssatz nach der Nachricht (Familie der Änderungssätze, ADR-0048 Regel 1) | Satz der Nachricht unverändert | eine Antwort wären zwei Sätze — nicht atomar (zweites Schreiben kann scheitern), Waisen-Regeln, und eine Antwort **ist** eine Nachricht, kein Eingriff in eine fremde — verworfen |
| V4 | V1 plus **Rückwärtsverzeichnis** Original → Antworten im Index, der Server schickt bei einer Änderung je Antwort ein Ereignis | der Server weiß, welche Zitate nachzuführen sind | eine Thread-Struktur durch die Hintertür (PO 143: kein „alle Antworten auf …“), Speicher je Antwort, Pflege beim Entfallen; der Tab kann die ≤ 500 Nachrichten seines Fensters selbst durchsehen — verworfen |
| V5 | nur Konvention im Text (`> Zitat` in Markdown), kein Feld | kein Server-Code | zitierter Text im Log (PO 148), kein lebender Verweis, kein Sprung ohne Suche, kein Wecken — verworfen |

### Einzelfragen

| # | Frage | (a) gewählt | verworfen (Stärke / Grund) |
|---|---|---|---|
| F1 | Feld beim Senden | `antwort_auf: "<zeit>"` (RFC 3339 wie `zeit` beim Bearbeiten, verglichen als Zeitpunkt) im Körper von `POST …/nachrichten`; fehlend oder `null` = ohne Bezug | (b) Objekt `{zeit, von}` — Stärke: explizit; Grund: `von` käme vom Client und müsste gegen den Index geprüft werden, ohne etwas zu gewinnen (die Zeit ist eindeutig). (c) eigene Route `…/nachrichten/antworten` — Stärke: getrennte Prüfreihenfolge; Grund: PO 149 „dieselbe Sende-Route“, Wiederholschlüssel, Anhänge, Quote müssten verdoppelt werden |
| F2 | Feld beim Lesen | `antwort_auf` **in jeder Nachricht**, `null` ohne Bezug, sonst Objekt `{zeit, von, zustand, textanfang, anhang, geaendert}` | (b) Feld fehlt ohne Bezug — Stärke: ≈ 18 Bytes je Nachricht weniger; Grund: Hausregel ADR-0025 (jedes Feld immer da, `null` statt fehlend), Agenten prüfen eine Form |
| F3 | Status der Abweisung | **422** `{fehler, regel: "antwort_auf", grund: "format" \| "unbekannt" \| "geloescht"}` | (b) 404 — Grund: 404 meint hier den Channel (ADR-0034); (c) 409 für gelöscht wie beim Bearbeiten — Stärke: gleiches Wort für gleichen Zustand; Grund: beim Bearbeiten ist die Nachricht das **Ziel** der Route, hier ist der Körper fehlerhaft; ein Status mit `grund` ist für Agenten einfacher |
| F4 | Textanfang | Server, reine Funktion: Leerraumfolgen (auch Zeilenumbrüche) → ein Leerzeichen, Rand weg; > 120 Zeichen (Unicode-Skalare) → die ersten 119, Rand weg, „…“; Markdown bleibt Quelltext | (b) 200 Zeichen — mehr Kontext für Agenten, aber das Zitat zeigt höchstens zwei Zeilen (≈ 80–90 Zeichen bei 375 px); (c) Markdown entfernen — zweiter Markdown-Leser im Server; (d) Tab kürzt selbst — zwei Regeln, die auseinanderlaufen |
| F5 | Zitat live nachführen (F0160) | Thema `aenderung` trägt zusätzlich `zitat`: das Bezugsobjekt **der geänderten Nachricht**, wie es in jeder Antwort darauf stünde; der Tab ersetzt `antwort_auf` aller Nachrichten seines Fensters mit `antwort_auf.zeit = nachricht.zeit`, wenn `zitat.geaendert` größer ist | (b) Tab berechnet den Textanfang aus `nachricht.text` — zweite Implementierung von F4 in JavaScript; (c) V4 — siehe oben; (d) eigenes Thema `zitat` — zweites Ereignis je Änderung ohne Gewinn |
| F6 | Sprung zum Original ohne I0011 | **der Verlauf bekommt `springen(zeit)`** (I0044 baut ihn, I0011 nutzt ihn später): im Fenster → hinscrollen; sonst erst einen Block `nach=<zeit − 1 ms>` holen, **dann** das Fenster ersetzen (Fehlschlag oder Original fehlt → Hinweis, Fenster bleibt) | (b) warten auf I0011 — F0159 hinge an einer nicht geplanten Probe (3–5 Mio vor I0011); (c) Sprung nur innerhalb des Fensters — gegen PO 147 und QZ4 |
| F7 | Wer wird geweckt (F0158) | die wartende Sitzung, deren **Kennung** gleich `antwort_auf.von` ist und die die Antwort nicht selbst gesendet hat; gilt für den Filter `erwaehnung` wie eine Erwähnung | (b) über den Namen wie `@name` — Stärke: gleiche Regel wie I0013; Grund: I0013 ist nicht entschieden, „Absender = Kennung“ (ADR-0048 Regel 6) ist es; eine neue Adresse ist ein neuer Teilnehmer |

## Entscheidung

**Gewählt: V1 mit F1 (a) bis F7 (a).**

Es überwiegt, dass der Bezug **nichts Neues** im Index braucht: das Nachschlagen über die Zeit und die Fassung je Nachricht gibt es seit ADR-0048, also ist der lebende Verweis bloß „beim Lesen auflösen“, und die Antwort bleibt ein einziger Satz, der wie jede Nachricht geschrieben, gezählt, zurückgebaut und entfallen wird. In Kauf genommen: je Antwort ein Nachschlagen beim Lesen (gerechnet ≈ 2 µs, siehe Lastnachweis); das Zitat eines entfallenen Originals kennt nur Zeit und Absender; der Tab sieht für das Live-Nachführen sein Fenster durch; F0158 bleibt bis zum Bau von I0016 offen.

Regeln:

1. **Metazeile** ([A00007](../Architektur/A00007-logformat.md)): `+ antwort_auf: <zeit> <kennung>` — Zeit des Originals in der Schreibweise des Logs (Millisekunden, Versatz; aus der **Kopfzeile des Originals** im Index, nie die Schreibweise des Clients), ein Leerzeichen, Kennung des Originals aus dessen Kopfzeile. Höchstens eine je Satz; Stelle: nach `+ art:`, vor den `+ anhang:`-Zeilen. Beispiel:
   ````text
   @ 2026-10-30T09:12:00.120+01:00 codex@127.0.0.1
   + art: agent
   + antwort_auf: 2026-10-30T09:10:03.004+01:00 claude@127.0.0.1
     Ja, Tests sind grün.

   ````
   - Ein Satz mit `+ antwort_auf:` **bleibt eine Nachricht** (`satzart` unverändert: nur `bearbeitet`/`geloescht` machen einen Änderungssatz). Änderungssätze tragen nie `+ antwort_auf:`; Bearbeiten ändert nur den Text, der Bezug bleibt.
   - **Defekt beim Laden** (`WARNING laden` mit Datei und Zeile im bestehenden Budget von 10 je Datei, **die Nachricht bleibt**, nur ohne Bezug, `antwort_auf: null`): zwei `+ antwort_auf:`; Zeit unlesbar; Kennung fehlt oder ist keine Kennung (Namensregel ADR-0013 und Herkunft); Zeit des Originals nicht echt vor der eigenen Zeit. Die Prüfung läuft im selben Durchgang über die Metazeilen wie `satzart` — kein zweiter Durchgang.
2. **Senden** ([A00004](../Architektur/A00004-http-schnittstelle.md), [A00006](../Architektur/A00006-gespraeche.md)) — `POST /api/channels/{name}/nachrichten` (und die Direktgespräch-Ressource ab I0012) nimmt das Feld `antwort_auf` an; die Erlaubnisliste des Sendekörpers wird `text`, `anhaenge`, `antwort_auf` (der Text von 422 `feld` nennt alle drei). Körpergrenze unverändert (ein Feld von ≈ 50 Bytes passt in die Reserve von ADR-0034 Regel 2).
   - **Vor der Sperre** (bei den Körperprüfungen, nach `text`/`anhaenge`): `antwort_auf` ist `null`, fehlt, oder eine Zeichenkette in RFC 3339 — sonst **422** `{regel: "antwort_auf", grund: "format"}`.
   - **Unter Gesprächs- und Schreibsperre**, direkt nach dem Wiederholschlüssel und **vor** dem Nehmen der Anhänge: `Verlauf::nachricht(zeit)` **im Gespräch des Pfads** — keine → **422** `{regel: "antwort_auf", grund: "unbekannt"}`; Fassung `Geloescht` → **422** `{regel: "antwort_auf", grund: "geloescht"}`; `Bearbeitet` und ungeändert sind gültig. Damit ist der Bezug gegen Löschen und Entfallen serialisiert: entweder 201 mit gültigem Bezug oder 422, nie ein Bezug auf ein in derselben Millisekunde gelöschtes Original.
   - **Wiederholschlüssel**: die Prüfsumme umfasst zusätzlich die Zeit des Bezugs als Zeitpunkt (Millisekunden) oder „keiner“ — derselbe Schlüssel mit anderem Bezug ist 422 `schluessel` (ADR-0034 Regel 6). Eine Wiederholung nach dem Löschen des Originals antwortet mit 200 (Wiederholung, ADR-0034 Regel 6; gespeichert ist gespeichert) mit dem **heutigen** Zustand im Zitat.
   - **Kosten**: Vorab und Restbuchung wie jedes Senden (ADR-0036); ein 422 `antwort_auf` unter der Sperre hat — wie 507 — Vorab und Rest schon gebucht. Kein Anhang genommen, kein Satz, kein Ereignis, `belegt` unverändert.
   - **Texte**: `format` „antwort_auf muss die Zeit einer Nachricht sein, wie sie im Verlauf steht (RFC 3339, z. B. 2026-10-30T09:10:03.004+01:00).“ · `unbekannt` „Die Nachricht, auf die du antwortest, gibt es in diesem Gespräch nicht (mehr).“ · `geloescht` „Die Nachricht, auf die du antwortest, ist gelöscht.“
3. **Auflösen beim Lesen** (reine Funktion `zitat(bezug, fund) → Zitat` über das Ergebnis des Nachschlagens; [A00009](../Architektur/A00009-verlaufsindex.md)): Bezug = Zeit und Kennung aus der Metazeile; Fund = `Verlauf::nachricht(zeit)` im eigenen Gespräch.
   - kein Fund → `zustand: "entfallen"`; Fund mit anderer Kennung als im Bezug (nur von Hand möglich, Zeiten sind eindeutig) → ebenfalls `"entfallen"` (nie fremder Text unter falschem Namen);
   - Fassung `Geloescht` → `"geloescht"`; `Bearbeitet` → `"bearbeitet"`, Text aus dem Änderungssatz; ungeändert → `"da"`;
   - **Form** `antwort_auf: {zeit, von, zustand, textanfang, anhang, geaendert}`: `zeit`, `von` aus dem Bezug (auch bei `entfallen`); `textanfang` nach F4 aus dem Text der **aktuellen** Fassung, bei `geloescht`/`entfallen` `""`; `anhang` = Anzeigename des ersten Anhangs, den die Nachricht in ihrer Fassung liefert, wenn ihr Text leer ist, sonst `null`; `geaendert` der Fassung oder `null`.
   - Der Textanfang wird beim Kopieren unter der Gesprächssperre **aus dem Satz gelesen, ohne ihn zu kopieren** — höchstens bis 120 Zeichen gesammelt sind (≤ 480 Bytes gelesen statt bis zu 16 KiB je Original), und nie mehr als 256 gelesene Zeichen, Leerraum mitgezählt (`LESEGRENZE_ZEICHEN`): ein Original aus Leerzeilen kostet je Antwort so höchstens diese Obergrenze, nicht seinen ganzen Satz (Nachtrag Security-Review #79 B1; Messung `tests/leistung_zitat_leerraum.rs`).
   - **Wer es trägt**: Antwort des Sendens (201 und Wiederholung), jeder Block, Thema `nachricht` (aufgelöst unter der Schreibsperre beim Veröffentlichen), `warten` (I0016). Eine **gelöschte Antwort** liefert `antwort_auf: null` (der Bezug ist Inhalt, ADR-0048 Regel 4). Das Byte-Budget je Block zählt die Bytes von `antwort_auf` mit.
   - **Keine Verschachtelung**: aufgelöst wird genau eine Stufe; `antwort_auf` des Originals wird nie mitgeliefert.
4. **Thema `aenderung`** ([A00011](../Architektur/A00011-live-kanal.md), ergänzt ADR-0048 Regel 11): zusätzlich `zitat` = `zitat(Bezug auf die geänderte Nachricht selbst, neuer Fund)` — bei Bearbeiten `zustand: "bearbeitet"` mit neuem Textanfang, bei Löschen `"geloescht"`, `textanfang: ""`. Veröffentlicht im selben Ereignis unter derselben Sperre; kein zusätzliches Ereignis, kein Rückwärtsverzeichnis. Agenten können `zitat` übergehen.
5. **Tab** ([A00005](../Architektur/A00005-weboberflaeche.md)), reine Funktion `zitatNachfuehren(fenster, ereignis)`: für jede Nachricht im Fenster mit `antwort_auf.zeit` gleich `ereignis.nachricht.zeit` (als Zeitpunkt) wird `antwort_auf` durch `ereignis.zitat` ersetzt, wenn `zitat.geaendert` größer ist als das bekannte (`null` ist kleiner als jede Zeit); `geloescht` wird nie zurückgenommen — dieselbe Regel wie ADR-0048 Regel 11; ebenso die offene Antwortzeile über dem Eingabefeld. Nach einer Lücke bringt das Neuladen der Blöcke (ADR-0048 F1) den Stand. `entfallen` erscheint erst beim nächsten Laden (Entfallen sendet kein Ereignis, ADR-0030).
6. **Zeichnen** (Tab): Zitat über dem Text als bedienbares Element (`button` oder Link mit `aria-label` aus Kennung, Uhrzeit und Textanfang); Kennung, Uhrzeit (HH:MM aus `zeit`) und Textanfang **nur über `textContent`**, nie `innerHTML`, nie Markdown (kein Nachladen fremder Ressourcen über ein Zitat); höchstens zwei Zeilen per CSS (`line-clamp`), `white-space: normal`. `geloescht` → „Nachricht gelöscht“, `entfallen` → „ältere Nachricht, nicht mehr vorhanden“, Text leer mit `anhang` → „Anhang: <anhang>“. Die **Antwortzeile** über dem Eingabefeld zeigt bis zur Bestätigung den `text` des Originals aus dem Fenster mit derselben CSS-Kürzung (kein Textanfang im Tab berechnet); die vorläufige Nachricht (ADR-0037) zeigt dasselbe, die bestätigte das `antwort_auf` des Servers. Wortlaute, Ansagen, Hervorhebung: `ux-reviewer`.
7. **Antwortzeile im Tab**: höchstens ein Bezug je Eingabe, gehalten im Zustand der Eingabe des Channels, **nicht** im Entwurf (`localStorage`); Channelwechsel, Abbrechen, `Esc` und erfolgreiches Senden verwerfen ihn; „Antworten“ an einer anderen Nachricht ersetzt ihn. Die vorläufige Nachricht trägt die Zeit des Bezugs, „Erneut senden“ sendet sie mit. **422 `antwort_auf`** → Text der Instanz am Feld, Zeile weg, Getipptes bleibt im Feld (wie die übrigen 422 des Sendens, R00012), nichts gespeichert.
8. **Sprung** (`springen(zeit)` im Verlauf, `web/verlauf.js`; Grundlage ADR-0026 Regeln 2, 3, 8):
   - (a) Original im Fenster → `scrollIntoView({block: 'center'})`, Hervorhebung (Form beim `ux-reviewer`, nicht nur Farbe, ≈ 2 s), Fokus auf das Original mit `preventScroll`;
   - (b) sonst, wenn `zustand` nicht `entfallen` ist: **erst** `GET …/nachrichten?nach=<zeit − 1 ms>&n=100` (Zeiten sind Millisekunden und eindeutig, also steht das Original vorn), mit eigenem `AbortController` des Channels; steht im Block eine Nachricht mit genau dieser Zeit → Fenster durch diesen Block ersetzen wie beim Öffnen (Regel 5 ohne „ans Ende“), dann (a); Nachladen oben und unten übernimmt danach Regel 2 mit dem Anker aus Regel 3; „Zum Neuesten“ (Regel 6) führt zurück;
   - (c) `entfallen`, Original nicht im Block oder Abruf gescheitert → Hinweis „ältere Nachricht, nicht mehr vorhanden“ bzw. der bestehende Fehlerhinweis, **das Fenster bleibt unverändert** (der alte Inhalt wird erst nach erfolgreichem Abruf ersetzt).
   - Die Suche (I0011) übernimmt `springen(zeit)` unverändert; die ADR zu I0011 entscheidet nur, woher die Zeit kommt.
9. **Wecken** (F0158, Regel für `warten` aus I0016): eine Nachricht weckt die wartende Sitzung mit Kennung `K` zusätzlich, wenn `antwort_auf.von == K` und `von != K`, gleich welcher Zustand; sie gilt für den Filter `erwaehnung` wie eine Erwähnung. Beim Nachholen aus dem Index liest `warten` `K` direkt aus der Metazeile — **kein** Nachschlagen des Originals, kein Rückwärtsverzeichnis. **Kosten**: ein Vergleich je Nachricht und wartender Sitzung, wie bei einer Erwähnung — **nicht teuer**; F0158 wartet allein darauf, dass I0016 gebaut ist (kein Kommentar „teuer“ nach PO 150). Änderungen (`aenderung`) wecken weiterhin nicht (ADR-0048 Regel 11).
10. **Grenzen und Sichtbarkeit**: nachgeschlagen wird nur im Gespräch des Pfads — wer in ein Gespräch senden darf, sieht es; eine Zeit aus einem anderen Channel oder einem fremden Direktgespräch ist dort nie vorhanden und antwortet wie eine nie dagewesene (`unbekannt`). Archivierte Channels (I0038): das Senden weist dort ohnehin ab, die Aktion „Antworten“ fehlt. Abwesende Teilnehmer: kein Sonderfall. Die Aufbewahrung ändert sich nicht — ein Bezug hält kein Original am Leben.
11. **Protokoll und Health**: keine neue Zeile im Ereigniskatalog, kein neues Feld im Health-Check; 422 `antwort_auf` läuft durch denselben Weg wie die übrigen 422 des Sendens; nie Text oder Textanfang im Protokoll. `log_bytes`/`belegt` enthalten die Metazeilen.

### Lastnachweis (gerechnet; gemessen wird P9)

- **Block**: ≤ 200 Nachrichten, im schlechtesten Fall alle Antworten → 200 × `Verlauf::nachricht` (binäre Suche im Tag: bei 14.300 Nachrichten/Tag ≈ 14 Schritte mit je einer Kopfzeit) + Fassung (BTreeMap) + ≤ 480 Bytes Textanfang lesen ≈ 2 µs je Antwort → ≈ 0,4 ms je Block unter der Gesprächssperre, gegen 50 ms für Q1. Bytes: ≤ 200 × (≈ 480 + Kennung + 2 Zeiten + Anhangname) ≈ 200 × 1 KiB — das Budget von 256 KiB beendet den Block notfalls früher (ADR-0039, mindestens eine Nachricht).
- **Senden**: ein Nachschlagen mehr unter der Sperre (≈ 2 µs) bei 30 Nachrichten/s — vernachlässigbar.
- **Laden**: keine neue Struktur, eine Metazeile mehr je Antwort im ohnehin laufenden Durchgang — Start Q6 unverändert.
- **Live**: `aenderung` wächst um ≤ 1 KiB; der Tab sieht je Änderung höchstens 500 Nachrichten durch (< 1 ms).
- **Sprung**: ein Block `nach=` (O(log n) im Server, wie Nachladen) und Zeichnen von 100 Nachrichten wie beim Öffnen (≈ 45 ms laut ADR-0026 Regel 8).

## Konsequenzen

**Positiv:**
- Eine Antwort ist und bleibt eine Nachricht: Schreiben, Rückbau, Quote, Entfallen, Strom, Nachholen gelten ohne neuen Begriff; ein älteres Programm zeigt sie als gewöhnliche Nachricht.
- Der lebende Verweis kostet kein neues Ordnungsmerkmal und keinen Rückwärtsindex; frühere Fassungen und gelöschte Texte können in keinem Zitat erscheinen, weil nur die aktuelle Fassung gelesen wird.
- `springen(zeit)` ist ein gemeinsamer Baustein für Zitat, Suche (I0011) und spätere Erwähnungen.
- Wecken ist eine Zeile in der Filterregel von `warten`.

**Negativ / Inkauf zu nehmen:**
- Jede Antwort im Block kostet ein Nachschlagen (gerechnet ≈ 2 µs).
- Das Zitat eines entfallenen Originals zeigt nur Zeit und Absender.
- `entfallen` erscheint in offenen Tabs erst beim nächsten Laden.
- Der Tab durchsucht je `aenderung` sein Fenster; Antworten außerhalb des Fensters holen den Stand beim Nachladen.
- Ein 422 `antwort_auf` kostet am Schreibkonto wie ein 507 (Vorab und Rest) — Abweichung vom Wortlaut „Schreibkonto bleibt unberührt“ in R00047 (Bericht an den planer).
- Ein Original, das am Rand des Fensters bearbeitet wird, während der Tab nachlädt, kann bis zum Ende des Abrufs den alten Textanfang zeigen; der gepufferte `aenderung` geht nach dem Block durch dieselbe Regel (ADR-0048 Regel 11) — hingenommen.

**Pre-Mortem (ein Monat nach dem Bau gescheitert, weil …):**
1. … eine Antwort im Log den zitierten Text trägt, weil der Browser ihn „zur Sicherheit“ in den Text schrieb oder der Server den Textanfang in die Metazeile legte. Früh-Indikator: P5 (Marke). Gegenmaßnahme: Regel 1 (nur Zeit und Kennung), Mutante Z5.
2. … nach einem Bearbeiten das Zitat den **alten** Text zeigt, weil der Textanfang aus dem Satz des Originals statt aus der Fassung kam. Früh-Indikator: P4. Gegenmaßnahme: Regel 3 über `Verlauf::nachricht` samt Fassung, Mutante Z3.
3. … das Öffnen großer Channels langsam wird, weil je Antwort das ganze Original kopiert wird. Früh-Indikator: P9. Gegenmaßnahme: Regel 3 (nur bis 120 Zeichen lesen), Messung im Release.
4. … ein Agent über eine Zeit aus einem fremden Direktgespräch erfährt, dass es dort eine gelöschte Nachricht gibt. Früh-Indikator: P3 (c), Kein-Orakel-Probe ab I0012. Gegenmaßnahme: Regel 2/10 (nur im Gespräch des Pfads), Mutante Z1.
5. … der Sprung den Verlauf leert, wenn das Original fehlt oder das Netz hakt. Früh-Indikator: E2E mit verzögert antwortender bzw. scheiternder Route (Gelerntes „UI-Tests mit verzögert antwortendem Fake“). Gegenmaßnahme: Regel 8 (c) — erst holen, dann ersetzen; Mutante Z14.
6. … das Zitat Markdown ausführt (`![x](http://…)` lädt ein Bild). Früh-Indikator: P11. Gegenmaßnahme: Regel 6 (`textContent`), Mutante Z9.

**Folgeentscheidungen / Aufgaben** (Vorschläge an den planer, keine Arbeitsliste des architekt):
- [ ] F0157 (I0043): Metazeile schreiben/lesen (Regel 1), Textanfang (F4), `zitat` auflösen (Regel 3), Senden mit Bezug unter der Sperre und Prüfsumme (Regel 2), Sendekörper und Texte (Regel 2), `antwort_auf` in Senden-Antwort, Block, `nachricht`; `zitat` in `aenderung` (Regel 4), Leistung P9 — Bausteine und Mini-Eignung im Bericht Karte #79.
- [ ] F0158: Regel 9 in den Filter von `warten`, sobald I0016 gebaut ist; bis dahin rot ohne Kommentar „teuer“.
- [ ] F0159 (I0044): Aktion „Antworten“, Antwortzeile (Regel 7), Senden mit Bezug, Zitat zeichnen (Regel 6), `springen(zeit)` (Regel 8), Ansagen.
- [ ] F0160: `zitatNachfuehren` (Regel 5) für Fenster und Antwortzeile.
- [ ] I0011, I0016, I0012, I0019: siehe „Folgen für andere Knoten“.

**Folgen für andere Knoten** (dort nicht geändert):
- **I0011 Suche**: Treffer springen über `springen(zeit)` (Regel 8); `textanfang` und Metazeile zählen nie als Treffer (die Metazeile ist kein Text, der Textanfang steht nicht im Log).
- **I0016 Warten**: liefert `antwort_auf` in jeder Nachricht (Regel 3) und weckt nach Regel 9.
- **I0013 Erwähnungen**: entscheidet, ob `@name` über den Namen oder die Kennung trifft; Regel 9 bleibt bei der Kennung, bis I0013 anders entscheidet (dann neue Ergänzung hier).
- **I0012 Direktgespräche**: dasselbe Feld auf der Senden-Route des Direktgesprächs; nachgeschlagen im Index des Direktgesprächs.
- **I0019 Anleitung**: Feld `antwort_auf` beim Senden, Bezugsobjekt beim Lesen, Zustände, 422 `antwort_auf` mit `grund`; die OpenAPI-Beschreibung ([A00002](../Architektur/A00002-programm-localchata.md)) nachziehen.

**Angriffsmatrix-Skizze** (für den `security-reviewer`; Akteure wie ADR-0048: A0 ohne Sitzung, A1 Kennung mit Sitzung, A2 Opfer):

| # | Variante | Akteur | Erwartet |
|---|---|---|---|
| S1 | Zeit aus `# technik` in `# allgemein` / aus fremdem Direktgespräch | A1 | 422 `unbekannt`, gleicher Text wie nie dagewesen, Log byte-gleich |
| S2 | `antwort_auf` als Objekt mit `von`, als Zahl, als Liste, doppelt im Körper | A1 | 422 `antwort_auf`/`format` bzw. 422 `feld`/`doppelt` (Feldleser, ADR-0046 L9) |
| S3 | Original mit `<img src=x onerror=…>` und `![x](http://angreifer/)` | A2 | Zitat zeigt den Text als Text, kein Element, keine Anfrage |
| S4 | Original bearbeitet mit Geheimnis entfernt | A2 | kein Zitat, kein Block, kein Ereignis liefert die frühere Fassung |
| S5 | 30 Antworten/s mit Bezug | A1 | Schreibkonto bremst wie Senden; Zustellung < 1 s |
| S6 | Bezug auf die Zeit eines Änderungssatzes | A1 | 422 `unbekannt` (ein Änderungssatz ist keine Nachricht, `Verlauf::nachricht` findet ihn nicht) |

### Abnahme-Proben (Test zuerst; „heute“ jeweils rot, weil nichts gebaut ist)

- **P1 Logformat (Unit, rein)**: `satz` einer Nachricht mit Bezug (t1, `claude@127.0.0.1`) → genau `+ antwort_auf: <t1 im Logformat> claude@127.0.0.1` nach `+ art:`, vor `+ anhang:`; Rückweg liest Bezug (t1, Kennung); `satzart` → `Nachricht`; zwei Bezugszeilen, `+ antwort_auf: gestern x@y`, Zeit ≥ eigene Zeit, fehlende Kennung → Nachricht ohne Bezug plus Defekt; Text `"x\n+ antwort_auf: …"` bleibt Text (eingerückt).
- **P2 Textanfang (Unit, rein)**: `"Frage?"` → `"Frage?"`; `"a\n\n  b\tc "` → `"a b c"`; 300 × `x` → 119 × `x` + `"…"` (120 Zeichen); 119 × `x` + `" "` + `"yyy"` → endet ohne Leerzeichen vor `"…"`; 200 × `"ä"` → 119 `ä` + `"…"` (Zeichen, nicht Bytes); `""` → `""`.
- **P3 Vier Fälle (Integration, echtes Programm)**: (a) gültig → 201, `antwort_auf: {zeit: t1, von: "claude@127.0.0.1", zustand: "da", textanfang: "Frage?", anhang: null, geaendert: null}`; (b) Zeit, die es im Gespräch nicht gibt → 422 `grund: "unbekannt"`; (c) Zeit aus `# technik` in `# allgemein` → 422 `unbekannt` mit **gleichem** `fehler`-Text wie (b); (d) gelöscht → 422 `geloescht`; (e) `"antwort_auf": "gestern"`, `42`, `{}` → 422 `format`; in (b)–(e) `sha256sum` über `channels/` und `belegt` gleich, ein in (b)–(d) mitgesendeter Anhang bleibt im Eingang — ein anschließendes Senden ohne Bezug mit derselben Anhang-ID ergibt 201.
- **P4 Zustände (Integration)**: Original bearbeitet zu `Frage!` → Lesen der Antwort `zustand: "bearbeitet"`, `textanfang: "Frage!"`, `geaendert` = Zeit des Änderungssatzes; gelöscht → `"geloescht"`, `textanfang: ""`, `anhang: null`; Tag des Originals entfallen (Testuhr, Frist 1) → `"entfallen"`, `zeit`/`von` weiter da; der Satz der Antwort im Log in allen drei Schritten byte-gleich; nie erscheint `Frage?` nach dem Bearbeiten in einer Antwort (Text-Suche über Block, Senden-Antwort, Strom-Mitschnitt).
- **P5 Log nur Bezug (Integration)**: QZ1 mit Marke; Byte-Differenz zur gleichen Antwort ohne Bezug genau 15 + 29 + 1 + Länge Kennung + 1.
- **P6 Strom und Wiederholung (Integration)**: Strom-Mitschnitt: `nachricht` der Antwort trägt `antwort_auf` mit `zustand: "da"`; Bearbeiten des Originals → `aenderung` mit `zitat: {zeit: t1, zustand: "bearbeitet", textanfang: "Frage!", …}`; Löschen → `zitat.zustand: "geloescht"`. Senden mit `Idempotency-Key` und Bezug t1, Wiederholung mit Bezug t2 → 422 `schluessel`; Wiederholung mit t1 nach dem Löschen des Originals → 200 (Wiederholung, ADR-0034 Regel 6) dieselbe Zeit, `zustand: "geloescht"`.
- **P7 Keine Verschachtelung, Zeitfolge, gelöschte Antwort (Integration)**: `anna` antwortet auf die Antwort von `codex` → Bezug `codex`, im Objekt kein weiteres `antwort_auf`; Antworten in Zeitfolge am Ende; die Antwort von `codex` gelöscht → ihr `antwort_auf: null`.
- **P8 Ohne Bezug unverändert (Integration)**: Senden ohne Feld und mit `"antwort_auf": null` → 201 wie vorher plus `antwort_auf: null`; Grenzen Länge 422, Schreibkonto 429, Quote 507 unverändert; ein Körper mit `antwort` (Tippfehler) → 422 `feld` mit Text, der `antwort_auf` nennt.
- **P9 Leistung (Release, Generator wie `tests/leistung.rs`)**: 100.000 Nachrichten über 7 Tage, jede zweite eine Antwort auf eine zufällige ältere, 10 % der Originale bearbeitet, 5 % gelöscht → neueste 100 < 50 ms Server-Zeit (Median aus 20), Start < 2 s; Messwert in den Bericht.
- **P10 Sprung (E2E)**: im Fenster → sichtbar, hervorgehoben, Fokus; außerhalb (100.000 Nachrichten, Original älteste) → sichtbar binnen 1 s, Long-Task-Messung ≤ 200 ms; Route für `nach=` verzögert 2 s → alter Inhalt bleibt bis zur Antwort; Route scheitert → Hinweis, Fenster unverändert (gleiche erste und letzte Zeit); `entfallen` → Hinweis ohne Abruf.
- **P11 Darstellung sicher (E2E)**: Original `<img src=x onerror="window.__x=1">` und `![b](http://127.0.0.1:<frei>/b.png)` → im Zitat als Text, `window.__x` undefiniert, keine Anfrage an den freien Port.
- **P12 Live (E2E, zwei Tabs)**: wie R00047 „Zitat lebt“ und „Offene Zeile lebt“; zusätzlich: ein gepuffertes `aenderung` vor dem Block (Route verzögert) landet nach dem Block im Zitat.
- **P13 Wecken (Integration, sobald I0016 gebaut)**: wie R00047 „Wecken“ — Antwort auf `codex` weckt `codex` < 1 s; auf `claude` nicht; eigene Antwort nicht; `aenderung` nicht.

### Mutanten (jeder muss mindestens eine Probe rot machen)

| # | Mutante | rot in |
|---|---|---|
| Z1 | Nachschlagen über alle Gespräche statt im Gespräch des Pfads | P3 (c) |
| Z2 | `Geloescht` beim Senden nicht geprüft | P3 (d) |
| Z3 | Textanfang aus dem Satz des Originals statt aus der Fassung | P4 |
| Z4 | Textanfang bei `geloescht` geliefert | P4, S4 |
| Z5 | Textanfang in die Metazeile geschrieben | P5 |
| Z6 | Prüfsumme ohne Bezug | P6 |
| Z7 | Bezugsprüfung **nach** dem Nehmen der Anhänge | P3 (anschließendes Senden mit derselben Anhang-ID scheitert) |
| Z8 | Kennung im Log aus dem Körper bzw. Zeit in der Schreibweise des Clients | P1, P3 (Zeit mit anderem Versatz) |
| Z9 | Zitat mit `innerHTML` oder Markdown gezeichnet | P11 |
| Z10 | `aenderung` ohne `zitat` bzw. Tab ignoriert `zitat` | P6, P12 |
| Z11 | Kennungsvergleich beim Auflösen aus | Unit zu Regel 3 (von Hand gesetzte Kennung → `entfallen`) |
| Z12 | eigene Antwort weckt | P13 |
| Z13 | gelöschte Antwort liefert `antwort_auf` | P7 |
| Z14 | Sprung ersetzt das Fenster vor dem Abruf | P10 |
| Z15 | Antwortzeile im Entwurf gespeichert (überlebt Channelwechsel) | E2E „Zeile und Channelwechsel“ (R00047) |

## Quellen / Bezuege

- [A00004 HTTP-Schnittstelle](../Architektur/A00004-http-schnittstelle.md), [A00005 Weboberfläche](../Architektur/A00005-weboberflaeche.md), [A00006 Gespräche](../Architektur/A00006-gespraeche.md), [A00007 Logformat](../Architektur/A00007-logformat.md), [A00009 Verlaufsindex](../Architektur/A00009-verlaufsindex.md), [A00011 Live-Kanal](../Architektur/A00011-live-kanal.md); Aspekte [X00002](../Architektur/Aspekte/X00002-benutzerrechte.md), [X00008](../Architektur/Aspekte/X00008-transaktionen-und-nebenlaeufigkeit.md)
- ADR-0004, ADR-0007, ADR-0013, ADR-0025, ADR-0026, ADR-0028, ADR-0030, ADR-0034, ADR-0035, ADR-0036, ADR-0037, ADR-0039, ADR-0044, ADR-0046, ADR-0048, ADR-0052
- R00047 und User Stories, WBS I0043, I0044, F0157–F0160, I0011, I0013, I0016; PO-Entscheidungen Zeilen 142, 143, 145–151; Vision „Nachrichten kopieren und mit Zitat beantworten“, „Mensch und KI gleichberechtigt“, „Agenten können warten und reagieren“, Leitplanken Datenhaltung und Plattenquote
- Referenz Teams-Chat „Zitierte Antwort“ (laut PO 146 abgerufen 2026-10-30): normale Nachricht mit Vorschau, Klick springt zum Original

## Ergänzungen
