---
id: R00025
status: Neu
datum: 2026-10-29
---

# R00025: Nachricht senden bei Störungen gehärtet

## Beschreibung

Das Senden bei Störungen aus R00012 schließt die Lücken, die die Auto-Reviews zu #22 gefunden haben und die nicht Muss waren. Danach gilt:

- **Eine Warnung, die sagt, wann es weitergeht**: bei getrenntem Strom steht genau eine Warnung „⚠ Verbindung verloren · neuer Versuch in 5 s“ mit einem Countdown bis zum nächsten Versuch. Der Verlauf springt dabei nicht. Wer mit einem Bildschirmleser liest, hört „Verbindung verloren“ und nach dem Nachholen „Wieder verbunden · 3 nachgeladen“.
- **Verwerfen ist nicht endgültig**: nach „Verwerfen“ steht 5 s lang „Verworfen · Rückgängig“; ein Klick bringt die Nachricht mit Text, Grund und Schaltern zurück.
- **Die Hinweiszeile hilft beim Bereinigen**: bei einem Steuerzeichen bietet sie den Schalter „Steuerzeichen entfernen“. Erst der Klick nimmt die verbotenen Zeichen aus dem Feld; ohne Klick wird nichts umgeformt. Ein Text nur aus Leerraum zeigt keinen Hinweis und nie eine negative Zahl.
- **Wiederholschlüssel halten unter Last**: ein Schlüssel gilt seine 10 Minuten auch dann, wenn andere Kennungen in dieser Zeit tausende Nachrichten senden; eine Kennung verdrängt nur ihre eigenen Schlüssel. Wird dennoch einer vor Ablauf verdrängt, sieht die Betreiberin das im Betriebsprotokoll und in `/api/gesundheit`. Die beiden Gründe für eine 422 `schluessel` stehen unterscheidbar im Protokoll.
- **Ein Weg zum Wiederbeitritt**: eine 401 beim Senden und eine 401 im Strom führen über denselben Weg zum selben Ergebnis.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ (wer die Verbindung verliert, weiß, wann es weitergeht, auch mit Bildschirmleser), „Das Log ist das Gedächtnis“ (keine Nachricht doppelt, auch unter dem Lastprofil), „Mensch und KI sind gleichberechtigt“ (der Wiederholschlüssel eines Agenten hält, egal wie viele andere schreiben), „eine gemeinsame LAN-Instanz“ (die Betreiberin sieht, wenn der Speicher nicht reicht). Kein Widerspruch zur Vision.

Herkunft: Karte #22 (I0020), Auto-Reviews vom 2026-10-29 — UX-Review Sollte F1, F2, F3, F4 und Optional „Countdown fehlt“; Observability-Review Sollte S1, S2; Security-Review Optional (Höchstzahl je Kennung); Code-Review Sollte A1, A2; Rechtschreib-Review Sollte. Schnitt des Directors: die Muss-Befunde (Code-Review B1–B3, mit A3) sind Nacharbeit #22 an R00012 (B0390–B0395), der Rest ist diese Anforderung; sie braucht die Nacharbeit.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0020 Nachricht senden bei Störungen und mit Komfort): F0084 Eine Verbindungswarnung mit Countdown, angesagt, F0085 Verwerfen lässt sich rückgängig machen, F0086 Hinweiszeile hilft beim Bereinigen, F0087 Wiederholschlüssel halten unter Last, F0088 Wiederbeitritt beim Senden auf dem Weg des Stroms.

**Architektur zuerst**: F0084, F0085, F0087 und F0088 hängen an Fragen an den `architekt` (WBS, Offene Fragen #22 Punkte 3–8); F0087 braucht vor dem Bau die Angriffsmatrix des `security-reviewer` (neue Grenze). F0086 ist ohne Architekturfrage baubar. Die Stellen, die eine Entscheidung abwarten, sind unten mit „(nach architekt, Punkt n)“ markiert; der planer zieht sie nach den Ergänzungen nach.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011, R00012 und R00014: Strom, getrennt, Leiste, Nachholen, Lücke, Rückzug (Wartezeiten bis zum nächsten Verbindungsversuch), vorläufig, fehlgeschlagen, Wiederholschlüssel, Kennung, Herkunft, Schreibkonto, Episode, Statusregion, Hinweiszeile, Textregel.

## Geschäftlicher Nutzen

Die Reviews zu #22 zeigen, wo das Senden bei Störungen an seinen Rändern stolpert. Bricht die Verbindung ab, erscheinen zwei Warnungen mit verschiedenen Worten, der Verlauf springt um 37 px, und niemand sagt, wann der nächste Versuch kommt; wer mit einem Bildschirmleser liest, hört nicht, dass die Verbindung wieder da ist. Wer „Verwerfen“ verklickt, hat seinen Text verloren. Eine eingefügte Terminalausgabe mit Farbcodes kann man nur Zeichen für Zeichen bereinigen, obwohl die Steuerzeichen unsichtbar sind. Ein langer Leerraum lässt die Hinweiszeile „noch -616 Zeichen“ sagen. Unter dem Lastprofil (Spitze 30 Nachrichten/s) hält der Wiederholspeicher mit 4.096 Einträgen nur etwa 136 s statt 10 min; eine einzige flutende Adresse verdrängt die Schlüssel aller anderen, und die Wiederholung eines Agenten steht dann doppelt im Channel — ohne Spur im Protokoll. Und eine 401 beim Senden nimmt einen anderen Weg als eine 401 im Strom.

Mit R00025 weiß die Betreiberin bei einem Abriss, woran sie ist, kann ein Verwerfen zurücknehmen und Farbcodes mit einem Klick entfernen. Ein Agent kann sich auf „genau einmal“ auch unter Last verlassen, und wenn der Speicher doch nicht reicht, sieht es die Betreiberin.

## Funktionale Anforderungen

### Eine Verbindungswarnung mit Countdown, angesagt (F0084; UX-Review #22 F1, F2, Optional „Countdown fehlt“)

- **Eine Warnung**: bei getrenntem Strom (R00012: Strom unterbrochen oder Netz des Tabs aus) ist genau ein Warnelement sichtbar. Heute sind es zwei — der Hinweis „⚠ Verbindung unterbrochen — localchatA verbindet neu.“ über der Seite (`#lc-verbindung`, R00005) und die Leiste „⚠ Verbindung verloren“ im Verlauf (R00012). Welches bleibt und wo es ohne offenen Channel steht, entscheidet der architekt (Punkt 4); der Wortlaut ist „⚠ Verbindung verloren“.
- **Countdown**: die Warnung nennt die Zeit bis zum nächsten Versuch des Stroms in ganzen Sekunden, aufgerundet, und zählt je Sekunde herunter: „⚠ Verbindung verloren · neuer Versuch in 5 s“; läuft ein Versuch, „⚠ Verbindung verloren · verbinde …“. Die Wartezeiten sind die des Stroms (sofort, 1 s, 2 s, 5 s, danach alle 10 s; `web/verbindung.js`). Woher der Tab den Zeitpunkt kennt, entscheidet der architekt (Punkt 3). Scheitert das Nachholen (R00012, Nacharbeit #22), zählt die Warnung bis zum nächsten Nachhol-Versuch.
- **Kein Sprung**: die Oberkante des Verlaufs (`.lc-log`) liegt vor, während und nach der Trennung an derselben Stelle (± 1 px).
- **Ansage**: die Statusregion (R00012 F0049) sagt „Verbindung verloren“, sobald die Warnung erscheint, und den Text der Meldung ohne ✓ („Wieder verbunden · 3 nachgeladen“, „Wieder verbunden“, „Wieder verbunden · neueste geladen“), sobald sie erscheint — sofort, außerhalb der Bündelung, wie ein Fehlschlag. Gesammelte fremde Nachrichten bleiben gesammelt. Ob dafür die Ansage-Region genügt oder eine zweite Region bleibt, entscheidet der architekt (Punkt 4; Empfehlung planer: eine Region für alle Ansagen).

### Verwerfen lässt sich rückgängig machen (F0085; UX-Review #22 F4)

- **Verwerfen** (R00012 F0046) entfernt eine fehlgeschlagene Nachricht nicht mehr sofort: an ihrer Stelle steht 5 s lang die Zeile „Verworfen“ mit dem Schalter „Rückgängig“, nicht höher als die Nachricht. Der Fokus liegt danach im Feld wie bisher.
- **Rückgängig** innerhalb der 5 s bringt die Nachricht unverändert zurück: Text, Grund, „Erneut senden“ und „Verwerfen“, derselbe Wiederholschlüssel; gesendet wird nichts. Der Fokus liegt danach auf „Erneut senden“ der Nachricht.
- Ab 5 000 ms nach „Verwerfen“ ist die Nachricht endgültig weg, wie bisher ohne Satz im Log.
- Channelwechsel und Neuladen innerhalb der 5 s: nach architekt (Punkt 5; Empfehlung planer: der Channelwechsel behält die Zeile, das Neuladen verwirft endgültig — wie fehlgeschlagene Nachrichten heute).

### Hinweiszeile hilft beim Bereinigen (F0086; UX-Review #22 F3, Code-Review #22 A1, Rechtschreib-Review #22 Sollte)

- **Schalter „Steuerzeichen entfernen“**: zeigt die Hinweiszeile „⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“ (R00012), steht daneben der Schalter „Steuerzeichen entfernen“. Ein Klick nimmt alle verbotenen Zeichen der Textregel aus dem Feld (C0 außer Tab und Zeilenumbruch, DEL, C1, ein Wagenrücklauf, der nicht unmittelbar vor einem Zeilenumbruch steht) — nur diese Zeichen, nicht ganze ANSI-Folgen. Danach werden Hinweis und Entwurf neu gesetzt, der Fokus liegt im Feld, die Schreibmarke am Ende.
- **Nichts wird automatisch umgeformt**: ohne Klick bleibt der Text, wie er eingefügt wurde; `Enter` sendet weiter nicht, solange ein Steuerzeichen darin steht (R00012).
- **Leerraum**: ein Text nur aus Leerraum (Unicode `White_Space`) zeigt keinen Hinweis und setzt kein `aria-invalid`, gleich wie viele Bytes er hat. Heute rechnet die Zeile auch dann „noch 16.384 − Bytes“ und zeigt ab 16.385 Bytes eine negative Zahl.
- Rechenbeispiel: 17.000 Leerzeichen → keine Zeile (heute „noch -616 Zeichen“); 17.000 Leerzeichen und „a“ → „⚠ Zu lang: 17.001 von höchstens 16.384 Byte — bitte teilen.“; 15.000 Leerzeichen und „a“ → „noch 1.383 Zeichen“.

### Wiederholschlüssel halten unter Last (F0087; Observability-Review #22 S1, S2, Security-Review #22 Optional)

- **Höchstzahl je Kennung zusätzlich zur Gesamtzahl**: der Speicher (R00012 F0046) hält je Kennung höchstens `H_k` und insgesamt höchstens `H_g` Einträge, je höchstens 10 Minuten alt. Über `H_k` fällt der älteste Eintrag dieser Kennung, über `H_g` der älteste insgesamt. Eine Kennung verdrängt so nie die Schlüssel einer anderen, solange `H_g` nicht erreicht ist. Die Werte setzt der architekt (Punkt 6); gerechnet sind je Kennung höchstens 630 Sendungen in 10 min (Schreibkonto 30 + 600 × 1/s, R00011 F0045) und im Lastprofil insgesamt 3.000 (dauerhaft 5/s) bis 18.000 (Spitze 30/s) — Empfehlung planer: `H_k` ≥ 630, `H_g` ≥ 18.000.
- **Verdrängung sichtbar**: fällt ein Eintrag vor Ablauf seiner 10 Minuten heraus, steht im Betriebsprotokoll eine WARNING-Episode „Wiederholschlüssel vor Ablauf verdrängt“ je Kennung (Mechanik R00014 F0055), und `/api/gesundheit` zählt jede Verdrängung (Zählername nach architekt, Punkt 7). Abgelaufene Einträge zählen nie als verdrängt.
- **Zwei Gründe für die 422 `schluessel`**: im Protokoll auf Debug trägt die Abweisung eines Kopfs mit falschem Format `grund=schluessel_format`, die eines Schlüssels mit anderem Text oder Channel `grund=schluessel_anders` (Mechanik `Abweisung` aus R00018 F0059, B0303; Schlüsselnamen nach architekt, Punkt 7). Die Antwort bleibt 422 `regel:"schluessel"` mit ihren Texten.
- Ändert R00012 F0046: „höchstens die 4 096 jüngsten Einträge“ wird „höchstens `H_k` je Kennung und `H_g` insgesamt“.

### Wiederbeitritt beim Senden auf dem Weg des Stroms (F0088; Code-Review #22 A2)

- Eine 401 auf das Senden und eine 401 im Strom treten über **denselben** Weg wieder bei (heute zwei: `web/start.js` `stillWiederBeitreten` für den Strom, `nachSendeAblehnungBeitreten` für das Senden): hat ein anderer Tab schon ein neues Token gemerkt, gilt es (kein `POST /api/beitreten`); sonst tritt der Tab still mit dem gemerkten Namen bei, je Browser nur ein Tab zugleich (Web Lock). Danach bekommen Strom und Senden dasselbe Token, und der eine neue Versuch des Sendens (R00012) trägt es.
- Scheitert der stille Beitritt (Name inzwischen vergeben, Fehler), verhält sich der Tab auf beiden Wegen gleich — wie, entscheidet der architekt (Punkt 8: Beitreten-Bildschirm, Warteschlange, Entwurf).

## Nicht-funktionale Anforderungen

- **Zeiten** — die Grenzen stehen nur hier; Tests übernehmen sie wörtlich und sind nie schärfer. Countdown und 5 s „Rückgängig“ prüfen Node-Tests exakt mit gestellter Zeit; die Tabelle nennt die Zustellung.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | Z14 | Wiederverbinden angesagt | Gate (E2E) | Netz von B wieder an → Statusregion enthält „Wieder verbunden“ | Z6 (≈ 50 ms + ein Abruf) + ein Frame | < 2 s | wer mit Bildschirmleser liest, erfährt so schnell wie die Sehenden, dass es weitergeht |
  | Z15 | Rückgängig verfällt | Gate (E2E) — Zustellung; Regel exakt im Node-Test | Klick „Verwerfen“ | Zeile weg nach 5 s + ein Frame | 4 s nach dem Klick noch da, spätestens 7 s danach weg | ein verklicktes Verwerfen lässt sich zurücknehmen, ohne dass die Zeile lange stehen bleibt |

  Reserve: Z14 > 10 × über dem erwarteten Wert (Grenze aus der Probe des ux-reviewer); Z15 40 % über der Regel (5 s → 7 s), Untergrenze 1 s unter der Regel.
- **Sicherheit**: die Höchstzahl je Kennung verhindert, dass eine Kennung oder eine Adresse die Schlüssel anderer verdrängt; der Speicher bleibt begrenzt (`H_g` Einträge, 10 min). Ein Schlüssel erscheint weiter nie in Log, Antwort oder Ereignis — auch nicht in der neuen Episode. Vor dem Bau schreibt der `security-reviewer` die Angriffsmatrix der Grenze (Flut über viele Kennungen einer Adresse, über viele Adressen, Speicherbedarf bei `H_g`).
- **Benutzerfreundlichkeit**: eine Warnung statt zwei, ohne Sprung; „Rückgängig“ und „Steuerzeichen entfernen“ sind Schalter in der Tab-Reihenfolge; bei 320 px kein Querscrollen mit Warnung, „Verworfen · Rückgängig“ und dem Schalter in der Hinweiszeile.
- **Nachvollziehbarkeit**: die Verdrängung eines Schlüssels steht als Episode im Protokoll und als Zähler im Health-Check; die zwei Gründe der 422 `schluessel` sind unterscheidbar.
- **Testbarkeit**: Countdown-Text, verworfen auf Zeit, Steuerzeichen entfernen, Hinweis und Speicher als reine Funktionen mit gestellter Zeit; Verdrängung über das echte Programm.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien wie in R00012: Start, `S(<name>)`, `alpha`, Browser A (`shoff`) und B (`anna`), „Strom getrennt“ (`context.setOffline(true)`) und „Strom wieder“. `heute.log` ist `logs/<heute>.log`.

### F0084 Eine Verbindungswarnung mit Countdown, angesagt

- [ ] Countdown-Text (Node-Test, rein, gestellte Zeit): nächster Versuch in 5 000 ms → „⚠ Verbindung verloren · neuer Versuch in 5 s“; 4 001 ms → „in 5 s“; 4 000 ms → „in 4 s“; Versuch läuft → „⚠ Verbindung verloren · verbinde …“. Die Mutante „abgerundet“ wird rot.
- [ ] Eine Warnung ohne Sprung (E2E; Abnahme-Probe aus dem UX-Review #22, F1, wörtlich übernommen): B hat `# allgemein` gewählt; Strom getrennt, dann wieder → „genau ein Element, Oberkante .lc-log unverändert vor/während/nach der Trennung“ — genau ein sichtbares Element mit „Verbindung verloren“ oder „Verbindung unterbrochen“ auf der Seite, die Oberkante von `.lc-log` ändert sich um höchstens 1 px. Vor der Härtung rot (zwei Elemente, 37 px).
- [ ] Countdown im Tab (E2E): B getrennt → die Warnung nennt „neuer Versuch in“ mit einer Zahl von 1 bis 10 und zählt herunter (zwei Lesungen im Abstand von 1 s unterscheiden sich).
- [ ] Ansage (E2E, **Z14**; Abnahme-Probe aus dem UX-Review #22, F2, wörtlich übernommen: „Statusregion enthält „Wieder verbunden“ binnen 2 s“): B getrennt → die Statusregion enthält „Verbindung verloren“; `beta` sendet 3; Strom wieder → binnen **Z14** enthält sie „Wieder verbunden · 3 nachgeladen“. Vor der Härtung rot.
- [ ] Ansage unabhängig von der Bündelung (Node-Test, rein, gestellte Zeit): t = 0 fremd → „Neue Nachricht von …“; t = 2 s fremd (gesammelt); t = 3 s wieder verbunden mit 0 → sofort „Wieder verbunden“; t = 10 s → „1 neue Nachricht in # allgemein“. Die Mutante „durch die Bündelung“ wird rot.
- [ ] Gescheitertes Nachholen (E2E, wie R00012 „Gescheitertes Nachholen“): solange das Nachholen scheitert, enthält die Statusregion kein „Wieder verbunden“.
- [ ] R00005 „Zuletzt online“ (E2E): nach der Entscheidung des architekt (Punkt 4) mit dem einen Warnelement statt des Hinweises „Verbindung unterbrochen“; die übrigen Zusagen des Kriteriums bleiben.
- [ ] Schmal (E2E): bei 320 px mit sichtbarer Warnung `scrollWidth <= innerWidth`.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zu Anfragen, die bei getrenntem Netz scheitern.

### F0085 Verwerfen lässt sich rückgängig machen

- [ ] Verworfen auf Zeit (Node-Test, rein, gestellte Zeit): fehlgeschlagene `x` mit Schlüssel `k`; „Verwerfen“ bei t = 0 → verworfen; „Rückgängig“ bei 4 999 ms → fehlgeschlagen mit Grund und Schlüssel `k`, kein Auftrag; erneut „Verwerfen“ bei t = 10 s → bei 14 999 ms verworfen, bei 15 000 ms entfernt. Die Mutanten „4 s“, „6 s“ und „Rückgängig sendet“ werden je rot.
- [ ] Rückgängig (E2E; Abnahme-Probe aus dem UX-Review #22, F4: „5 s „Verworfen · Rückgängig““): wie R00012 „Verwerfen“ mit `drei`; Klick „Verwerfen“ → an der Stelle steht „Verworfen“ mit „Rückgängig“, der Fokus liegt im Feld; Klick „Rückgängig“ → `drei` zeigt wieder ⚠ „Keine Antwort vom Server.“ mit „Erneut senden“ und „Verwerfen“, laut Netzprotokoll kein `POST`, der Fokus liegt auf „Erneut senden“.
- [ ] Verfallen (E2E, **Z15**): Klick „Verwerfen“ → 4 s danach steht „Verworfen“ noch, spätestens **Z15** danach ist die Zeile weg; kein Satz `drei` im Log.
- [ ] Schmal (E2E): bei 320 px mit „Verworfen · Rückgängig“ `scrollWidth <= innerWidth`.
- [ ] Konsole bei A: wie R00012 „Verwerfen“.

### F0086 Hinweiszeile hilft beim Bereinigen

- [ ] Steuerzeichen entfernen (Node-Test, rein; Zeichen als Escape): `"rot \u001b[31mFehler\u001b[0m"` → `"rot [31mFehler[0m"`, 2 entfernt; `"a\rb"` → `"ab"`, 1; `"a\r\nb"`, `"a\tb"`, `"👩‍💻"` → unverändert, 0; jedes Ergebnis ergibt in der Textregel kein `zeichen`. Die Mutante „Wagenrücklauf vor Umbruch entfernt“ wird rot.
- [ ] Schalter (E2E): A fügt `rot ` + `\u001b` + `[31m` ein → „⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“ und der Schalter „Steuerzeichen entfernen“; `Enter` → kein `POST`, das Feld ist unverändert (nichts umgeformt); Klick → das Feld enthält `rot [31m`, die Hinweiszeile ist leer, `aria-invalid` ist weg, der Fokus liegt im Feld; `Enter` → 201.
- [ ] Ohne Steuerzeichen kein Schalter (E2E): „Zu lang“ und „noch n Zeichen“ zeigen keinen Schalter.
- [ ] Leerraum (Node-Test, rein): die drei Fälle des Rechenbeispiels; dazu 14.999 Leerzeichen → keine Zeile. Die Mutante „leer wie ok“ wird rot.
- [ ] Schmal (E2E): bei 320 px mit Hinweis und Schalter `scrollWidth <= innerWidth`.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß, keine Konsolenfehler.

### F0087 Wiederholschlüssel halten unter Last

- [ ] Speicher (Unit, rein, gestellte Uhr; kleine Grenzen über den Konstruktor, je Kennung 3, gesamt 5): A merkt `a1`; B merkt `b1`–`b4` → `b1` verdrängt (Kennung B, eigener Eintrag), `a1` noch bekannt (wiederholt); C merkt `c1`, `c2` → beim zweiten ist die Gesamtgrenze überschritten, der älteste insgesamt fällt, gemeldet mit Kennung und Alter; ein Eintrag, der älter als 600 000 ms ist, fällt ohne Meldung. Die Mutanten „nur Gesamtgrenze“ und „abgelaufen als verdrängt“ werden je rot. Mit den echten Konstanten: `H_k` und `H_g` wie vom architekt gesetzt (Test liest die Konstanten).
- [ ] Verdrängung sichtbar (Unit, Speicher-Senke): eine Verdrängung → genau eine Beginn-Zeile `WARNING … Wiederholschlüssel vor Ablauf verdrängt — Episode beginnt` mit `kennung=` der verdrängten Kennung, kein Schlüssel im Text oder Kontext; Zähler im Health-Check 1; ein abgelaufener Eintrag → keine Zeile, Zähler 0. Die Mutante „keine Meldung“ wird rot.
- [ ] Unter Last (Integration über das echte Programm; Abnahme-Probe aus dem Observability-Review #22, S1, wörtlich übernommen): „alfa sendet „Wichtig“ mit auftrag-1; 160 Kennungen senden 4.480 Nachrichten; alfa wiederholt → erwartet WARNING-Episode „Wiederholschlüssel vor Ablauf verdrängt“, Health-Zähler ≥ 1, bei geänderter Grenze 200 und nur ein Satz; Gegenprobe mit gestellter Uhr > 10 min → keine Zeile.“ Lesart: mit `H_g` und `H_k` aus Punkt 6 antwortet die Wiederholung 200, die Tagesdatei enthält „Wichtig“ genau einmal, keine Episode; mit einer über den Testknopf des architekt (Punkt 7) auf 4.096 verkleinerten Gesamtgrenze erscheinen die Episode und Zähler ≥ 1. Vor der Härtung rot (201, zwei Sätze).
- [ ] Fremde verdrängen nicht (Integration über das echte Programm; Abnahme-Probe aus dem Security-Review #22, Optional, wörtlich übernommen): „A merkt v1; 4.096 Sendungen anderer Kennungen; A wiederholt v1 → 200, eine Zeile; Kontrolle A selbst 4.097 neue → v1 vergessen.“ Die Kontrolle „A selbst 4.097 neue“ ist über das echte Programm nicht innerhalb der 10 min herstellbar (das Schreibkonto erlaubt je Kennung höchstens 630); sie läuft als „Speicher“ oben gegen `H_k`.
- [ ] Zwei Gründe (Integration, Stufe Debug): Kopf `Idempotency-Key: a b` → eine Zeile mit `status=422` und `grund=schluessel_format`; `beta` sendet `Hallo` mit `k1`, dann `Hallo!` mit `k1` → eine Zeile mit `grund=schluessel_anders`; beide Antworten unverändert 422 `regel:"schluessel"`; kein `k1` im Protokoll. Die Mutante „ein Grund für beide“ wird rot.

### F0088 Wiederbeitritt beim Senden auf dem Weg des Stroms

- [ ] Ein Weg (Node-Test mit gestellter API): 401 auf das Senden, ein anderer Tab hat schon ein neues Token geschrieben → kein `POST /api/beitreten`, der neue Versuch trägt dieses Token, Strom und Senden haben dasselbe; ohne neues Token → genau ein `POST /api/beitreten` mit dem gemerkten Namen und danach der Versuch mit dem neuen Token. Die Mutante „Senden ohne Geschwister-Token“ wird rot.
- [ ] 401 (E2E): R00012 „401“ bleibt grün.
- [ ] Scheitern (E2E): nach der Entscheidung des architekt (Punkt 8) — eine 401 auf das Senden, deren stiller Beitritt scheitert, führt zum selben sichtbaren Ergebnis wie eine 401 im Strom, deren stiller Beitritt scheitert.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/verbindungsleiste.js`, `web/hauptansicht.js`, `web/lc-seite.css`, `web/ansage.js`, `web/live.js`, `web/strom.js`, `web/start.js` (F0084, F0088); `web/zustellung.js`, `web/verlauf.js`, `web/senden.js` (F0085); `web/textregel.js`, `web/hinweis.js`, `web/eingabe.js` (F0086).
- Server: `src/gespraeche/wiederholung.rs`, `src/gespraeche/` (Meldung an das Protokoll), `src/http/senden.rs`, Gesundheit (F0087).
- Tests: Unit im Modul; Integration `tests/senden_wiederholung.rs`; Oberfläche rein `e2e/einheit/`; Abläufe `e2e/tests/r00025-*.spec.js`; R00005-E2E zieht mit (F0084).

## Technische Überlegungen

### Ablauf

1. **Trennung**: Strom unterbrochen → Rückzug im Strom mit Zeitpunkt des nächsten Versuchs → Tab → eine Warnung mit Countdown → Ansage „Verbindung verloren“; wieder verbunden → Nachholen (R00012) → Meldung → Ansage.
2. **Verwerfen**: fehlgeschlagen → „Verwerfen“ → verworfen{seit} → „Rückgängig“ → fehlgeschlagen | 5 s → entfernt.
3. **Bereinigen**: Textregel → Hinweis `zeichen` → Schalter → Klick → Text ohne verbotene Zeichen → Feld, Entwurf, Hinweis.
4. **Senden mit Schlüssel** (R00012) → Speicher: neu | wiederholt | anders, dazu verdrängt → Episode und Zähler; 422 → `Abweisung` mit `schluessel_format` | `schluessel_anders`.
5. **401** (Strom oder Senden) → ein Weg zum Wiederbeitritt → Token für beide.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0084: B0396 Warnung mit Countdown formen (rein) · B0397 Nächster Versuch des Stroms im Tab (unklar) · B0398 Eine Warnung ohne Layoutsprung (unklar) · B0399 Trennen und Wiederverbinden ansagen
- F0085: B0400 Verworfen auf Zeit in der Zustellung (rein) · B0401 Platzhalter „Verworfen · Rückgängig“ · B0402 Rückgängig verdrahten
- F0086: B0403 Steuerzeichen entfernen (rein) · B0404 Schalter „Steuerzeichen entfernen“ · B0405 Leerraum ohne Hinweis (rein)
- F0087: B0406 Speicher je Kennung und gesamt (rein, unklar) · B0407 Verdrängung sichtbar · B0408 Verdrängung über das echte Programm geprobt (unklar) · B0409 Zwei Gründe für die 422 `schluessel`
- F0088: B0410 Ein Weg zum Wiederbeitritt (unklar)

**Elemente** (nur öffentliche Signaturen, Vorschlag): `leisteInhalt(zustand, jetztMs)` mit Zeitpunkt des nächsten Versuchs; `ansage(zustand, {art: 'verbindung', text}, jetzt)`; `zustellen(zustand, {art: 'rueckgaengig' | 'verfallen', …})`; `steuerzeichenEntfernen(text) → {text, entfernt}`; `Wiederholung::pruefen(…) -> (Ergebnis, Option<Verdraengt>)` mit Konstanten `HOECHSTENS_JE_KENNUNG`, `HOECHSTENS_EINTRAEGE`.

Wireframe: Artboard `D0003.dc.html` fehlt. Gestaltungsquelle: Designsystem `components/VerlaufsMarken/README.md` („Verbindung“: Leiste mit Countdown), `components/Nachricht/README.md` (fehlgeschlagen, Schalter), `components/Eingabe/README.md` — Vorgabe, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/hauptansicht.js` `verbindungZeigen` und `wiederverbindenEinrichten` (eine Warnung), `web/start.js` (ein Weg zum Wiederbeitritt), `web/zustellung.js` (verworfen auf Zeit), `web/hinweis.js` (leer ohne Hinweis), `src/gespraeche/wiederholung.rs` (zwei Grenzen, Verdrängung).

## Tests

Nach Skill `test-pyramide` und ADR-0009; Tests sind nie schärfer als diese Anforderung und übernehmen Z14 und Z15 wörtlich. **Unit:** Speicher mit zwei Grenzen, Verdrängung mit Speicher-Senke. **Integration:** unter Last und fremde verdrängen nicht über das echte Programm (Gelerntes der Factory: Probe gegen den echten Einstieg), zwei Gründe auf Stufe Debug. **Oberfläche rein:** Countdown-Text, Ansage, verworfen auf Zeit, Steuerzeichen entfernen, Leerraum, ein Weg zum Wiederbeitritt. **E2E:** eine Warnung ohne Sprung, Countdown, Ansage (Z14), Rückgängig und Verfallen (Z15), Schalter „Steuerzeichen entfernen“, schmal; R00005 „Zuletzt online“ zieht mit.

## Abhängigkeiten

- Abhängig von: Nacharbeit #22 an R00012 (B0390–B0395, rot) — F0084 braucht F0047 (Leiste und Fehlerfall des Nachholens, B0393) und F0049 (Statusregion); F0085 und F0088 brauchen F0046; F0086 braucht F0048; F0087 braucht F0046, F0055 (Episoden, R00014, grün) und F0059 (`Abweisung`, R00018, Karte #26, rot).
- Innerhalb: F0085, F0086, F0088 parallel; F0084 nach B0393; F0087 nach F0059.
- Vor dem Bau: Antworten des architekt auf die Punkte 3–8 (WBS, Offene Fragen #22) und Angriffsmatrix des security-reviewer für F0087.
- Blockiert: nichts.

## Offene Fragen

An den architekt (über den Director; WBS, Offene Fragen #22):

- Punkt 3 — Quelle des Countdowns über die Grenze des geteilten Workers.
- Punkt 4 — Ort der einen Warnung (mit und ohne offenen Channel, Überlagern oder fester Platz), eine oder zwei Statusregionen; ändert R00005 „Zuletzt online“.
- Punkt 5 — Zustand des Rückgängig bei Channelwechsel und Neuladen.
- Punkt 6 — `H_k` und `H_g` (ADR-0034 Ergänzung).
- Punkt 7 — Katalogzeile der Episode, Zählername im Health-Check, Regelschlüssel `schluessel_format`/`schluessel_anders`, Testknopf für eine kleine Gesamtgrenze.
- Punkt 8 — Folge eines gescheiterten stillen Beitritts aus dem Senden.

An den Betreiber: keine.

Entschieden vom planer für R00025 (Rückfragen mit Empfehlung beantwortet):

- **„Steuerzeichen entfernen“ nimmt nur die verbotenen Zeichen**, nicht ganze ANSI-Folgen — vorhersagbar und gleich der Regel, die die Hinweiszeile nennt; das Entfernen ganzer Folgen wäre Umformen über die Regel hinaus.
- **Leerraum zeigt keinen Hinweis** statt „noch 0 Zeichen“ oder „Zu lang“ — ein Text nur aus Leerraum wird ohnehin nicht gesendet (R00011), die Zeile hätte nichts zu sagen.
- **Ansage ohne ✓**, mit dem Text der Leiste — das Zeichen hat für den Bildschirmleser keinen Inhalt.
- **Fokus nach „Rückgängig“ auf „Erneut senden“** — wer zurücknimmt, will meist erneut senden; nach „Verwerfen“ bleibt der Fokus im Feld wie in R00012.
- **Wortlaut des Countdowns** „neuer Versuch in 5 s“ — Vorschlag; ux- und rechtschreib-reviewer prüfen.

## Manuelle Vorbereitungstätigkeiten

- architekt beantwortet die Punkte 3–8; danach zieht der planer R00025 nach (Werte `H_k`, `H_g`, Namen, Ort der Warnung, Verhalten bei Scheitern).
- security-reviewer schreibt die Angriffsmatrix für F0087; ohne Matrix startet F0087 nicht.

## Manuelle Nachbereitungstätigkeiten

- Designsystem-README „Verbindung verloren.“ ohne Punkt angleichen und `VerlaufsMarken` um „Verworfen · Rückgängig“ ergänzen (über den Director).
- `AI-Team/umgebung.md`: neuen Testknopf aus Punkt 7 nachtragen (Director).

## Umfang

Aus der WBS gezählt; Standard-Bubbles mit der Untergrenze 0,4h, unklare als Richtwert 2–4h (ungemessen):

- F0084 = 4 Bubbles: 2 Standard (0,4h), 2 unklar (B0397, B0398). Rest: 0,8h klar + 4–8h unklar.
- F0085 = 3 Bubbles: 3 Standard. Rest: 1,2h klar.
- F0086 = 3 Bubbles: 3 Standard. Rest: 1,2h klar.
- F0087 = 4 Bubbles: 2 Standard, 2 unklar (B0406, B0408). Rest: 0,8h klar + 4–8h unklar.
- F0088 = 1 Bubble: 1 unklar (B0410). Rest: 2–4h unklar.
- R00025 gesamt = 15 Bubbles: 10 Standard, 5 unklar. Rest: 4,0h klar + 10–20h unklar (Richtwerte, ungemessen).
- Nicht enthalten: Nacharbeit #22 (6 Bubbles, R00012); die optionalen Punkte der Reviews (WBS, Offene Fragen #22).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Unter dem Lastprofil schreiben bis zu 50 Agenten, in der Spitze 30 Nachrichten je Sekunde. Der Wiederholspeicher mit 4.096 Einträgen gesamt ist dann nach etwa 136 s voll und verdrängt den Schlüssel eines Agenten, dessen Antwort unterwegs verloren ging; seine Wiederholung legt eine zweite Nachricht an, und andere Agenten arbeiten sie doppelt ab. Wenn der Speicher je Kennung begrenzt ist und insgesamt für 10 Minuten des Lastprofils reicht, dann verdrängt keine Kennung die Schlüssel einer anderen, und „genau einmal“ gilt wieder für die ganze Zusage; wenn eine Verdrängung trotzdem geschieht, dann sieht die Betreiberin sie im Protokoll und im Health-Check, bevor jemand Dubletten meldet. Auf der Seite des Menschen: wenn bei einem Abriss genau eine Warnung mit Countdown steht und angesagt wird, dann weiß jeder — sehend oder mit Bildschirmleser —, dass die Instanz es selbst weiter versucht, und lädt nicht neu. Wenn „Verwerfen“ 5 s zurücknehmbar ist und Steuerzeichen mit einem Klick verschwinden, dann kostet ein Fehlgriff oder eine eingefügte Terminalausgabe keinen neu getippten Text. Der Hebel ist die Grenze je Kennung: sie macht die Zusage des Wiederholschlüssels unabhängig davon, wie viele andere schreiben.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).
- Kein Artboard für D0003.
- ADR-Ergänzungen zu den Punkten 3–8 stehen aus.

## Notizen

### Verworfene Alternativen

- **Nur die Gesamtzahl erhöhen** (ohne Grenze je Kennung): verworfen — eine flutende Adresse verdrängt dann weiter die Schlüssel aller anderen, nur später.
- **Rückgängig ohne Zeitgrenze** (verworfene Nachrichten in einem Papierkorb): verworfen — die Zeile bliebe stehen, und eine fehlgeschlagene Nachricht ist kein Dokument; 5 s ist der Vorschlag des ux-reviewer.
- **Steuerzeichen beim Einfügen still entfernen**: verworfen — R00012 sagt „Umgeformt wird nichts“; ein Mensch soll sehen, dass sein Text Steuerzeichen enthielt.
- **Eine Anforderung je Slice**: verworfen — die fünf Slices sind das Arbeitspaket „Härtung #22“ und durchlaufen gemeinsam Entwicklertest und Abnahme.

### Bewusst außerhalb

Optionale Punkte der Reviews #22 (WBS, Offene Fragen #22, „nicht eingeplant“): `wiederholt` als Health-Zähler, „Verlaufsblock gelesen“ mit `kennung` und Richtung, Spur für Senden langsamer als die Zeitgrenze des Browsers, Gruppierung vorläufiger Nachrichten, Erfolgsansage nach „Erneut senden“, `aria-describedby` der Schalter, Entwurf in der Seitenleiste, Feldwerte im 422-Text in „…“, doppeltes „Nicht gesendet“ im Text der 502, „Verlassen“ → „Weggehen“ in Testnamen; `cargo audit` im Gate (Director).
