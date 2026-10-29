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
- **Ein Weg zum Wiederbeitritt**: eine 401 beim Senden und eine 401 im Strom führen über denselben Weg zum selben Ergebnis — auch wenn der stille Beitritt scheitert: dann zeigt der Tab den Beitreten-Bildschirm, und ungesendete Nachrichten gehen nicht verloren, sondern stehen nach dem nächsten Beitritt mit „Erneut senden“ da.

Zahlt ein auf: [Vision](R00000-vision.md) — „Live überall“ (wer die Verbindung verliert, weiß, wann es weitergeht, auch mit Bildschirmleser), „Das Log ist das Gedächtnis“ (keine Nachricht doppelt, auch unter dem Lastprofil), „Mensch und KI sind gleichberechtigt“ (der Wiederholschlüssel eines Agenten hält, egal wie viele andere schreiben), „eine gemeinsame LAN-Instanz“ (die Betreiberin sieht, wenn der Speicher nicht reicht). Kein Widerspruch zur Vision.

Herkunft: Karte #22 (I0020), Auto-Reviews vom 2026-10-29 — UX-Review Sollte F1, F2, F3, F4 und Optional „Countdown fehlt“; Observability-Review Sollte S1, S2; Security-Review Optional (Höchstzahl je Kennung); Code-Review Sollte A1, A2; Rechtschreib-Review Sollte. Schnitt des Directors: die Muss-Befunde (Code-Review B1–B3, mit A3) sind Nacharbeit #22 an R00012 (B0390–B0395), der Rest ist diese Anforderung; sie braucht die Nacharbeit.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0020 Nachricht senden bei Störungen und mit Komfort): F0084 Eine Verbindungswarnung mit Countdown, angesagt, F0085 Verwerfen lässt sich rückgängig machen, F0086 Hinweiszeile hilft beim Bereinigen, F0087 Wiederholschlüssel halten unter Last, F0088 Wiederbeitritt beim Senden auf dem Weg des Stroms.

**Architektur entschieden** (2026-10-29; WBS, Offene Fragen #22 Punkte 3–8): [ADR-0015](../Dokumentation/ADRs/0015-ein-ereignisstrom-je-browser-sharedworker-mit-sitzungs-header.md) (Zeitpunkt des nächsten Versuchs), [ADR-0038](../Dokumentation/ADRs/0038-eingabe-mehrzeilig-enter-sendet-entwurf-je-channel-gebuendelte-ansage.md) (eine Verbindungswarnung, eine Live-Region), [ADR-0037](../Dokumentation/ADRs/0037-zustellung-im-tab-vorlaeufig-bestaetigt-zeichnen-je-frame.md) (Verworfen auf Zeit), [ADR-0034](../Dokumentation/ADRs/0034-nachricht-senden-post-nachrichten-text-hoechstens-16-kib-steuerzeichen-abweisen-wiederholschluessel.md) (Höchstzahl je Kennung und gesamt), [ADR-0042](../Dokumentation/ADRs/0042-stufen-ereigniskatalog-episoden-und-tagesgrenze-des-betriebsprotokolls.md) und [ADR-0043](../Dokumentation/ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) (Katalogzeile, Regelschlüssel, Testknopf, Zähler), [ADR-0012](../Dokumentation/ADRs/0012-identitaet-und-sitzung.md) (ein Weg zum Wiederbeitritt), je Ergänzung Karte #36; Modell A00005, A00006, A00011, A00015. Angeglichen am 2026-10-29 (planer, `/anforderung review`, Karte #36). **Vor dem Bau von F0087** fehlt noch die Angriffsmatrix des `security-reviewer` (manuelle Vorbereitung unten); F0084, F0085, F0086 und F0088 haben keine offene Vorbedingung außer der Nacharbeit #22.

### Domänenbezug

Kein Glossar im Projekt (`Dokumentation/Domaenenwissen/` fehlt). Begriffe wie in R00011, R00012 und R00014: Strom, getrennt, Leiste, Nachholen, Lücke, Rückzug (Wartezeiten bis zum nächsten Verbindungsversuch), vorläufig, fehlgeschlagen, Wiederholschlüssel, Kennung, Herkunft, Schreibkonto, Episode, Statusregion, Hinweiszeile, Textregel.

## Geschäftlicher Nutzen

Die Reviews zu #22 zeigen, wo das Senden bei Störungen an seinen Rändern stolpert. Bricht die Verbindung ab, erscheinen zwei Warnungen mit verschiedenen Worten, der Verlauf springt um 37 px, und niemand sagt, wann der nächste Versuch kommt; wer mit einem Bildschirmleser liest, hört nicht, dass die Verbindung wieder da ist. Wer „Verwerfen“ verklickt, hat seinen Text verloren. Eine eingefügte Terminalausgabe mit Farbcodes kann man nur Zeichen für Zeichen bereinigen, obwohl die Steuerzeichen unsichtbar sind. Ein langer Leerraum lässt die Hinweiszeile „noch -616 Zeichen“ sagen. Unter dem Lastprofil (Spitze 30 Nachrichten/s) hält der Wiederholspeicher mit 4.096 Einträgen nur etwa 136 s statt 10 min; eine einzige flutende Adresse verdrängt die Schlüssel aller anderen, und die Wiederholung eines Agenten steht dann doppelt im Channel — ohne Spur im Protokoll. Und eine 401 beim Senden nimmt einen anderen Weg als eine 401 im Strom.

Mit R00025 weiß die Betreiberin bei einem Abriss, woran sie ist, kann ein Verwerfen zurücknehmen und Farbcodes mit einem Klick entfernen. Ein Agent kann sich auf „genau einmal“ auch unter Last verlassen, und wenn der Speicher doch nicht reicht, sieht es die Betreiberin.

## Funktionale Anforderungen

### Eine Verbindungswarnung mit Countdown, angesagt (F0084; UX-Review #22 F1, F2, Optional „Countdown fehlt“)

- **Eine Warnung**: bei getrenntem Strom (R00012: Strom unterbrochen oder Netz des Tabs aus) ist genau ein Warnelement sichtbar — die Leiste „⚠ Verbindung verloren“, die oben über der Mitte liegt (`.lc-leiste` in `.lc-center`, überlagernd wie Ladezeile und Pille), mit und ohne offenen Channel, einmal je Hauptansicht (ein Channelwechsel ersetzt sie nicht). Der Hinweis „⚠ Verbindung unterbrochen — localchatA verbindet neu.“ über der Seite (`#lc-verbindung`, `role="alert"`, R00005) entfällt samt seiner Zeile. Steht die Ladezeile zugleich, liegt sie direkt unter der Leiste; ohne offenen Channel verdeckt die Leiste den Anfang des Hinweises der Mitte, er rückt nicht. „Stand unbekannt“ in der Teilnehmerliste (R00005) bleibt (ADR-0038 Ergänzung Karte #36).
- **Countdown**: die Warnung nennt die Zeit bis zum nächsten Versuch in ganzen Sekunden, aufgerundet, und zählt im Sekundentakt des Tabs herunter: „⚠ Verbindung verloren · neuer Versuch in 5 s“; ist der Zeitpunkt erreicht, „⚠ Verbindung verloren · verbinde …“. Der Strom meldet den Zeitpunkt als absolute Zeit `naechsterVersuchUm` mit dem Zustand `unterbrochen`, auch an Tabs, die erst während der Wartezeit dazukommen; der Rest wird auf 0 bis 10 s begrenzt (ADR-0015 Ergänzung Karte #36). Welcher Zeitpunkt gilt: Strom unterbrochen → der des Stroms (1 s, 2 s, 5 s, danach alle 10 s); Strom verbunden, Nachholen gescheitert (R00012) → der des nächsten Nachhol-Versuchs; **Netz des Tabs aus, der Strom hat noch nicht gemeldet** → „⚠ Verbindung verloren“ ohne Countdown.
- **Kein Sprung**: die Oberkante des Verlaufs (`.lc-log`) liegt vor, während und nach der Trennung an derselben Stelle (± 1 px).
- **Ansage**: die Ansage-Region (R00012 F0049) ist die **einzige** Live-Region der Seite. Sie sagt „Verbindung verloren“ einmal je Trennung (beim Wechsel nach getrennt, nicht je Versuch und nicht je Countdown-Schritt) und den Text der Meldung ohne ✓ („Wieder verbunden · 3 nachgeladen“, „Wieder verbunden“, „Wieder verbunden · neueste geladen“), sobald sie erscheint — sofort, außerhalb der Bündelung, wie ein Fehlschlag. Gesammelte fremde Nachrichten bleiben gesammelt, ihr Takt wird nicht zurückgesetzt. Solange das Nachholen scheitert, gibt es keine Ansage „Wieder verbunden“. Die Leiste selbst ist keine Live-Region (ihr Countdown ändert sich je Sekunde).
- **Statuszeile ohne eigene Rolle**: die Statuszeile unten (Frist gesetzt, Channel angelegt) bleibt sichtbar, verliert `role="status"`; ihr Text geht zugleich sofort an die Ansage-Region.
- Ändert R00005 (Hinweis „Verbindung unterbrochen“ mit `role="alert"` → die Leiste; Kriterium „Zuletzt online“ prüft die Leiste) und R00012 (Leiste im Verlauf, schiebend, ohne Countdown → überlagernd mit Countdown; zwei `role="status"` → eine Live-Region).

### Verwerfen lässt sich rückgängig machen (F0085; UX-Review #22 F4)

- **Verwerfen** (R00012 F0046) entfernt eine fehlgeschlagene Nachricht nicht mehr sofort: an ihrer Stelle steht 5 s lang die Zeile „Verworfen“ mit dem Schalter „Rückgängig“, nicht höher als die Nachricht. Der Fokus liegt danach im Feld wie bisher.
- **Rückgängig** innerhalb der 5 s bringt die Nachricht unverändert zurück: Text, Grund, „Erneut senden“ und „Verwerfen“, derselbe Wiederholschlüssel; gesendet wird nichts. Der Fokus liegt danach auf „Erneut senden“ der Nachricht.
- Ab 5 000 ms nach „Verwerfen“ ist die Nachricht endgültig weg, wie bisher ohne Satz im Log.
- **Channelwechsel** innerhalb der 5 s behält die verworfene Nachricht; die Zeit läuft weiter. Wer innerhalb der 5 s zurückkommt, findet „Verworfen · Rückgängig“ an ihrer Stelle, wer später kommt, nichts. **Neuladen** oder Schließen verwirft sie endgültig — wie eine fehlgeschlagene (ADR-0037 Ergänzung Karte #36 „Verworfen auf Zeit“).
- Eine verworfene Nachricht ist nicht in der Warteschlange, hält sie nicht an und wird nie bestätigt; ein fremdes Ereignis mit gleichem Text trifft sie nicht.

### Hinweiszeile hilft beim Bereinigen (F0086; UX-Review #22 F3, Code-Review #22 A1, Rechtschreib-Review #22 Sollte)

- **Schalter „Steuerzeichen entfernen“**: zeigt die Hinweiszeile „⚠ Steuerzeichen U+001B an Stelle 5 — bitte entfernen.“ (R00012), steht daneben der Schalter „Steuerzeichen entfernen“. Ein Klick nimmt alle verbotenen Zeichen der Textregel aus dem Feld (C0 außer Tab und Zeilenumbruch, DEL, C1, ein Wagenrücklauf, der nicht unmittelbar vor einem Zeilenumbruch steht) — nur diese Zeichen, nicht ganze ANSI-Folgen. Danach werden Hinweis und Entwurf neu gesetzt, der Fokus liegt im Feld, die Schreibmarke am Ende.
- **Nichts wird automatisch umgeformt**: ohne Klick bleibt der Text, wie er eingefügt wurde; `Enter` sendet weiter nicht, solange ein Steuerzeichen darin steht (R00012).
- **Leerraum**: ein Text nur aus Leerraum (Unicode `White_Space`) zeigt keinen Hinweis und setzt kein `aria-invalid`, gleich wie viele Bytes er hat. Heute rechnet die Zeile auch dann „noch 16.384 − Bytes“ und zeigt ab 16.385 Bytes eine negative Zahl.
- Rechenbeispiel: 17.000 Leerzeichen → keine Zeile (heute „noch -616 Zeichen“); 17.000 Leerzeichen und „a“ → „⚠ Zu lang: 17.001 von höchstens 16.384 Byte — bitte teilen.“; 15.000 Leerzeichen und „a“ → „noch 1.383 Zeichen“.

### Wiederholschlüssel halten unter Last (F0087; Observability-Review #22 S1, S2, Security-Review #22 Optional)

- **Höchstzahl je Kennung zusätzlich zur Gesamtzahl**: der Speicher (R00012 F0046) hält je Kennung höchstens **640** und insgesamt höchstens **36.000** Einträge, je höchstens 10 Minuten alt (ADR-0034 Ergänzung Karte #36). Beim Merken fallen zuerst abgelaufene Einträge (sie zählen nie als verdrängt), dann bei 640 Einträgen der Kennung ihr ältester, dann bei 36.000 Einträgen der älteste insgesamt. Eine Kennung verdrängt so nie die Schlüssel einer anderen, solange die Gesamtgrenze nicht erreicht ist.
  - Rechenbeispiel (Herleitung aus ADR-0036 und dem Lastprofil, gerechnet): je Kennung höchstens Vorrat 30 + 600 s × 1/s = 630 Einträge in 10 min → 640 lässt Reserve; im Lastprofil insgesamt 5/s × 600 s = 3.000 dauerhaft, 30/s × 600 s = 18.000 in der Spitze → 36.000 ist das Doppelte; um fremde Schlüssel zu verdrängen, braucht es ⌈36.000 ÷ 630⌉ = 58 Kennungen, die 10 min lang ihr Schreibkonto ausschöpfen, auf mindestens 8 Herkünften.
  - Kein Durchlauf über alle Einträge je Sendung: Nachschlagen und Verdrängen sind amortisiert konstant (Index), weil beides unter der einen Schreibsperre läuft (ADR-0035).
- **Verdrängung sichtbar**: fällt ein Eintrag vor Ablauf seiner 10 Minuten heraus, steht im Betriebsprotokoll die WARNING-Episode `senden` „Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen“ je **verdrängter** Kennung (Mechanik R00014 F0055); die Beginn-Zeile trägt `kennung`, `grenze` (`kennung` oder `gesamt`), `hoechstens` (die gegriffene Grenze) und `alter_ms`, die Ende-Zeile die Zählung je Grenze; nie Schlüssel, nie Text (ADR-0042 Ergänzung Karte #36). `/api/gesundheit` zählt jede Verdrängung im flachen Feld `schluessel_verdraengt` (ADR-0043 Ergänzung Karte #36). Abgelaufene Einträge zählen nie.
- **Zwei Gründe für die 422 `schluessel`**: im Protokoll auf Debug trägt die Abweisung eines Kopfs `Idempotency-Key` mit falschem Format `grund=schluessel_ungueltig`, die eines bekannten Schlüssels mit anderem Text oder Channel `grund=schluessel_anders` (Mechanik `Abweisung` aus R00018 F0059, B0303; Namen nach ADR-0042 Ergänzung Karte #36 Punkt 2). Die Antwort bleibt 422 `regel:"schluessel"` mit ihren Texten.
- **Testknopf** `LOCALCHATA_TESTGRENZE_SCHLUESSEL=<n>` (nur Debug-Build): setzt die Gesamtgrenze auf `n` (ganze Zahl 1 bis 36.000; die Grenze je Kennung bleibt); jeder andere Wert, auch der leere, ergibt eine stderr-Zeile mit Variable und erlaubten Werten und Exit 2, gelesen vor der Sperre; im Release-Build wird die Variable nicht gelesen (ADR-0042 Ergänzung Karte #36 Punkt 3).
- Ändert R00012 F0046: „höchstens die 4 096 jüngsten Einträge“ wird „höchstens 640 je Kennung und 36.000 insgesamt“.

### Wiederbeitritt beim Senden auf dem Weg des Stroms (F0088; Code-Review #22 A2)

- Eine 401 auf das Senden und eine 401 im Strom treten über **denselben** Weg wieder bei (heute zwei: `web/start.js` `stillWiederBeitreten` für den Strom, `nachSendeAblehnungBeitreten` für das Senden): hat ein anderer Tab schon ein neues Token gemerkt, gilt es (kein `POST /api/beitreten`); sonst tritt der Tab still mit dem gemerkten Namen bei, je Browser nur ein Tab zugleich (Web Lock). Danach bekommen Strom und Senden dasselbe Token, und der eine neue Versuch des Sendens (R00012) trägt es.
- Melden Strom und Senden dieselbe Ablehnung, gibt es **einen** Lauf; der zweite wartet auf das Ergebnis des ersten.
- **Scheitert der stille Beitritt** (Name inzwischen vergeben, ungültig, Fehler, Server weg), verhält sich der Tab gleich, welcher Weg es auslöste (ADR-0012 Ergänzung Karte #36):
  - die Warteschlange hält sofort an — kein weiterer Sendeversuch mit dem toten Token;
  - jede Nachricht „wird gesendet …“ wird fehlgeschlagen mit dem Grund „Nicht gesendet — die Sitzung war beendet.“ (Wortlaut Vorschlag des architekt; ux- und rechtschreib-reviewer prüfen); fehlgeschlagene und verworfene bleiben, wie sie sind (verworfene mit ihrer Uhr);
  - der Strom endet, der Beitreten-Bildschirm zeigt den Zustand — wie heute bei einer 401 im Strom;
  - die Nachrichten überstehen den Bildschirm: nach dem nächsten Beitritt — gleich unter welchem Namen — stehen sie in ihren Channels unter dem Verlauf mit „Erneut senden“ und „Verwerfen“; gesendet wird erst auf „Erneut senden“, mit dem gespeicherten Schlüssel; nichts wird automatisch gesendet;
  - der Entwurf bleibt und liegt nach dem Beitritt wieder im Feld.
- Neuladen auf dem Beitreten-Bildschirm: ungesendete Nachrichten sind weg (R00012), der Entwurf bleibt.

## Nicht-funktionale Anforderungen

- **Zeiten** — die Grenzen stehen nur hier; Tests übernehmen sie wörtlich und sind nie schärfer. Countdown und 5 s „Rückgängig“ prüfen Node-Tests exakt mit gestellter Zeit; die Tabelle nennt die Zustellung.

  | # | Was | Art | Startpunkt der Uhr / Messweg | erwartet | Grenze | Zweck |
  |---|---|---|---|---|---|---|
  | Z14 | Wiederverbinden angesagt | Gate (E2E) | Netz von B wieder an → Statusregion enthält „Wieder verbunden“ | Z6 (≈ 50 ms + ein Abruf) + ein Frame | < 2 s | wer mit Bildschirmleser liest, erfährt so schnell wie die Sehenden, dass es weitergeht |
  | Z15 | Rückgängig verfällt | Gate (E2E) — Zustellung; Regel exakt im Node-Test | Klick „Verwerfen“ | Zeile weg nach 5 s + ein Frame | 4 s nach dem Klick noch da, spätestens 7 s danach weg | ein verklicktes Verwerfen lässt sich zurücknehmen, ohne dass die Zeile lange stehen bleibt |

  Reserve: Z14 > 10 × über dem erwarteten Wert (Grenze aus der Probe des ux-reviewer); Z15 40 % über der Regel (5 s → 7 s), Untergrenze 1 s unter der Regel.
- **Sicherheit**: die Höchstzahl je Kennung verhindert, dass eine Kennung oder eine Adresse die Schlüssel anderer verdrängt; Ein Schlüssel erscheint weiter nie in Log, Antwort oder Ereignis — auch nicht in der neuen Episode. Der Speicher bleibt auch unter Flut begrenzt: höchstens 36.000 Einträge, nach Hypothese ≈ 360 B je Eintrag, ≤ 600 B mit Index, also ≤ 22 MB (ADR-0034 Regel 4, gerechnet, nicht gemessen). Der Testknopf wirkt nie im Release-Build. Vor dem Bau schreibt der `security-reviewer` die Angriffsmatrix der zwei Grenzen (manuelle Vorbereitung); ihre Zeilen werden Akzeptanzkriterien.
- **Benutzerfreundlichkeit**: eine Warnung statt zwei, ohne Sprung; „Rückgängig“ und „Steuerzeichen entfernen“ sind Schalter in der Tab-Reihenfolge; bei 320 px kein Querscrollen mit Warnung, „Verworfen · Rückgängig“ und dem Schalter in der Hinweiszeile.
- **Nachvollziehbarkeit**: die Verdrängung eines Schlüssels steht als Episode im Protokoll und als Zähler im Health-Check; die zwei Gründe der 422 `schluessel` sind unterscheidbar.
- **Testbarkeit**: Countdown-Text, verworfen auf Zeit, Steuerzeichen entfernen, Hinweis und Speicher als reine Funktionen mit gestellter Zeit; Verdrängung über das echte Programm mit dem Testknopf.
- **Messwerte ohne Gate** (ADR-0034, für den Bericht des Entwicklers, kein Akzeptanzkriterium): (a) Release, 36.000 Einträge gefüllt, 10.000 × Prüfen und Merken — Median je Paar, erwartet wenige µs, Richtgrenze 100 µs; (b) Speicher des gefüllten Speichers — bestätigt oder korrigiert die Hypothese ≤ 22 MB.

## Akzeptanzkriterien

Gemeinsam für alle Kriterien wie in R00012: Start, `S(<name>)`, `alpha`, Browser A (`shoff`) und B (`anna`), „Strom getrennt“ (`context.setOffline(true)`) und „Strom wieder“. `heute.log` ist `logs/<heute>.log`.

### F0084 Eine Verbindungswarnung mit Countdown, angesagt

- [ ] Countdown-Text (Node-Test, rein, gestellte Zeit): Rest bis `naechsterVersuchUm` 5 000 ms → „⚠ Verbindung verloren · neuer Versuch in 5 s“; 4 001 ms → „in 5 s“; 4 000 ms → „in 4 s“; 0 ms und −300 ms → „⚠ Verbindung verloren · verbinde …“; 25 000 ms (verstellte Uhr) → „in 10 s“; kein Zeitpunkt → „⚠ Verbindung verloren“ ohne „neuer Versuch“ und ohne „verbinde“. Die Mutanten „abgerundet“, „ohne Begrenzung“ und „ohne Zeitpunkt wie verbinde“ werden je rot.
- [ ] Zeitpunkt aus dem Strom (Node-Test, rein, gestellte Uhr und gestellter Schlaf; erster Test aus ADR-0015 Ergänzung Karte #36, wörtlich): „`halten` mit `holen`, das immer scheitert, `jetzt` bei 0 → `unterbrochen` mit `naechsterVersuchUm` 1.000, dann 2.000 nach dem zweiten Versuch (relativ zu dessen Ende), 5.000, 10.000, 10.000; nach einem gelesenen Strom wieder 1.000 — jeweils gleich der Zeit, die `schlafen` bekommt. Verteiler: Zustand `unterbrochen` mit Zeitpunkt 7.000, danach verbindet ein neuer Tab → er bekommt denselben Zeitpunkt. Mutante „Wartezeit statt Zeitpunkt weitergereicht“ → rot.“
- [ ] Geltender Zeitpunkt (Node-Test, rein): Strom `unterbrochen` → der des Stroms; Strom verbunden, Nachholen gescheitert → der des nächsten Nachhol-Versuchs; Netz des Tabs aus ohne `unterbrochen` → kein Zeitpunkt. Die Mutante „Quelle immer der Strom“ wird rot.
- [ ] Eine Warnung ohne Sprung (E2E; Abnahme-Probe aus dem UX-Review #22, F1, wörtlich übernommen): B hat `# allgemein` gewählt; Strom getrennt, dann wieder → „genau ein Element, Oberkante .lc-log unverändert vor/während/nach der Trennung“ — genau ein sichtbares Element mit „Verbindung verloren“ oder „Verbindung unterbrochen“ auf der Seite, die Oberkante von `.lc-log` ändert sich um höchstens 1 px. Vor der Härtung rot (zwei Elemente, 37 px).
- [ ] Ohne offenen Channel (E2E): B hat keinen Channel gewählt; Strom getrennt → genau ein sichtbares Element mit „Verbindung verloren“; ein Channelwechsel während der Trennung ändert die Zahl nicht. Die Mutante „Leiste je Channel neu angelegt“ wird rot.
- [ ] Eine Live-Region (E2E; Durchsetzung aus ADR-0038 Ergänzung Karte #36): vor, während und nach einer Trennung gilt `document.querySelectorAll('[aria-live], [role="status"], [role="alert"]').length === 1`; A setzt eine Frist → die Statuszeile zeigt den Text, und die Ansage-Region enthält ihn. Die Mutante „zweite Region“ wird rot.
- [ ] Countdown im Tab (E2E): B getrennt → die Warnung nennt „neuer Versuch in“ mit einer Zahl von 1 bis 10 und zählt herunter (zwei Lesungen im Abstand von 1 s unterscheiden sich).
- [ ] Ansage (E2E, **Z14**; Abnahme-Probe aus dem UX-Review #22, F2, wörtlich übernommen: „Statusregion enthält „Wieder verbunden“ binnen 2 s“): B getrennt → die Statusregion enthält „Verbindung verloren“; `beta` sendet 3; Strom wieder → binnen **Z14** enthält sie „Wieder verbunden · 3 nachgeladen“. Vor der Härtung rot.
- [ ] Ansage unabhängig von der Bündelung (Node-Test, rein, gestellte Zeit): t = 0 fremd → „Neue Nachricht von …“; t = 2 s fremd (gesammelt); t = 3 s wieder verbunden mit 0 → sofort „Wieder verbunden“; t = 10 s → „1 neue Nachricht in # allgemein“. Zwei Meldungen `unterbrochen` hintereinander (zweiter gescheiterter Versuch) → genau eine Ansage „Verbindung verloren“. Die Mutanten „durch die Bündelung“ und „je Versuch angesagt“ werden je rot.
- [ ] Gescheitertes Nachholen (E2E, wie R00012 „Gescheitertes Nachholen“): solange das Nachholen scheitert, enthält die Statusregion kein „Wieder verbunden“.
- [ ] R00005 „Zuletzt online“ (E2E): das Programm wird beendet → A zeigt die Leiste „⚠ Verbindung verloren“ statt des Hinweises „Verbindung unterbrochen“, `#lc-verbindung` gibt es nicht mehr; die übrigen Zusagen des Kriteriums (gefüllter Punkt in `lc-offline`, „zuletzt online“) bleiben.
- [ ] Schmal (E2E): bei 320 px mit sichtbarer Warnung `scrollWidth <= innerWidth`.
- [ ] Konsole bei A und B: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zu Anfragen, die bei getrenntem Netz scheitern.

### F0085 Verwerfen lässt sich rückgängig machen

- [ ] Verworfen auf Zeit (Node-Test, rein, gestellte Zeit): fehlgeschlagene `x` mit Schlüssel `k`; „Verwerfen“ bei t = 0 → verworfen; „Rückgängig“ bei 4 999 ms → fehlgeschlagen mit Grund und Schlüssel `k`, kein Auftrag; erneut „Verwerfen“ bei t = 10 s → bei 14 999 ms verworfen, bei 15 000 ms entfernt. Die Mutanten „4 s“, „6 s“ und „Rückgängig sendet“ werden je rot.
- [ ] Rückgängig (E2E; Abnahme-Probe aus dem UX-Review #22, F4: „5 s „Verworfen · Rückgängig““): wie R00012 „Verwerfen“ mit `drei`; Klick „Verwerfen“ → an der Stelle steht „Verworfen“ mit „Rückgängig“, der Fokus liegt im Feld; Klick „Rückgängig“ → `drei` zeigt wieder ⚠ „Keine Antwort vom Server.“ mit „Erneut senden“ und „Verwerfen“, laut Netzprotokoll kein `POST`, der Fokus liegt auf „Erneut senden“.
- [ ] Channelwechsel (Node-Test, rein, gestellte Zeit; erster Test aus ADR-0037 Ergänzung Karte #36, wörtlich): „verworfen bei 0, Channelwechsel bei 1.000, Rückkehr bei 3.000 → Zeile da, Rückgängig → fehlgeschlagen mit demselben Schlüssel; Rückkehr bei 6.000 → entfernt. Mutante „Wechsel verwirft“ → rot.“ Dazu: ein fremdes Ereignis mit gleichem Text während „verworfen“ → bleibt verworfen; die Mutante „verworfen bestätigt“ wird rot.
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

- [ ] Speicher (Unit, rein, gestellte Uhr; kleine Grenzen über den Konstruktor, je Kennung 3, gesamt 5): A merkt `a1`; B merkt `b1`–`b4` → `b1` verdrängt (Kennung B, eigener Eintrag), `a1` noch bekannt (wiederholt); C merkt `c1`, `c2` → beim zweiten ist die Gesamtgrenze überschritten, der älteste insgesamt fällt, gemeldet mit Kennung und Alter; ein Eintrag, der älter als 600 000 ms ist, fällt ohne Meldung. Jede Meldung trägt die Grenze (`kennung` bei `b1`, `gesamt` bei der Gesamtgrenze). Die Mutanten „nur Gesamtgrenze“, „abgelaufen als verdrängt“ und „Grenze vertauscht“ werden je rot.
- [ ] Grenzen halten den Bedarf (Unit; Durchsetzung aus ADR-0034 Ergänzung Karte #36): aus dem Schreibkonto je Kennung gerechnet Vorrat + Nachschub × 600 s = 30 + 600 = 630 ≤ `HOECHSTENS_JE_KENNUNG` (640); `HOECHSTENS_GESAMT` (36.000) ≥ 30 × 600 = 18.000 (Spitze des Lastprofils). Wer das Schreibkonto großzügiger macht, wird rot.
- [ ] Verdrängung sichtbar (Unit, Speicher-Senke): eine Verdrängung an der Gesamtgrenze → genau eine Beginn-Zeile `WARNING senden Wiederholschlüssel vor Ablauf verdrängt … Episode beginnt` (Text nach dem Katalog: „Wiederholschlüssel vor Ablauf verdrängt — eine Wiederholung kann doppelt ankommen“) mit `kennung=` der verdrängten Kennung, `grenze=gesamt`, `hoechstens=` und `alter_ms=`, kein Schlüssel und kein Text in Zeile oder Kontext (`grep`-Gegenprobe); `schluessel_verdraengt` im Health-Check 1; ein abgelaufener Eintrag → keine Zeile, Zähler 0. Die Mutante „keine Meldung“ wird rot.
- [ ] Health-Check (Integration): ein leerer Ordner liefert `"schluessel_verdraengt": 0` neben den übrigen Kennzahlen.
- [ ] Unter Last (Integration über das echte Programm; Abnahme-Probe aus dem Observability-Review #22, S1, wörtlich übernommen): „alfa sendet „Wichtig“ mit auftrag-1; 160 Kennungen senden 4.480 Nachrichten; alfa wiederholt → erwartet WARNING-Episode „Wiederholschlüssel vor Ablauf verdrängt“, Health-Zähler ≥ 1, bei geänderter Grenze 200 und nur ein Satz; Gegenprobe mit gestellter Uhr > 10 min → keine Zeile.“ Lesart: mit den echten Grenzen (640 je Kennung, 36.000 gesamt) antwortet die Wiederholung 200, die Tagesdatei enthält „Wichtig“ genau einmal, keine Episode; mit `LOCALCHATA_TESTGRENZE_SCHLUESSEL=4096` (Debug-Build) erscheinen die Episode und `schluessel_verdraengt` ≥ 1. Vor der Härtung rot (201, zwei Sätze).
- [ ] Fremde verdrängen nicht (Integration über das echte Programm; Abnahme-Probe aus dem Security-Review #22, Optional, wörtlich übernommen): „A merkt v1; 4.096 Sendungen anderer Kennungen; A wiederholt v1 → 200, eine Zeile; Kontrolle A selbst 4.097 neue → v1 vergessen.“ Die Kontrolle „A selbst 4.097 neue“ ist über das echte Programm nicht innerhalb der 10 min herstellbar (das Schreibkonto erlaubt je Kennung höchstens 630); sie läuft als „Speicher“ oben gegen die Grenze je Kennung (640).
- [ ] Testknopf (Integration, Debug-Build; erster Test aus ADR-0042 Ergänzung Karte #36, wörtlich): „`LOCALCHATA_TESTGRENZE_SCHLUESSEL=0` und `=x` → Exit 2, kein `logs/`; `=5` → die sechste Sendung verdrängt, Episode und Zähler 1.“ Dazu: leerer Wert → Exit 2; die stderr-Zeile nennt die Variable und die erlaubten Werte 1 bis 36.000.
- [ ] Zwei Gründe (Integration, Stufe Debug): Kopf `Idempotency-Key: a b` → eine Zeile mit `status=422` und `grund=schluessel_ungueltig`; `beta` sendet `Hallo` mit `k1`, dann `Hallo!` mit `k1` → eine Zeile mit `grund=schluessel_anders`; beide Antworten unverändert 422 `regel:"schluessel"`; kein `k1` im Protokoll. Die Mutante „ein Grund für beide“ wird rot.
- [ ] Angriffsmatrix (nach der manuellen Vorbereitung): jede Zeile der Angriffsmatrix des `security-reviewer` zu den zwei Grenzen des Wiederholspeichers ist grün; der planer übernimmt ihre Zeilen hierher, sobald sie geschrieben ist. Vor der Matrix startet F0087 nicht.

### F0088 Wiederbeitritt beim Senden auf dem Weg des Stroms

- [ ] Ein Lauf (Node-Test mit gestellter API; erster Test aus ADR-0012 Ergänzung Karte #36, wörtlich): „Strom und Senden melden dasselbe abgelehnte Token im selben Task → genau ein `POST /api/beitreten`. Mutante „zwei Läufe“ → rot.“
- [ ] Ein Weg (Node-Test mit gestellter API): 401 auf das Senden, ein anderer Tab hat schon ein neues Token geschrieben → kein `POST /api/beitreten`, der neue Versuch trägt dieses Token, Strom und Senden haben dasselbe; ohne neues Token → genau ein `POST /api/beitreten` mit dem gemerkten Namen und danach der Versuch mit dem neuen Token. Die Mutante „Senden ohne Geschwister-Token“ wird rot.
- [ ] 401 (E2E): R00012 „401“ bleibt grün.
- [ ] Scheitern im Tab (Node-Test mit gestellter API; erster Test aus ADR-0012 Ergänzung Karte #36, wörtlich): „stiller Beitritt scheitert (409 `vergeben`) nach einer 401 beim Senden, zwei vorläufige in der Warteschlange → kein weiterer `POST …/nachrichten`, beide fehlgeschlagen mit dem Grund aus Regel 4, der Zustand `vergeben` geht an `zeigen`; dasselbe, wenn der Strom die Ablehnung zuerst meldet. Mutante „Senden scheitert still wie heute“ → rot.“ Der Grund lautet „Nicht gesendet — die Sitzung war beendet.“
- [ ] Scheitern (E2E; erster Test aus ADR-0012 Ergänzung Karte #36, wörtlich): „401 auf das Senden, Beitritt scheitert → Beitreten-Bildschirm wie bei einer 401 im Strom; danach Beitritt unter anderem Namen → die Nachricht steht fehlgeschlagen mit „Erneut senden“; Klick → gesendet unter dem neuen Namen, genau ein Satz. Entwurf im Feld wie vorher.“ Vor dem Klick kein `POST …/nachrichten`.
- [ ] Neuladen auf dem Beitreten-Bildschirm (E2E): wie „Scheitern“, aber vor dem Beitritt neu geladen → nach dem Beitritt steht keine der ungesendeten Nachrichten da, der Entwurf liegt im Feld. Die Mutante „Zustellung mit der Hauptansicht verworfen“ wird im Kriterium „Scheitern“ rot.
- [ ] Konsole bei A: keine unbehandelte Ausnahme, kein CSP-Verstoß; Konsolenfehler höchstens „Failed to load resource“ zu den gestellten 401 und 409.

## Betroffene Verzeichnisstruktur

- Oberfläche: `web/verbindungsleiste.js`, `web/hauptansicht.js`, `web/lc-seite.css`, `web/ansage.js`, `web/live.js`, `web/verteiler.js`, `web/strom.js`, `web/sekundentakt.js`, `web/start.js` (F0084); `web/start.js`, `web/hauptansicht.js`, `web/zustellung.js` (F0088, die Zustellung gehört dem Tab); `web/zustellung.js`, `web/verlauf.js`, `web/senden.js` (F0085); `web/textregel.js`, `web/hinweis.js`, `web/eingabe.js` (F0086).
- Server: `src/gespraeche/wiederholung.rs`, `src/gespraeche/` (Meldung an das Protokoll), `src/http/senden.rs`, `src/http/gesundheit.rs`, Lesen des Testknopfs beim Start (F0087).
- Tests: Unit im Modul; Integration `tests/senden_wiederholung.rs` (auch Testknopf); Oberfläche rein `e2e/einheit/`; Abläufe `e2e/tests/r00025-*.spec.js`; R00005-E2E zieht mit (F0084).

## Technische Überlegungen

### Ablauf

1. **Trennung**: Strom unterbrochen → Halter meldet `unterbrochen` mit `naechsterVersuchUm` → Verteiler (merkt ihn, reicht ihn neuen Tabs nach) → Tab wählt den geltenden Zeitpunkt → eine Leiste mit Countdown im Sekundentakt → Ansage „Verbindung verloren“ (einmal je Trennung); wieder verbunden → Nachholen (R00012) → Meldung → Ansage. Statuszeile → Ansage-Region.
2. **Verwerfen**: fehlgeschlagen → „Verwerfen“ → verworfen{seit} → „Rückgängig“ → fehlgeschlagen | 5 s → entfernt; ein Zeitgeber je Tab für die nächste fällige verworfene, gezeichnet nur im offenen Channel.
3. **Bereinigen**: Textregel → Hinweis `zeichen` → Schalter → Klick → Text ohne verbotene Zeichen → Feld, Entwurf, Hinweis.
4. **Senden mit Schlüssel** (R00012) → Speicher (Index, Grenzen 640 je Kennung und 36.000 gesamt): neu | wiederholt | anders, dazu `verdraengt{kennung, grenze, alter_ms}` → Episode `senden` und Zähler `schluessel_verdraengt`; 422 → `Abweisung` mit `schluessel_ungueltig` | `schluessel_anders`.
5. **401** (Strom oder Senden) → `wiederBeitreten` (ein Lauf je abgelehntem Token) → Token für beide | Scheitern → Warteschlange hält, vorläufige fehlgeschlagen → Beitreten-Bildschirm → Beitritt → Nachrichten mit „Erneut senden“ zurück.

### Grobentwurf (Vorplanung)

Die Bubbles aus der WBS als Datenfluss — **Vorplanung**: welche es am Ende werden, entscheidet der Entwickler beim Bauen.

- F0084: B0396 Warnung mit Countdown formen (rein) · B0397 Nächster Versuch des Stroms im Tab (unklar) · B0398 Eine Warnung ohne Layoutsprung (unklar) · B0399 Trennen und Wiederverbinden ansagen · B0411 Statuszeile ohne eigene Rolle, eine Live-Region
- F0085: B0400 Verworfen auf Zeit in der Zustellung (rein) · B0401 Platzhalter „Verworfen · Rückgängig“ · B0402 Rückgängig verdrahten
- F0086: B0403 Steuerzeichen entfernen (rein) · B0404 Schalter „Steuerzeichen entfernen“ · B0405 Leerraum ohne Hinweis (rein)
- F0087: B0406 Speicher je Kennung und gesamt (rein, unklar) · B0407 Verdrängung sichtbar · B0408 Verdrängung über das echte Programm geprobt (unklar) · B0409 Zwei Gründe für die 422 `schluessel`
- F0088: B0410 Ein Weg zum Wiederbeitritt (unklar) · B0412 Zustellung gehört dem Tab, übersteht den Beitreten-Bildschirm (unklar)

**Elemente** (nur öffentliche Signaturen, Vorschlag): `leisteInhalt(naechsterVersuchUm | null, jetztMs)`; Zustandsnachricht `{art: 'zustand', zustand, beitreten, token, naechsterVersuchUm}`; `ansage(zustand, {art: 'verbindung' | 'statuszeile', text}, jetzt)`; `zustellen(zustand, {art: 'rueckgaengig' | 'verfallen', …})`; `steuerzeichenEntfernen(text) → {text, entfernt}`; `wiederBeitreten(abgelehntesToken)`; `Wiederholung` mit beiden Grenzen im Konstruktor, Meldung `Verdraengt { kennung, grenze, alter_ms }`, Konstanten `HOECHSTENS_JE_KENNUNG = 640`, `HOECHSTENS_GESAMT = 36.000`, `HOECHSTENS_ALTER_MS = 600.000`.

Wireframe: Artboard `D0003.dc.html` fehlt. Gestaltungsquelle: Designsystem `components/VerlaufsMarken/README.md` („Verbindung“: Leiste mit Countdown), `components/Nachricht/README.md` (fehlgeschlagen, Schalter), `components/Eingabe/README.md` — Vorgabe, nicht Akzeptanzkriterium.

### Änderungen an bestehenden Klassen

- `web/hauptansicht.js` `verbindungZeigen` und `wiederverbindenEinrichten` (eine Warnung, `#lc-verbindung` entfällt, Statuszeile ohne Rolle), `web/live.js` `halten` und `web/verteiler.js` (Zeitpunkt im Zustand), `web/start.js` (ein Weg zum Wiederbeitritt, `nachSendeAblehnungBeitreten` entfällt; die Zustellung gehört dem Tab), `web/zustellung.js` (verworfen auf Zeit), `web/hinweis.js` (leer ohne Hinweis), `src/gespraeche/wiederholung.rs` (zwei Grenzen, Index, Verdrängung), `src/http/gesundheit.rs` (Feld `schluessel_verdraengt`).

## Tests

Nach Skill `test-pyramide` und ADR-0009; Tests sind nie schärfer als diese Anforderung und übernehmen Z14 und Z15 wörtlich. **Unit:** Speicher mit zwei Grenzen, Grenzen halten den Bedarf, Verdrängung mit Speicher-Senke. **Integration:** unter Last und fremde verdrängen nicht über das echte Programm (Gelerntes der Factory: Probe gegen den echten Einstieg), Testknopf, Health-Check, zwei Gründe auf Stufe Debug, Angriffsmatrix. **Oberfläche rein:** Countdown-Text, Zeitpunkt aus dem Strom, geltender Zeitpunkt, Ansage, verworfen auf Zeit mit Channelwechsel, Steuerzeichen entfernen, Leerraum, ein Lauf und ein Weg zum Wiederbeitritt, Scheitern im Tab. **E2E:** eine Warnung ohne Sprung, ohne offenen Channel, eine Live-Region, Countdown, Ansage (Z14), Rückgängig und Verfallen (Z15), Schalter „Steuerzeichen entfernen“, Scheitern und Neuladen auf dem Beitreten-Bildschirm, schmal; R00005 „Zuletzt online“ zieht mit. **Messwerte ohne Gate:** Speicher bei 36.000 Einträgen (Zeit je Paar, Speicher).

## Abhängigkeiten

- Abhängig von: Nacharbeit #22 an R00012 (B0390–B0395, rot) — F0084 braucht F0047 (Leiste und Fehlerfall des Nachholens, B0393) und F0049 (Statusregion); F0085 und F0088 brauchen F0046; F0086 braucht F0048; F0087 braucht F0046, F0055 (Episoden, R00014, grün) und F0059 (`Abweisung`, R00018, Karte #26, rot).
- Innerhalb: F0085, F0086, F0088 parallel; F0084 nach B0393 (Zeitpunkt des Nachhol-Versuchs, Leiste); F0087 nach F0059; F0085 und F0088 berühren beide die Zustellung (`web/zustellung.js`, bei F0088 der Umzug in den Tab) — nacheinander, F0088 zuerst empfohlen, weil B0412 den Ort der Zustellung festlegt, an dem B0402 seinen Zeitgeber hängt.
- Vor dem Bau: Angriffsmatrix des security-reviewer für F0087 (manuelle Vorbereitung). Die Antworten des architekt auf die Punkte 3–8 liegen vor (ADR-Ergänzungen Karte #36, 2026-10-29).
- Blockiert: nichts.

## Offene Fragen

An den architekt: keine mehr. Entschieden am 2026-10-29 (WBS, Offene Fragen #22):

- Punkt 3 — Quelle des Countdowns: absoluter Zeitpunkt `naechsterVersuchUm` im Zustand `unterbrochen`, `Date.now()` auf beiden Seiten (ADR-0015 Ergänzung Karte #36, Option A); daraus der Fall „Netz des Tabs aus, Strom hat noch nicht gemeldet → Warnung ohne Countdown“.
- Punkt 4 — eine überlagernde Leiste oben in der Mitte, mit und ohne Channel; eine Live-Region; Statuszeile ohne Rolle (ADR-0038 Ergänzung Karte #36, Optionen A und E).
- Punkt 5 — verworfen in der Zustellung; Channelwechsel behält, Neuladen verwirft (ADR-0037 Ergänzung Karte #36, Option A = Empfehlung planer).
- Punkt 6 — 640 je Kennung, 36.000 gesamt, Index statt Durchlauf (ADR-0034 Ergänzung Karte #36, Optionen A und D).
- Punkt 7 — Episode `senden`, `schluessel_ungueltig`/`schluessel_anders` (statt des Vorschlags `schluessel_format`), Testknopf `LOCALCHATA_TESTGRENZE_SCHLUESSEL`, Zähler `schluessel_verdraengt` (ADR-0042 und ADR-0043, Ergänzungen Karte #36).
- Punkt 8 — ein Weg; beim Scheitern Beitreten-Bildschirm, die Zustellung übersteht ihn, nichts automatisch gesendet, Entwurf bleibt (ADR-0012 Ergänzung Karte #36, Option A).

An den Betreiber: keine.

Entschieden vom planer für R00025 (Rückfragen mit Empfehlung beantwortet):

- **„Steuerzeichen entfernen“ nimmt nur die verbotenen Zeichen**, nicht ganze ANSI-Folgen — vorhersagbar und gleich der Regel, die die Hinweiszeile nennt; das Entfernen ganzer Folgen wäre Umformen über die Regel hinaus.
- **Leerraum zeigt keinen Hinweis** statt „noch 0 Zeichen“ oder „Zu lang“ — ein Text nur aus Leerraum wird ohnehin nicht gesendet (R00011), die Zeile hätte nichts zu sagen.
- **Ansage ohne ✓**, mit dem Text der Leiste — das Zeichen hat für den Bildschirmleser keinen Inhalt.
- **Fokus nach „Rückgängig“ auf „Erneut senden“** — wer zurücknimmt, will meist erneut senden; nach „Verwerfen“ bleibt der Fokus im Feld wie in R00012.
- **Wortlaut des Countdowns** „neuer Versuch in 5 s“ — Vorschlag; ux- und rechtschreib-reviewer prüfen.

## Manuelle Vorbereitungstätigkeiten

- ~~architekt beantwortet die Punkte 3–8~~ — erledigt 2026-10-29 (ADR-Ergänzungen Karte #36); R00025 nachgezogen.
- **Angriffsmatrix vor F0087** (schreibt der `security-reviewer`, über den Director; Stand 2026-10-29: nicht geschrieben): Matrix der zwei Grenzen des Wiederholspeichers — mindestens Flut über viele Kennungen einer Herkunft, über viele Herkünfte (≥ 58 Kennungen auf ≥ 8 Herkünften erreichen die Gesamtgrenze), eine Kennung gegen ihre eigene Grenze, Speicherbedarf bei 36.000 Einträgen, Schlüssel oder Text in Protokoll, Antwort und Ereignis, Testknopf im Release-Build. Empfohlener Ort wie bei Karte #35: eine Ergänzung „Angriffsmatrix Karte #36“ in ADR-0034. Danach übernimmt der planer ihre Zeilen als Akzeptanzkriterien (Kriterium „Angriffsmatrix“ unter F0087). Ohne Matrix startet F0087 nicht; F0084, F0085, F0086 und F0088 hängen nicht daran.

## Manuelle Nachbereitungstätigkeiten

- Designsystem-README „Verbindung verloren.“ ohne Punkt angleichen und `VerlaufsMarken` um „Verworfen · Rückgängig“ ergänzen (über den Director).
- `AI-Team/umgebung.md`: Testknopf `LOCALCHATA_TESTGRENZE_SCHLUESSEL` (nur Debug-Build, 1 bis 36.000, sonst Exit 2) nachtragen (Director).
- ADR-0037, Ergänzung Karte #22/#36, 7f: die in B0391 gebaute Mechanik nachtragen (architekt, nach dem Bau der Nacharbeit #22; hier nur zur Erinnerung).
- Messwerte (a) und (b) aus dem Bericht des Entwicklers in ADR-0034 Regel 4 bestätigen oder korrigieren (architekt).

## Umfang

Aus der WBS gezählt; Standard-Bubbles mit der Untergrenze 0,4h, unklare als Richtwert 2–4h (ungemessen):

- F0084 = 5 Bubbles: 3 Standard (0,4h), 2 unklar (B0397, B0398). Rest: 1,2h klar + 4–8h unklar.
- F0085 = 3 Bubbles: 3 Standard. Rest: 1,2h klar.
- F0086 = 3 Bubbles: 3 Standard. Rest: 1,2h klar.
- F0087 = 4 Bubbles: 2 Standard, 2 unklar (B0406, B0408). Rest: 0,8h klar + 4–8h unklar.
- F0088 = 2 Bubbles: 2 unklar (B0410, B0412). Rest: 4–8h unklar.
- R00025 gesamt = 17 Bubbles (vorher 15; +B0411, +B0412 nach den ADR-Ergänzungen Karte #36): 11 Standard, 6 unklar. Rest: 4,4h klar + 12–24h unklar (Richtwerte, ungemessen).
- Nicht enthalten: Nacharbeit #22 (6 Bubbles, R00012); die optionalen Punkte der Reviews (WBS, Offene Fragen #22).

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser: Unter dem Lastprofil schreiben bis zu 50 Agenten, in der Spitze 30 Nachrichten je Sekunde. Der Wiederholspeicher mit 4.096 Einträgen gesamt ist dann nach etwa 136 s voll und verdrängt den Schlüssel eines Agenten, dessen Antwort unterwegs verloren ging; seine Wiederholung legt eine zweite Nachricht an, und andere Agenten arbeiten sie doppelt ab. Wenn der Speicher je Kennung begrenzt ist und insgesamt für 10 Minuten des Lastprofils reicht, dann verdrängt keine Kennung die Schlüssel einer anderen, und „genau einmal“ gilt wieder für die ganze Zusage; wenn eine Verdrängung trotzdem geschieht, dann sieht die Betreiberin sie im Protokoll und im Health-Check, bevor jemand Dubletten meldet. Auf der Seite des Menschen: wenn bei einem Abriss genau eine Warnung mit Countdown steht und angesagt wird, dann weiß jeder — sehend oder mit Bildschirmleser —, dass die Instanz es selbst weiter versucht, und lädt nicht neu. Wenn „Verwerfen“ 5 s zurücknehmbar ist und Steuerzeichen mit einem Klick verschwinden, dann kostet ein Fehlgriff oder eine eingefügte Terminalausgabe keinen neu getippten Text. Der Hebel ist die Grenze je Kennung: sie macht die Zusage des Wiederholschlüssels unabhängig davon, wie viele andere schreiben.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht).
- Kein Artboard für D0003.
- Angriffsmatrix für F0087 steht aus (manuelle Vorbereitung).

## Notizen

### Verworfene Alternativen

- **Nur die Gesamtzahl erhöhen** (ohne Grenze je Kennung): verworfen — eine flutende Adresse verdrängt dann weiter die Schlüssel aller anderen, nur später.
- **Rückgängig ohne Zeitgrenze** (verworfene Nachrichten in einem Papierkorb): verworfen — die Zeile bliebe stehen, und eine fehlgeschlagene Nachricht ist kein Dokument; 5 s ist der Vorschlag des ux-reviewer.
- **Steuerzeichen beim Einfügen still entfernen**: verworfen — R00012 sagt „Umgeformt wird nichts“; ein Mensch soll sehen, dass sein Text Steuerzeichen enthielt.
- **Eine Anforderung je Slice**: verworfen — die fünf Slices sind das Arbeitspaket „Härtung #22“ und durchlaufen gemeinsam Entwicklertest und Abnahme.

### Review 2026-10-29 (Karte #36, planer im Auftrag des Directors, `/anforderung review` nach den ADR-Ergänzungen Karte #36)

- A Beschreibung: Wiederbeitritt um das Scheitern ergänzt; „Architektur zuerst“ durch die Liste der Entscheidungen ersetzt. Vision-Bezug und Nutzen tragen unverändert.
- B Funktionale Anforderungen: F0084 mit Ort und Gestalt der einen Leiste, Quelle des Countdowns, Fall „Netz aus ohne Countdown“, einer Live-Region und der Statuszeile ohne Rolle; F0085 mit Channelwechsel und Neuladen; F0087 mit 640/36.000 samt Rechenbeispiel, Katalogzeile, `schluessel_verdraengt`, `schluessel_ungueltig`/`schluessel_anders` und Testknopf; F0088 mit einem Lauf und dem Verhalten beim Scheitern.
- C Akzeptanzkriterien: neu „Zeitpunkt aus dem Strom“, „Geltender Zeitpunkt“, „Ohne offenen Channel“, „Eine Live-Region“, „Channelwechsel“ (F0085), „Grenzen halten den Bedarf“, „Health-Check“, „Testknopf“, „Angriffsmatrix“ (Platzhalter bis zur Matrix), „Ein Lauf“, „Scheitern im Tab“, „Scheitern“ (E2E), „Neuladen auf dem Beitreten-Bildschirm“, Konsole F0088; geschärft „Countdown-Text“, „Ansage unabhängig von der Bündelung“, „R00005 Zuletzt online“, „Speicher“, „Verdrängung sichtbar“, „Unter Last“, „Zwei Gründe“. Erste Tests der ADRs wörtlich. User Stories: US-8 mit dem Testknopf, neu US-10 (Channelwechsel) und US-11 (Scheitern).
- D/E Ablauf und Grobentwurf: Zeitpunkt im Zustand, `wiederBeitreten`, Konstanten und Meldung `Verdraengt`; neue Bubbles B0411, B0412.
- F Tests: Ebenen um die neuen Kriterien und die Messwerte ohne Gate ergänzt.
- G Offene Fragen, Abhängigkeiten, Wirkungskette: Punkte 3–8 entschieden; F0085 und F0088 nacheinander (Zustellung), F0088 zuerst empfohlen; Wirkungskette trägt unverändert (Hebel bleibt die Grenze je Kennung).
- H Gesamteindruck: Umfang 15 → 17 Bubbles; vor dem Bau fehlt nur die Angriffsmatrix (F0087).

### Bewusst außerhalb

Optionale Punkte der Reviews #22 (WBS, Offene Fragen #22, „nicht eingeplant“): `wiederholt` als Health-Zähler, „Verlaufsblock gelesen“ mit `kennung` und Richtung, Spur für Senden langsamer als die Zeitgrenze des Browsers, Gruppierung vorläufiger Nachrichten, Erfolgsansage nach „Erneut senden“, `aria-describedby` der Schalter, Entwurf in der Seitenleiste, Feldwerte im 422-Text in „…“, doppeltes „Nicht gesendet“ im Text der 502, „Verlassen“ → „Weggehen“ in Testnamen; `cargo audit` im Gate (Director).
