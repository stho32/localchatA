---
id: R00023
status: Neu
datum: 2026-10-29
---

# R00023: Plattenquote

## Beschreibung

Die Gesamtgröße der Gespräche ist auf eine Quote begrenzt, die der Betreiber beim Start einstellt (`--plattenquote`, Standard 4 GiB). Danach gilt:

- **Einstellen**: `--plattenquote 512MiB` oder `LOCALCHATA_PLATTENQUOTE=512MiB`, `aus` für keine Grenze. Ein falscher Wert beendet den Start mit Exit 2. Der Start-Eintrag und `/api/gesundheit` nennen die wirksame Quote.
- **An der Quote wird nicht gesendet**: würde eine neue Nachricht die Gespräche über die Quote bringen, antwortet das Senden 507 mit Grund und Ausweg. Nichts wird angehängt, und deswegen wird nichts gelöscht. Lesen, Channel anlegen und Frist setzen gehen weiter.
- **Platz entsteht nur über die Frist**: entfallen alte Tage (Tageswechsel, Verkürzen, Start), wird wieder angenommen.
- **Der Betreiber sieht es kommen**: über 90 % der Quote steht eine Warnung im Betriebsprotokoll, die Abweisungen einer Kennung stehen als Episode darin, und `/api/gesundheit` zählt sie in `quote_abgewiesen`.

Zahlt ein auf: [Vision](R00000-vision.md) — Leitplanke „Plattenquote“ (Gesamtgröße begrenzt, an der Grenze wird Senden abgewiesen, nichts gelöscht, Platz nur über die Aufbewahrungsfrist) und „eine gemeinsame LAN-Instanz“ (ein entlaufener Agent füllt nicht Platte und Speicher des Rechners). Kein Widerspruch zur Vision: keine zweite Löschhandlung neben der Frist.

Herkunft: Karte #34 — Betreiber 2026-10-29 „Ja, als Quote“ auf die Frage aus ADR-0036, Ergänzung Karte #31 (a) (das Schreibkonto verlangsamt nur: ≈ 5,7 GB/Tag je Herkunft). Mechanik entschieden vom architekt in [ADR-0044](../Dokumentation/ADRs/0044-plattenquote-gespraeche-im-index-gezaehlt-senden-507-nichts-wird-deswegen-geloescht.md) mit Ergänzungen Karte #34 an ADR-0034, ADR-0036, ADR-0042 und ADR-0043.

Slices (WBS `Dokumentation/Planung/localchata.md`, unter I0023 Plattenquote in D0001 Start und Ablage): F0074 Quote einstellen, F0075 Senden an der Quote abgewiesen, F0076 Quote im Betriebsprotokoll.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00008, R00011, R00014 und R00016: Gespräch, Tagesdatei, Verlaufsindex, Frist, Entfallen, Verkürzen, Bestätigung, Schreibkonto, Kennung, Herkunft, Episode, Start-Eintrag. Neu: **Quote** (höchstens erlaubte Bytes aller Gespräche), **belegt** (Bytes der Tagesdateien, die die Instanz im Index hält).

## Geschäftlicher Nutzen

Heute bremst das Schreibkonto nur. Eine Herkunft, die dauerhaft flutet, schreibt bis zu ≈ 5,7 GB je Tag. Bei der Standardfrist von 7 Tagen sind das ≈ 40 GB, bei 3.650 Tagen Frist wächst es ohne Grenze. Der Verlaufsindex hält jeden Tag in der Frist im Arbeitsspeicher. Darum stirbt die Instanz am Speicher, lange bevor die Platte voll ist — und mit ihr die Zusammenarbeit aller.

Mit R00023 belegen die Gespräche im Standard höchstens 4 GiB, auf der Platte und im Speicher. Das Lastprofil „Team-LAN“ bleibt darunter (eine Woche ≈ 1,6 GB, Bestand des Profils ≈ 2,65 GB). Eine flutende Herkunft braucht vom Normalstand ≈ 11 h bis zur Quote. In dieser Zeit sieht der Betreiber die 90-%-Warnung und kann eingreifen. An der Quote verliert niemand etwas, das schon geschrieben ist.

## Funktionale Anforderungen

### Quote einstellen (F0074; ADR-0044 Regeln 3, 7, 9; ADR-0043 Ergänzung Karte #34)

- **Werte**: ganze Zahl mit Einheit `MiB` oder `GiB`, Groß-/Kleinschreibung egal, ohne Leerzeichen (`4GiB`, `512mib`), Spanne 1 MiB bis 1.024 GiB; oder das Wort `aus`.
- **Vorrang**: Option vor Umgebungswert vor Standard (4 GiB = 4.294.967.296 Bytes); gelesen nur beim Start.
- **Falscher Wert** (ohne Einheit, `MB`/`GB`, `0…`, außerhalb der Spanne, negativ, leer, keine Zahl): auf stderr „--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus“, Exit 2, nichts geschrieben, kein `logs/` angelegt. Ein falscher Umgebungswert ist nur ohne Option ein Fehler: ist `--plattenquote` gültig gesetzt, wird `LOCALCHATA_PLATTENQUOTE` weder gelesen noch geprüft (ADR-0044 Ergänzung Karte #26, dieselbe Regel wie `--protokoll-stufe`). Option ohne Wert: „--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe“, Exit 2.
- **Hilfe**: die Aufrufzeile von `--hilfe` nennt `[--plattenquote <größe>]`; der Eintrag steht umbrochen wie die übrigen Optionen auf drei Zeilen — `  --plattenquote <größe>`, darunter eingerückt „Höchstgröße aller Gespräche (Tageslogs der Channels und Direktgespräche)“ und „wie 4GiB oder 512MiB, aus für keine Grenze; Standard 4GiB; auch über LOCALCHATA_PLATTENQUOTE“.
- **Start-Eintrag**: `INFORMATION start` trägt zusätzlich `plattenquote=<bytes>` bzw. `plattenquote=aus`, `belegt=<bytes>` und `plattenquote_option=--plattenquote` | `LOCALCHATA_PLATTENQUOTE` | `standard` (Schlüssel `plattenquote_option` statt `option`, Hinweis aus Karte #26; den Katalog in ADR-0042 zieht der architekt nach).
- **Health-Check**: neues Feld `plattenquote` (wirksame Quote in Bytes, `null` bei `aus`).

### Senden an der Quote abgewiesen (F0075; ADR-0044 Regeln 1, 2, 4, 5, 6, 9, 11; ADR-0034 Ergänzung Karte #34)

- **Was zählt**: die Bytes der Tagesdateien aller Gespräche, soweit die Instanz sie im Verlaufsindex hält. Nicht: `einstellungen.toml`, `logs/`, Dateien, die die Instanz nicht lädt. `log_bytes` in `/api/gesundheit` ist genau diese Zahl.
- **Regel**: `belegt + Bytes des neuen Satzes > Quote` → abgewiesen. Genau bis an die Quote wird angenommen (die Grenze gehört dazu).
- **Antwort**: `POST /api/channels/{name}/nachrichten` → **507** `{"fehler": …, "regel": "quote", "belegt": <bytes>, "hoechstens": <bytes>}` ohne `Retry-After`. Text:
  „Nicht gesendet: Die Gespräche belegen 4,0 GiB, die Quote dieser Instanz ist 4,0 GiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.“
- **Größen im Text**: ab 1 GiB in GiB, sonst in MiB; eine Nachkommastelle, halbe nach oben; deutsches Komma, Tausenderpunkt.
- **Stelle in der Prüfreihenfolge** (ADR-0034 Regel 3, Schritt 9): nach Vorab-Buchung, Körpergrenze (413), allen 4xx und der Restbuchung am Schreibkonto (429). Die gebuchten Einheiten werden nicht erstattet — eine Schleife auf 507 bremst das Schreibkonto.
- **Nichts geschieht**: eine 507 hängt nichts an, schickt kein Ereignis `nachricht` und löscht nichts.
- **Wiederholung**: ein bekannter Wiederholschlüssel antwortet auch an der Quote 200 mit der ursprünglichen Nachricht.
- **Was weitergeht**: Lesen, Ereignisstrom, Beitreten, Verlassen, Channel anlegen, Frist setzen — auch Verlängern.
- **Platz entsteht** nur, wenn Tage entfallen (Start, Tageswechsel, Verkürzen der Frist), oder durch eine größere Quote beim Neustart. Ein Tag, dessen Löschen scheiterte, zählt weiter.
- **Kennzahl**: neues Feld `quote_abgewiesen` in `/api/gesundheit` — Zahl der 507 seit dem Start.
- **Oberfläche**: der Browser zeigt den `fehler` der 507 an der fehlgeschlagenen Nachricht wie jeden Fehler außer 500; „Erneut senden“ bleibt (R00012).
- **Ausgeschaltet** (`aus`): keine Abweisung; `log_bytes` zählt weiter.

### Quote im Betriebsprotokoll (F0076; ADR-0044 Regeln 7, 8; ADR-0042 Ergänzung Karte #34)

- **„Über 90 %“** heißt `belegt × 10 > Quote × 9`, ganzzahlig.
- **Wechsel nach oben** (beim Senden, einmal je Wechsel): `WARNING quote Gespräche belegen 90 % der Quote | belegt=<bytes> hoechstens=<bytes>`.
- **Wechsel nach unten** (beim Entfallen, einmal je Wechsel): `INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=… hoechstens=… grund=<anlass>` mit `start` | `tageswechsel` | `frist_gesetzt`.
- **Start** (nach dem Laden, einmal): über der Quote → `WARNING quote Gespräche belegen mehr als die Quote — Senden wird abgewiesen | belegt=… hoechstens=…`; sonst über 90 % → `WARNING quote Gespräche belegen 90 % der Quote | …`. Geladen wird trotzdem alles.
- **Abweisungen**: `WARNING quote Senden abgewiesen: Quote erreicht` als Episode je Kennung nach R00014/R00016 (Beginn mit `kennung`, `channel`, `belegt`, `hoechstens`; Ende-Zeile mit der Zahl der Abweisungen). Jede einzelne 507 zusätzlich als `DEBUG`.
- **Ausgeschaltet**: keine `quote`-Zeile.

### Rechenbeispiele

- **Standard**: Quote 4 GiB = 4.294.967.296 Bytes; Warnschwelle über 3.865.470.566,4 Bytes, also ab 3.865.470.567 Bytes („3,6 GiB“).
- **Grenze** (Quote 1.000 Bytes): geladen 900; Satz 80 → 980 angenommen; Satz 21 → 1.001 > 1.000 → abgewiesen, `belegt` bleibt 980; Satz 20 → 1.000 angenommen.
- **Schwelle** (Quote 1.000): 900 ist nicht über 90 %, 901 ist darüber. 880 → 901 ergibt eine WARNING; 901 → 922 keine weitere; 922 → 72 (Tag mit 850 Bytes entfällt) eine INFORMATION.
- **Integrationsfall** (Quote 1 MiB = 1.048.576): gestern 1.048.276 Bytes → beim Start über 90 % (10.482.760 > 9.437.184), nicht über der Quote. Ein Text zu 17 Zeichen (`Quote-Probe 17 Z.`) vom Agenten `beta@127.0.0.1` ergibt einen Satz zu 81 Bytes (Kopfzeile 47 + Metazeile `+ art: agent` 13 + eingerückter Text mit Abschluss 21). Drei Sätze → 1.048.519 (201, 201, 201); der vierte → 1.048.600 > 1.048.576 → 507 mit `belegt: 1048519`. Text: „… belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. …“. Frist auf 1 verkürzt → gestern entfällt → `belegt` 243 → INFORMATION mit `grund=frist_gesetzt`; die nächste Nachricht → 201.
- **Größen in Worten**: 4.294.967.296 → „4,0 GiB“; 1.073.741.824 → „1,0 GiB“; 1.073.741.823 → „1.024,0 MiB“; 536.870.912 → „512,0 MiB“; 1.101.005 → „1,1 MiB“; 1.101.004 → „1,0 MiB“ (1,05 MiB = 1.101.004,8 Bytes).
- **Schreibkonto im Integrationsfall**: 4 Sendungen (4 Einheiten) + 409-Vorschau und bestätigtes Verkürzen (20 Einheiten, R00021) = 24 ≤ 30 je Kennung — keine 429 dazwischen.

## Nicht-funktionale Anforderungen

- **Sicherheit** (CWE-400, Lehre #5): Platte und Speicher der Gespräche überschreiten die Quote nie, auch bei dauerhafter Flut und 3.650 Tagen Frist. Archivieren (I0022) wird kein Gegenzug: archivierte Bytes zählen mit, sobald I0022 gebaut ist.
- **Leistung**: die Prüfung kostet je Senden eine Addition und einen Vergleich unter der Sperre, die das Senden ohnehin hält, und keinen Plattenzugriff; der Start zählt ohne zusätzliches Lesen. `log_bytes` im Health-Check wird O(1). Kein neues Zeitkriterium; die Leistungskriterien aus R00011 gelten unverändert.
- **Betrieb**: zwischen der 90-%-Warnung und der ersten 507 liegen bei einer flutenden Herkunft ≈ 11 h (gerechnet in ADR-0044 Regel 3, nicht gemessen); das Monitoring entscheidet aus `log_bytes`, `plattenquote` und `quote_abgewiesen`, ohne das Protokoll zu lesen.
- **Benutzerfreundlichkeit**: wer an der Quote sendet, liest Grund und Ausweg an seiner Nachricht; ein Agent erkennt den Fall an `regel: "quote"` und wiederholt nicht.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien, wenn nicht anders genannt: echtes Programm mit Wegwerf-Ordner, `--port 0`, `TZ=Europe/Berlin`, `LOCALCHATA_JETZT`; Beitritte und Sitzungen wie in R00011; `heute.log` ist `logs/<heute>.log`. „Der Quote-Ordner“ ist ein Ordner mit `channels/allgemein/einstellungen.toml` (Frist 7) und einer gültigen Tagesdatei von gestern mit 1.048.276 Bytes (= 1.048.576 − 300), hergestellt mit der Testhilfe aus B0359. „Die Zeile `X`“ meint die Zeile ab der Stufe, ohne Zeitstempel.

### F0074 Quote einstellen

- [ ] **Größe lesen** (Unit, rein; Unit Aufruf aus ADR-0044 „Folgeentscheidungen“, wörtlich): „`--plattenquote 4GiB` → 4.294.967.296; `512mib` → 536.870.912; `aus` → keine Quote; `1MiB` → 1.048.576; `0MiB`, `0`, `4`, `4GB`, `1025GiB`, `-1GiB`, leer → Aufruffehler mit „4GiB oder 512MiB, oder aus“; ohne Wert → `WertFehlt`; Option schlägt Umgebung; ohne beides → 4 GiB.“ Dazu `1024GiB` → 1.099.511.627.776 und `--plattenquote 1MiB` mit `LOCALCHATA_PLATTENQUOTE=4` → 1.048.576, Herkunft `--plattenquote` (ADR-0044 Ergänzung Karte #26). Die Mutanten „`GB` wie `GiB`“, „Spanne ohne Obergrenze“ und „`0` heißt aus“ werden rot.
- [ ] **Falscher Wert beim Start** (Integration): `LOCALCHATA_PLATTENQUOTE=4` → Exit 2, stderr enthält „--plattenquote braucht eine Größe wie 4GiB oder 512MiB, oder aus“, im Ordner gibt es kein `logs/`. `--plattenquote 2MiB` mit `LOCALCHATA_PLATTENQUOTE=4` startet dagegen mit der Quote der Option: Start-Eintrag mit `plattenquote=2097152 belegt=0 plattenquote_option=--plattenquote` (ADR-0044 Ergänzung Karte #26). `--plattenquote` als letztes Argument → Exit 2 mit „--plattenquote braucht einen Wert — Aufruf siehe localchatA --hilfe“. Die Mutante „Prüfung nach dem Anlegen von `logs/`“ wird rot.
- [ ] **Hilfe** (Integration): `localchatA --hilfe` → Exit 0, die Ausgabe enthält den Eintrag zu `--plattenquote` aus „Funktionale Anforderungen“ wörtlich, auf drei Zeilen umbrochen wie dort, und die Aufrufzeile nennt `[--plattenquote <größe>]`.
- [ ] **Wirksame Quote sichtbar** (Integration): leerer Ordner, `--plattenquote 512MiB` → `heute.log` hat den Start-Eintrag mit `plattenquote=536870912 belegt=0 plattenquote_option=--plattenquote`; `/api/gesundheit` → `plattenquote: 536870912`. Mit `LOCALCHATA_PLATTENQUOTE=aus` → `plattenquote=aus` und `plattenquote_option=LOCALCHATA_PLATTENQUOTE`, Health `plattenquote: null`. Ohne Angabe → `plattenquote=4294967296 plattenquote_option=standard`, Health `plattenquote: 4294967296`. Die Mutanten „Umgebung schlägt Option“ und „Herkunft immer `standard`“ werden rot.
- [ ] **Health-Form** (Integration): der Test `leerer_ordner_ist_bereit_mit_kennzahlen_null` erwartet zusätzlich `"plattenquote": 4294967296` (ab F0075 auch `"quote_abgewiesen": 0`); alle übrigen Felder unverändert.

### F0075 Senden an der Quote abgewiesen

- [ ] **Zähler und Grenze** (Unit, Gespräche mit Speicherablage und gestellter Uhr; erster Unit-Test aus ADR-0044 „Folgeentscheidungen“, wörtlich): „Quote 1.000 Bytes; geladen ein Tag mit 900 Bytes → `belegt = 900`. Senden mit Satz von 80 Bytes → ok, `belegt = 980 = Σ Index`. Satz von 21 Bytes → `Quote { belegt: 980, hoechstens: 1.000 }`, Ablage und Index unverändert, kein Rückruf. Satz von genau 20 Bytes → ok, `belegt = 1.000` (Grenze gehört dazu; Mutanten `>=` statt `>` und „Prüfung nach dem Anhängen“ werden rot). Wiederholung mit bekanntem Schlüssel bei voller Quote → `Wiederholt`, kein Fehler. `pflegen` nimmt den 900-Byte-Tag → `belegt = 100`; Mutante „Entfallen zieht nicht ab“ wird rot. Nach jedem Schritt `belegt = Σ Verlauf::bytes()`.“
- [ ] **Ohne Quote** (Unit): mit `aus` wird derselbe 21-Byte-Satz bei `belegt = 980` angehängt. Die Mutante „`aus` heißt 0“ wird rot.
- [ ] **Löschen scheitert** (Unit, Speicherablage „Entfernen scheitert“ wie R00008): der Tag bleibt im Index, `belegt` bleibt gleich und gleich Σ Index. Die Mutante „zieht vor dem Entfernen ab“ wird rot.
- [ ] **Größen in Worten** (Unit, rein): die Werte aus „Rechenbeispiele“ ergeben genau die dort genannten Texte.
- [ ] **507 im echten Programm** (Integration; erster Integrationstest aus ADR-0044 „Folgeentscheidungen“, ohne die Protokollzeilen, die F0076 belegt): Quote-Ordner, Start mit `--plattenquote 1MiB`; `beta` sendet Texte zu 17 Zeichen → 201, 201, 201, dann 507 mit `regel: "quote"`, `belegt: 1048519`, `hoechstens: 1048576`, ohne `Retry-After`, `fehler` „Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.“ Die Tagesdatei von heute ist nach der 507 Byte für Byte wie vor ihr; ein zweiter Strom (`gamma`) hat genau drei Ereignisse `nachricht`; `/api/gesundheit` → `plattenquote: 1048576`, `quote_abgewiesen: 1`, `log_bytes: 1048519`. Die Mutante „Prüfung nach dem Anhängen“ wird rot (die Tagesdatei wächst).
- [ ] **Platz über die Frist** (Integration, Fortsetzung): `beta` setzt die Frist von `allgemein` auf 1 → 409; bestätigt → 200; die Tagesdatei von gestern ist weg; `log_bytes: 243`; die nächste Nachricht von `beta` → 201.
- [ ] **Was an der Quote weitergeht** (Integration, Quote-Ordner, Quote erreicht wie oben; Lesen, Anlegen und Frist mit der Kennung `delta`): `GET /api/channels/allgemein/nachrichten` → 200; Channel `neu` anlegen → 201; Frist von `allgemein` auf 30 verlängern → 200; `beta` sendet noch einmal mit dem Wiederholschlüssel einer seiner drei angenommenen Nachrichten → 200 mit der ursprünglichen Nachricht, `quote_abgewiesen` unverändert (Wiederholschlüssel gelten je Kennung, R00011).
- [ ] **Ausgeschaltet** (Integration): Quote-Ordner, Start mit `--plattenquote aus`; vier Nachrichten wie oben → viermal 201; Health `plattenquote: null`, `quote_abgewiesen: 0`, `log_bytes: 1048600`.
- [ ] **Im Browser** (E2E, neu `e2e/tests/r00023-quote.spec.js`): Quote-Ordner, Start mit `--plattenquote 1MiB`, die Quote ist über die API erreicht (wie oben); A ist beigetreten und hat `# allgemein` offen; A sendet „Hallo zusammen“ → die Nachricht steht als fehlgeschlagen mit dem Text der 507 und der Schaltfläche „Erneut senden“; der Verlauf zeigt keine neue bestätigte Nachricht. Rechnung: an der Quote sind 57 Bytes frei (1.048.576 − 1.048.519); „Hallo“ ergäbe bei `shoff@127.0.0.1` genau 57 Bytes und würde angenommen, „Hallo zusammen“ ergibt 66. Die Mutante „507 wie 500“ (Text für Plattenfehler statt `fehler`) wird rot.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenzeile außer „Failed to load resource“ zu der einen 507, die das Kriterium „Im Browser“ herbeiführt.
- [ ] **Nacharbeit Karte #34: 507-Text groß nach Doppelpunkt** (B0486; Unit, Integration, E2E; Rechtschreib-Review #34 Muss, § 81 Amtliches Regelwerk, Duden D 34). Abnahme-Probe des `rechtschreib-reviewer`, wörtlich: „`git grep -n 'Nicht gesendet: die Gespräche' -- src tests e2e` → 0; `git grep -n 'Nicht gesendet: Die Gespräche belegen' -- src/http/senden.rs tests/plattenquote.rs e2e/tests/r00023-quote.spec.js` → 4 Fundstellen; Laufzeit (Quote-Ordner 1.048.276 Bytes, --plattenquote 1MiB, 4× senden „Quote-Probe 17 Z.“) → 507 fehler = „Nicht gesendet: Die Gespräche belegen 1,0 MiB, die Quote dieser Instanz ist 1,0 MiB. Platz entsteht, wenn alte Tage nach ihrer Aufbewahrungsfrist entfallen (beim nächsten Tageswechsel) oder eine Frist verkürzt wird; die Quote stellt der Betreiber beim Start mit --plattenquote ein.“; Browser „⚠ Nicht gesendet: Die Gespräche …““ Die Mutante „klein nach dem Doppelpunkt“ wird rot.
- [ ] **Bestehende Tests**: die Integrations- und E2E-Tests aus R00008, R00011, R00012, R00014 und R00021 laufen mit dem Standard von 4 GiB unverändert grün (kein Test erreicht 3,6 GiB).

### F0076 Quote im Betriebsprotokoll

- [ ] **Schwelle als Zustandswechsel** (Unit, rein, Quote 1.000): 880 → 900 kein Wechsel; 900 → 901 nach oben; 901 → 980 keiner; 980 → 901 keiner; 901 → 900 nach unten; 950 → 100 nach unten; 100 → 950 nach oben. Die Mutanten „`>=` statt `>`“ und „Wechsel bei jedem Senden über 90 %“ werden rot.
- [ ] **Wechsel melden** (Unit, Speicher-Senke, Quote 1.000): geladen zwei Tage mit 850 + 30 Bytes; gesendet 21 → genau eine `WARNING quote Gespräche belegen 90 % der Quote | belegt=901 hoechstens=1000`; noch einmal 21 → keine weitere Zeile; Verkürzen nimmt den 850-Byte-Tag → genau eine `INFORMATION quote Gespräche wieder unter 90 % der Quote | belegt=72 hoechstens=1000 grund=frist_gesetzt`. Mit Anlass Tageswechsel bzw. Start trägt dieselbe Zeile `grund=tageswechsel` bzw. `grund=start`. Die Mutante „grund immer start“ wird rot.
- [ ] **Start** (Unit, Speicher-Senke): geladen 1.001 bei Quote 1.000 → genau eine `WARNING quote Gespräche belegen mehr als die Quote — Senden wird abgewiesen | belegt=1001 hoechstens=1000` und keine 90-%-Zeile; geladen 950 → genau eine 90-%-WARNING; geladen 2.000 mit `aus` → keine `quote`-Zeile. Die Mutante „keine Prüfung beim Start“ wird rot.
- [ ] **Protokoll im echten Programm** (Integration; Protokollzeilen des ersten Integrationstests aus ADR-0044, wörtlich): „Start mit `--plattenquote 1MiB` → Protokoll hat `WARNING quote Gespräche belegen 90 % der Quote`“; nach der 507 „genau eine `WARNING quote Senden abgewiesen: Quote erreicht — Episode beginnt`“; nach dem bestätigten Verkürzen „`INFORMATION quote Gespräche wieder unter 90 % der Quote … grund=frist_gesetzt`“. Ausgeschrieben für den Quote-Ordner: `WARNING quote Gespräche belegen 90 % der Quote | belegt=1048276 hoechstens=1048576` nach dem Laden, vor der ersten Anfrage; die Beginn-Zeile der Episode trägt `kennung=beta@127.0.0.1 channel=allgemein belegt=1048519 hoechstens=1048576`, auf stderr dieselbe Zeile.
- [ ] **Episode je Kennung** (Integration, `LOCALCHATA_EPISODE_RUHE_MS=2000`, `--protokoll-stufe debug`): an der Quote sendet `beta` dreimal → drei 507, genau eine Beginn-Zeile der Episode, drei `DEBUG`-Zeilen zu den Abweisungen; nach 2 s Ruhe eine Ende-Zeile `… Episode endet: 3 in …`. Die Mutante „je 507 eine WARNING“ wird rot.

## Betroffene Verzeichnisstruktur

- Start: `src/start/aufruf.rs` (Option, Hilfe), neu z. B. `src/start/plattenquote.rs` (Größe lesen, wirksame Quote — wie `src/start/protokollstufe.rs`), `src/start/mod.rs` (Quote an `Gespraeche::laden`, Start-Eintrag, Start-Prüfung).
- Gespräche: `src/gespraeche/mod.rs` (`laden`, `senden_mit`, `kennzahlen`, `Sendefehler::Quote`), `src/gespraeche/pflege.rs` (Abzug beim Entfallen, Anlass für die Meldung), Schwelle z. B. in neu `src/gespraeche/quote.rs`.
- HTTP: `src/http/senden.rs` (507, Episode, DEBUG), `src/http/gesundheit.rs` (`plattenquote`, `quote_abgewiesen`).
- Testhilfe: `src/testdaten.rs` bzw. `examples/` (Quote-Ordner, aufrufbar wie `verlauf-erzeugen`).
- Tests: Unit neben dem Code; Integration neu z. B. `tests/plattenquote.rs`; E2E neu `e2e/tests/r00023-quote.spec.js`. Oberfläche ohne Programmcode (`web/zustellung.js` `grundAus` zeigt `fehler` schon).

## Technische Überlegungen

### Ablauf

1. **Start**
   - 1.1 Option und Umgebung lesen → wirksame Quote und Herkunft | Exit 2 vor `logs/`.
   - 1.2 `Gespraeche::laden(…, quote)` → `belegt` = Σ Rohtext-Bytes der geladenen Tage.
   - 1.3 Start-Prüfung: über der Quote → WARNING „mehr als die Quote“; sonst über 90 % → WARNING 90 %.
   - 1.4 Start-Eintrag mit `plattenquote`, `belegt`, `plattenquote_option`.
2. **Senden** (`senden_mit`, unter Gesprächs- und Schreibsperre)
   - 2.1 Wiederholschlüssel prüfen → bekannt: ursprüngliche Nachricht (vor der Quote).
   - 2.2 Zeit vergeben, Satz bilden.
   - 2.3 Quote gesetzt und `belegt + Bytes(Satz) > Quote` → `Sendefehler::Quote { belegt, hoechstens }`.
   - 2.4 sonst anhängen, Index, `belegt += Bytes(Satz)` (+ 1 bei ergänztem Zeilenende), Wechsel nach oben → WARNING.
   - 2.5 Route: `Quote` → 507, `quote_abgewiesen` + 1, DEBUG, Episode je Kennung.
3. **Entfallen** (Pflege, ohne Schreibsperre)
   - 3.1 je entferntem Tag `belegt −= Bytes` (atomar).
   - 3.2 Wechsel nach unten → INFORMATION mit Anlass.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche es am Ende werden, entscheidet der Entwickler.

- F0074: B0352 Größe lesen · B0353 Wirksame Quote beim Start · B0354 Quote im Start-Eintrag und im Health-Check
- F0075: B0355 Belegt beim Laden · B0356 Quote vor dem Anhängen prüfen · B0357 Entfallen zieht ab · B0358 507 antworten · B0359 Quote über das echte Programm belegt (mit Testhilfe) · B0360 507 im Browser belegt
- F0076: B0361 Schwelle als Zustandswechsel · B0362 Wechsel melden beim Senden, Entfallen und Start · B0363 Abweisungen als Episode
- Nacharbeit Karte #34: F0075 B0486 507-Text groß nach Doppelpunkt

Elemente (Namen entscheidet der Entwickler):

- `Plattenquote` (Wert, rein) — `Bytes(u64)` | `Aus`; `fn lesen(text: &str) -> Result<Plattenquote, Groessenfehler>`.
- `Sendefehler::Quote { belegt: u64, hoechstens: u64 }` — neue Variante neben `Ablage`.
- Zähler `belegt: AtomicU64` in `Gespraeche` (einziger Halter, A00006 Ergänzung Karte #34).
- `Schwellenwechsel` (rein) — `fn wechsel(vorher: u64, nachher: u64, hoechstens: u64) -> Option<Richtung>`.
- `fn groesse_in_worten(bytes: u64) -> String` (rein) — für den 507-Text.

Wireframe: keine neue Gestaltung; die fehlgeschlagene Nachricht wie gebaut (R00011 F0043, R00012 F0046).

### Änderungen an bestehenden Klassen

- `gespraeche::Gespraeche::laden`: nimmt die Quote; summiert `belegt`.
- `gespraeche::Gespraeche::senden_mit`: Prüfung vor dem Anhängen, Erhöhen nach Erfolg.
- `gespraeche::Gespraeche::kennzahlen`: `log_bytes` aus dem Zähler.
- `gespraeche::pflege` (`pflegen`, `frist_setzen`): Abzug je entferntem Tag, Wechsel nach unten mit Anlass.
- `http::senden`: Zweig `Sendefehler::Quote`; `http::gesundheit`: zwei Felder.
- `start`: Option, Hilfe, Start-Eintrag, Start-Prüfung.

## Tests

Nach Skill `test-pyramide`; je Kriterium die genannte Mutante, das Ergebnis steht im Bericht des Entwicklers.

- **Unit (rein bzw. Speicherablage, gestellte Uhr)**: Größe lesen, Zähler und Grenze, Ohne Quote, Löschen scheitert, Größen in Worten, Schwelle als Zustandswechsel, Wechsel melden, Start.
- **Integration (echtes Programm)**: Falscher Wert beim Start, Hilfe, Wirksame Quote sichtbar, Health-Form, 507 im echten Programm, Platz über die Frist, Was an der Quote weitergeht, Ausgeschaltet, Protokoll im echten Programm, Episode je Kennung.
- **E2E**: Im Browser.

## Abhängigkeiten

- Abhängig von: I0009 Nachricht senden (gelb; Senderoute, Schreibkonto, Wiederholschlüssel — R00011), I0018 Aufbewahrungsfrist je Channel festlegen (gelb; Entfallen beim Start, Tageswechsel und Verkürzen — R00008; F0072 Frist bucht am Schreibkonto, R00021, für die Rechnung im Integrationsfall), I0021 Betriebsprotokoll (gelb; Start-Eintrag, Episoden, Health-Kennzahlen — R00014, R00016). Innerhalb: F0075 braucht F0074, F0076 braucht F0075.
- Nicht parallel, sondern nacheinander mit: Arbeiten an `src/http/senden.rs` (F0069, F0070 aus R00020) und `src/protokoll/episoden.rs` (F0073 aus R00022) — dieselben Dateien.
- Blockiert: — (I0010, I0012, I0022 bringen ihren Anteil am Zähler mit, sobald sie geplant werden; Hinweise in der WBS).

## Offene Fragen

Keine an den Betreiber; die Richtung ist entschieden (Betreiber 2026-10-29, ADR-0044).

Entschieden vom planer (Rückfragen mit Empfehlung beantwortet; den Wortlaut von ADR-0044 gleicht der architekt an):

- **„Über 90 %“ streng**: `belegt × 10 > Quote × 9`; ADR-0044 Regel 7 sagt „steigt über 90 %“ ohne Grenzfall.
- **Rundung und Tausenderpunkt** im 507-Text: halbe nach oben, Tausenderpunkt wie die 413-Texte (`mit_tausenderpunkt`); ADR-0044 Regel 4 nennt nur Einheit und Komma.
- **Start über der Quote**: nur die Zeile „mehr als die Quote — Senden wird abgewiesen“, keine zusätzliche 90-%-Zeile; ADR-0044 Regel 7 nennt beide mit „bzw.“.
- **Testhilfe für den Quote-Ordner** (B0359): das Gegeben „knapp unter 1 MiB“ ist über die API nur mit vielen Sendungen am Schreibkonto herstellbar; die Testhilfe macht es für Entwicklertest und Abnahme in einem Aufruf, wie `verlauf-erzeugen` für R00007.

## Manuelle Vorbereitungstätigkeiten

- Keine.

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md`: `--plattenquote` / `LOCALCHATA_PLATTENQUOTE` nachtragen (Director, nach dem Bau).
- Monitoring-Empfehlung (ADR-0044 Regel 9): Hinweis bei `log_bytes ≥ 0,9 × plattenquote`, Alarm bei steigendem `quote_abgewiesen` — in die Betriebsanleitung, sobald es eine gibt.
- Anleitung für Agenten (I0019): 507 nicht wiederholen, sondern mit `fehler` melden.

## Umfang

Gezählt aus der WBS (Bubbles). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; Richtwert, ungemessen für diese Bubble-Typen; 2-4 h ist die Bandbreite unklarer Bubbles.

- F0074 Quote einstellen: 3 Bubbles · 3 klar (1,2 h), keine unklaren
- F0075 Senden an der Quote abgewiesen: 6 Bubbles · 5 klar (2,0 h) · 1 unklar (B0359, 2–4 h)
- F0076 Quote im Betriebsprotokoll: 3 Bubbles · 2 klar (0,8 h) · 1 unklar (B0362, 2–4 h)
- I0023 gesamt: 12 Bubbles · 10 klar (4,0 h) · 2 unklar (4–8 h) · keine Ausbaustufen
- Nachtrag 2026-10-29 (Nacharbeit Karte #34): + B0486 unter F0075, klar (0,4 h). F0075 damit 7 Bubbles · 6 klar (2,4 h) · 1 unklar; R00023 zusammen 13 Bubbles · 11 klar (4,4 h) · 2 unklar (4–8 h); offen nur B0486.

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die offene Frage aus dem Security-Review #8 (ADR-0036, Ergänzung Karte #31 (a)): das Schreibkonto lässt eine Herkunft ≈ 5,7 GB je Tag schreiben, und der Verlaufsindex hält alles in der Frist im Speicher. Der Betreiber hat entschieden: „Ja, als Quote.“

Die Wirkungskette:

- Ein Zähler neben dem Verlaufsindex kennt die Bytes, die die Instanz hält. Deshalb ist dieselbe Zahl die Grenze für Platte und Speicher, und die Prüfung kostet je Senden nur einen Vergleich.
- Das Senden prüft vor dem Anhängen. Deshalb überschreiten die Gespräche die Quote nie, auch nicht bei dauerhafter Flut und langer Frist.
- An der Quote wird abgewiesen, nicht gelöscht. Deshalb bleibt die Frist die einzige Löschhandlung, wie die Vision es verlangt, und niemand verliert Geschriebenes.
- Die 90-%-Warnung kommt Stunden vor der ersten 507, und der 507-Text nennt den Ausweg. Deshalb kann der Betreiber eingreifen (Frist verkürzen, Quote anheben), bevor alle stumm sind, und jeder Absender weiß, was zu tun ist.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht); Begriffe aus R00008, R00011, R00014 und R00016.
- 507 nach RFC 4918 Abschnitt 11.5 — in ADR-0044 aus dem Gedächtnis zitiert, nicht geöffnet.

## Notizen

- **Änderung von Zusagen** (die genannten Anforderungen bleiben unverändert, R00023 nennt die Änderung):
  - R00011: das Senden kann zusätzlich 507 antworten (Schritt 9 der Prüfreihenfolge, ADR-0034 Ergänzung Karte #34).
  - R00014: der Start-Eintrag trägt zusätzlich `plattenquote`, `belegt`, `plattenquote_option`; `/api/gesundheit` zwei Felder mehr; `log_bytes` liest den Zähler (gleiche Bedeutung).
- **Optionen**: entschieden in ADR-0044 — Frage 1 (A abweisen mit Warnschwelle, gewählt; B ältestes zuerst löschen und C Frist automatisch verkürzen verworfen, weil sie eine zweite Löschhandlung neben der Frist wären; D nur warnen verworfen, weil es nicht begrenzt; E Anteil je Herkunft als Folgeoption, falls ein Einzelner die Quote wiederholt füllt), Frage 2 (M1 Zähler am Index, gewählt; M2 Summe je Senden verletzt die Sperrordnung; M3 Hülle um die Ablage wäre Fachlogik im Provider; M4 Ordner vermessen misst die falsche Größe), Frage 3 (S1 Option mit Einheit, gewählt), Frage 4 (507, gewählt; 429, 503, 409/422 verworfen).

### Nach dem Bau angeglichen (Karte #34, 2026-10-29)

Abweichungen, die der `entwickler` gebaut und begründet hat und die `code-reviewer` und `ux-reviewer` angenommen haben; der Code bleibt, die Anforderung zieht nach (`/anforderung review`, ohne Bau), damit Entwicklertest und Abnahme gegen das Gebaute prüfen:

- **Satzlänge**: R00023 rechnete 81 Bytes für einen Text zu 30 Zeichen ohne die Metazeile `+ art: agent` eines Agenten. Ein Text zu 17 Zeichen ergibt die 81 Bytes; alle Zahlen (1.048.519, 243, 1.048.600, 57 Bytes frei) gelten unverändert. Angeglichen: „Rechenbeispiele“, Kriterium „507 im echten Programm“, Vorspann der User Stories (trägt US-3, US-5, US-8).
- **Vorrang**: eine gültige Option übersteuert eine falsche Umgebung (ADR-0044 Ergänzung Karte #26) statt Exit 2. Angeglichen: „Falscher Wert“, Kriterien „Größe lesen“ und „Falscher Wert beim Start“.
- **Schlüssel** `plattenquote_option` statt `option` im Start-Eintrag (Hinweis aus Karte #26). Angeglichen: „Start-Eintrag“, Kriterium „Wirksame Quote sichtbar“, Ablauf 1.4, Notiz zu R00014, US-1.
- **Hilfe** umbrochen auf drei Zeilen wie die übrigen Optionen. Angeglichen: „Hilfe“ und Kriterium „Hilfe“.
- **Reihenfolge im Protokoll**: die 90-%-WARNING beim Start steht vor dem Start-Eintrag (nach dem Laden, vor der ersten Anfrage — wie im Kriterium „Protokoll im echten Programm“). Angeglichen: US-7.
- **Schreibung** „weitergeht“ (Rechtschreib-Review #34 Sollte). ADR-0044 und ADR-0042 zieht der architekt nach (Regel 4 Wortlaut, Katalog `plattenquote_option`).

### Nacharbeit Karte #34 (2026-10-29)

Weiche nach den Auto-Reviews #34: das Rechtschreib-Review Muss (507-Text klein nach dem Doppelpunkt) trifft einen Wortlaut, den diese Anforderung selbst vorgab — als Befund am eigenen Paket keine neue Interaction und kein `aus-bug`, sondern Nacharbeit unter derselben Anforderung (B0486). Wortlaut angeglichen in „Funktionale Anforderungen“ (Antwort), im Kriterium „507 im echten Programm“, in US-3 und US-4 und im Fertig-Kriterium von F0075. Die Sollte und Optional der Reviews #34 sind Härtung in R00031 (F0110–F0114).

### Bewusst außerhalb

- Anteil der Anhänge (I0010), der Direktgespräche (I0012) und des Archivs (I0022) am Zähler — bringen diese Interactions mit (ADR-0044 Folgeentscheidungen; Hinweise in der WBS).
- Live-Hinweis an die Tabs (ADR-0044 Regel 10).
- Quote zur Laufzeit ändern (ADR-0044 Option S4).
