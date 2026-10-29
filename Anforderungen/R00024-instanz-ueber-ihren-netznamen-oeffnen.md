---
id: R00024
status: Neu
datum: 2026-10-29
---

# R00024: Instanz über ihren Netznamen öffnen

## Beschreibung

Ein Teilnehmer öffnet die Instanz über den Namen, den das eigene Netz ihr gibt, ohne dass der Betreiber etwas einstellt. Gemeint ist der Name, den der Router dem Rechner gibt (`http://nuc5.fritz.box:7430/`), oder ein Name unter `.local`, `.home.arpa` oder `.internal` bzw. ein einteiliger Name, der auf eine eigene Adresse zeigt. Jeder andere Name, vor allem jeder öffentliche, bleibt ohne `--erreichbar-als` abgewiesen und löst nicht einmal eine Namensauflösung aus. Die Abweisung ist für einen Browser eine deutsche Seite mit einem klickbaren Link auf die IP-Adresse, die sofort funktioniert; ein Agent bekommt Text mit derselben Adresse. Für bewusste Ausnahmen gibt es den Schalter `--alle-namen`: er nimmt jeden Namen an, warnt beim Start und lässt die Origin-Regeln aktiv.

Zahlt ein auf: [Vision](R00000-vision.md) — „Eine gemeinsame Instanz im LAN, die alle Teilnehmer ansprechen — auch über Rechnergrenzen“ und „Keine Anmeldung“ (der Schutz gegen fremde Websites bleibt im Grenzschutz, nicht in einer Anmeldung). Kein Widerspruch zur Vision: öffentliche Namen bleiben eine bewusste Entscheidung des Betreibers, die Instanz wird kein öffentlicher Internet-Dienst („Ausdrücklich nicht Ziel“).

Auslöser ist der Befund des Betreibers vom 2026-10-29 (Karte #35, Vorrang): „Wenn ich über DNS zugreife, bekomme ich die Meldung, die Instanz wäre unter dem Namen xy nicht freigegeben. Das ist aber etwas, das viele Browser nicht verstehen. Daher soll es möglich sein, den DNS-Namen zu verwenden, solange dieser auf die richtige IP aufgelöst wird.“ Die Richtung hat der Betreiber nach der Erklärung des Rebinding-Risikos entschieden: Namen des eigenen Netzes automatisch, öffentliche Namen nur mit `--erreichbar-als`, eine verständliche Seite statt einer nackten 403, ein bewusster Schalter `--alle-namen`.

Entschieden vom architekt: [ADR-0045](../Dokumentation/ADRs/0045-host-namen-des-eigenen-netzes-automatisch-oeffentliche-nur-ausdruecklich.md) (Regeln 1–9, Schnitt; ersetzt [ADR-0014](../Dokumentation/ADRs/0014-grenzschutz-host-origin-sicherheitsheader.md) Regel 1, die Regeln 2–6 gelten unverändert). Bausteine: [A00004 HTTP-Schnittstelle](../Dokumentation/Architektur/A00004-http-schnittstelle.md) (Namensauskunft, Grenzschutz) und [A00003 Start](../Dokumentation/Architektur/A00003-start.md) (lokale Adresse je Verbindung, `--alle-namen`), je Ergänzung Karte #35.

Slices (WBS `Dokumentation/Planung/localchata.md`, alle unter I0024 Instanz über ihren Netznamen öffnen, Dialog D0001 Start und Ablage): F0077 Über den Rückwärtsnamen öffnen, F0078 Name im lokalen Namensraum, F0079 Abweisung als verständliche Seite, F0080 Schalter `--alle-namen`.

Angeglichen am 2026-10-29 (planer, `/anforderung review`, Karte #35) an die Ergänzung „Angriffsmatrix Karte #35“ von ADR-0045 (Entscheidungen E1–E8, Abschnitt „Angriffsmatrix“) und an [ADR-0043](../Dokumentation/ADRs/0043-protokollstufe-einstellen-und-kennzahlen-im-health-check.md) Ergänzung Karte #26 (gültige Option übersteuert eine falsche Umgebung, E7).

Angeglichen am 2026-10-29 (planer, `/anforderung review`, Nacharbeit Karte #35) an die Ergänzung „Karte #35 Nacharbeit“ von ADR-0045 (Punkte 1–5): eine Frist je Anfrage und ein Weg je Name, Cache in vier Fächern, Rate neuer Auflösungen, Gnadenfrist mit Auffrischen, Auflösung auf eigenem Faden, Panik belegt, Wortlaut der Risikosätze. Die Abnahme-Proben aus den Reviews #35 (code-reviewer B1, security-reviewer W1, rechtschreib-reviewer) stehen wörtlich in den Akzeptanzkriterien „Nacharbeit Karte #35“.

## Geschäftlicher Nutzen

Im Heimnetz und im kleinen Büro ist der Name, den der Router vergibt, der normale Weg zu einem Rechner. Heute weist die Instanz genau diesen Namen ab, und zwar mit einem `text/plain`, das der Teilnehmer im Browser nicht einordnen kann und dessen Ausweg (`--erreichbar-als`) nur der Betreiber hat. Mit R00024 funktioniert `http://nuc5.fritz.box:7430/` ohne Startoption, ohne Neustart und auch nach einem DHCP-Wechsel. Wo ein Name trotzdem abgewiesen wird, zeigt die Seite den Link, der sofort funktioniert, und sagt dem Betreiber, was er tun kann. Der Schutz gegen DNS-Rebinding bleibt dabei bestehen: eine fremde Website, die ein Teilnehmer öffnet, bekommt weiterhin keinen Zugriff auf Channels, Verläufe und Fristen.

## Funktionale Anforderungen

### Über den Rückwärtsnamen öffnen (F0077; ADR-0045 Regeln 1–3, 5, 6, 8)

- Ein Host-Name außerhalb des lokalen Namensraums, den die Rückwärtsauflösung der lokalen Adresse der Verbindung liefert, ist erlaubt, ohne Groß-/Kleinschreibung und mit oder ohne abschließenden Punkt.
- Vor jeder Prüfung wird der Host-Anteil in die Normalform gebracht; was sie verletzt, ist abgewiesen, ohne dass etwas aufgelöst wird.
- IP-Literale, `localhost`, der Rechnername samt `<rechnername>.local` und die Namen aus `--erreichbar-als` bleiben ohne Auflösung erlaubt.
- Ein Host-Anteil, der kein IP-Literal ist und dessen letztes Label eine Zahl ist (`2130706433`, `0x7f000001`, `127.1`, `1.2.3.4.5`), verletzt die Normalform und ist ohne Auflösung abgewiesen (E2).
- Ein Name außerhalb des lokalen Namensraums (also jeder öffentliche Name) löst nie eine Vorwärtsauflösung aus und ist nur mit `--erreichbar-als` erlaubt. Die Rückwärtsfrage hängt an der lokalen Adresse, nicht am Namen: sie läuft für jeden Namen außerhalb des lokalen Namensraums, der an der festen Liste scheitert und die Normalform erfüllt, höchstens einmal je lokaler Adresse und Cache-Lebensdauer (E8). Ein Name im lokalen Namensraum wird nie rückwärts gefragt: je Name gibt es genau einen Weg, und eine Anfrage stellt höchstens eine neue Frage (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1).
- Auflösungen laufen begrenzt: höchstens 2 gleichzeitig und höchstens 10 neue je Sekunde (ein Eimer für die ganze Instanz, Vorrat 20). Eine Anfrage wartet auf alle Fragen zusammen höchstens 1 s ab ihrem Eingang (eine Frist je Anfrage); ein Cache-Treffer kostet keine Wartezeit. Eine Anfrage, die eine neue Auflösung bräuchte und keine freie Erlaubnis oder kein Zeichen aus dem Eimer bekommt, wartet nicht: ihr Name ist für diese Anfrage nicht erlaubt, sie startet keine Auflösung und hinterlässt keinen Cache-Eintrag; den Cache schreibt nur eine gestartete Auflösung (E3).
- Der Cache hält „erlaubt“ 5 min und „nicht erlaubt“ (auch Zeitgrenze, Fehler und Panik) 60 s, in vier Fächern: Vorwärts „erlaubt“ 128, Vorwärts „nicht erlaubt“ 96, Rückwärts mit Namen 16, Rückwärts ohne Namen 16 — zusammen 256 fertige Einträge. Ist ein Fach voll, fällt der älteste fertige Eintrag desselben Fachs; ein negativer Eintrag verdrängt nie einen positiven, ein Vorwärtseintrag nie einen Rückwärtseintrag.
- Gnadenfrist: ein positiver Eintrag (Vorwärts „erlaubt“, Rückwärts mit Namen) gilt nach seinem Ablauf noch 10 min weiter. Eine Anfrage in dieser Zeit ist sofort erlaubt und startet eine Auffrischung, wenn keine läuft und eine Erlaubnis frei ist (ohne Zeichen aus dem Eimer); das Ergebnis ersetzt den Eintrag. Ein Name, der nicht mehr auf die Instanz zeigt, bleibt so bis zu 15 min nach der letzten Bestätigung erlaubt (in Kauf genommen, ADR-0045).
- Eine Panik in der Auflösung zählt als „ohne Antwort“ (nicht erlaubt, 60 s) und steht als EXCEPTION mit Ort im Protokoll. Das Ende des Programms wartet nicht auf eine hängende Auflösung.
- Der `Origin`-Vergleich bleibt wörtlich gegen den gesendeten `Host`-Kopf (ADR-0014 Regel 2); die Normalform gilt nur für das Urteil über den Namen (E5).
- Das Betriebsprotokoll nennt je neuem Cache-Eintrag „erlaubt“ einmal den Namen und die lokale Adresse; eine zu langsame Auflösung steht als Warning-Episode darin. Auf Stufe debug steht vor jeder Frage an die Namensauskunft eine Zeile „Namensauskunft gefragt“ mit `art` (`vorwaerts`, `rueckwaerts`, `eigen`) und `schluessel` (E6).

### Name im lokalen Namensraum (F0078; ADR-0045 Regeln 4, 6, 8)

- Ein einteiliger Name oder ein Name unter `.local`, `.home.arpa`, `.internal` ist erlaubt, wenn seine Vorwärtsauflösung mindestens eine eigene Adresse liefert (Loopback zählt).
- Ein Name im lokalen Namensraum wird nur vorwärts geprüft; ein Rückwärtsname im lokalen Namensraum (etwa ein PTR auf `nas.home.arpa`) ist nur über diese Regel erlaubt, also wenn seine Vorwärtsauflösung eine eigene Adresse liefert (ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1).
- Suffixe werden nur an Label-Grenzen erkannt; es gibt kein abgeleitetes Suffix (nicht aus der Suchdomäne, nicht aus dem Rückwärtsnamen).
- Jede Adresse der Antwort wird kanonisch (`::ffff:127.0.0.1` → `127.0.0.1`); unspezifizierte Adressen (`0.0.0.0`, `::`), Multicast (`224.0.0.0/4`, `ff00::/8`) und `255.255.255.255` zählen nie als eigen, ohne dass gefragt wird (E1).

### Abweisung als verständliche Seite (F0079; ADR-0045 Regel 7)

- Ein abgewiesener Host bekommt weiter 403. Fragt ein Browser nach einem Dokument, ist der Körper eine deutsche HTML-Seite ohne Stil und Skript mit einem klickbaren Link auf `http://<lokale Adresse>:<port>/`.
- Alle anderen Clients bekommen Text mit dem Namen, der Adresse und dem Ausweg; es gibt keine Umleitung.
- Fehlt der `Host` oder ist er leer, bekommt ein Browser-Dokument die Seite ohne Namen und ohne Satz für den Betreiber (es gibt keinen Namen zum Freigeben); alle anderen bekommen den Text „nennt keinen Host“, ergänzt um die Adresse. Ein doppelter `Host` bleibt 403 ohne Seite (E4).

### Schalter `--alle-namen` (F0080; ADR-0045 Regel 9)

- `--alle-namen` oder `LOCALCHATA_ALLE_NAMEN=1` nimmt jeden Host in Normalform an, ohne die Namensauskunft zu fragen.
- Steht `--alle-namen` im Aufruf, ist der Schalter an, und `LOCALCHATA_ALLE_NAMEN` wird nicht gelesen — auch ein falscher Wert hält den Start nicht auf. Ohne den Schalter im Aufruf ist ein Wert außer `1`, `0` und leer Exit 2 (ADR-0043 Ergänzung Karte #26, Regeln 1, 2 und 4; E7).
- Origin- und `Sec-Fetch-Site`-Regel, CORS und Sicherheitsheader aus ADR-0014 bleiben aktiv.
- Der Zustand ist sichtbar: als WARNING beim Start, in `--hilfe` und im Health-Check.

## Nicht-funktionale Anforderungen

- **Sicherheit** (DNS-Rebinding, CWE-350; Flut, CWE-400): kein Name, den ein Angreifer im öffentlichen DNS besitzt, wird automatisch erlaubt, und keiner löst eine Auflösung aus. Eine Flut frei gewählter Namen verdrängt keinen erlaubten Namen aus dem Cache und wird höchstens mit 10 neuen Auflösungen je Sekunde ins Netz weitergereicht. Mit `--alle-namen` ist der Rebinding-Weg bewusst offen, CSRF bleibt durch ADR-0014 Regel 2 geschlossen. Die Angriffsmatrix steht in ADR-0045 (Abschnitt „Angriffsmatrix“, vom `security-reviewer` vor dem Bau geschrieben, Entscheidungen E1–E8 in der Ergänzung „Angriffsmatrix Karte #35“); sie ist Teil dieser Anforderung (Kriterium „Angriffsmatrix“).
- **Leistung**: der Laufzeit-Takt löst nie auf; eine Anfrage wartet höchstens 1 s auf alle Auflösungen zusammen (eine Frist je Anfrage). Zweck der Grenze: ein frei wählbarer `Host` darf die Instanz nicht blockieren, ein nicht vorhandener `.local`-Name kostet sonst 5 s (Probe in ADR-0045). Die Uhr startet beim Eingang der Anfrage im Grenzschutz. Erwartet sind im Heimnetz 4–141 ms (gemessen in der ADR-Probe); die Grenze liegt also weit über 130 % des erwarteten Werts. Geprüft wird die Regel exakt mit gestellter Uhr; im echten Programm gibt es kein Zeitkriterium.
- **Ende**: eine hängende Namensauflösung verzögert das Ende nicht. Zweck: die Frist für das Ende aus ADR-0010 (5 s); erwartet ohne offene Verbindung 0,002 s (gemessen im Code-Review #35), heute 4,0 s. Die Grenze im Test (< 200 ms im Prozess — B0421, < 1 s im echten Programm ab dem Signal — B0485) liegt weit über 130 % des erwarteten Werts. Das Prozessende im echten Programm hängt sonst am NSS-Modul: `exit()` der glibc wartet auf den Faden der Auflösung (ADR-0010 Ergänzung „Karte #35 Nacharbeit, Prozessende“).
- **Plattform**: Rückwärtsauflösung über `dns-lookup` unter Linux, macOS und Windows; „eigene Adresse“ über `UdpSocket::bind((ip, 0))`.
- **Zeit- und Konsolenkriterien** nach `AI-Team/planer.md`.

## Akzeptanzkriterien

Gemeinsam für alle Integrationskriterien: Start im Debug-Build mit `localchatA <ordner> --port 0 --host 127.0.0.1 --kein-browser` auf einem frischen Wegwerf-Ordner, `TZ=Europe/Berlin`. `<p>` ist der Port aus der Startzeile, `heute.log` die Datei `logs/<lokales Datum>.log`. „Testnamen“ heißt `LOCALCHATA_TESTNAMEN='127.0.0.1>nuc5.fritz.box;chat.local=127.0.0.1;drucker.local=192.0.2.1;boese.example=127.0.0.1;chat=127.0.0.1;nas.home.arpa=127.0.0.1;dienst.internal=127.0.0.1;chat.fritz.box=127.0.0.1'`. Angefragt wird immer `http://127.0.0.1:<p>/api/gesundheit` mit dem genannten `Host:`-Kopf. „Erlaubt“ heißt 200 mit `{"status":"bereit",…}`, „abgewiesen“ heißt 403.

### Sicherheit

- [ ] **Angriffsmatrix ADR-0045 vollständig grün**: jede Zeile des Abschnitts „Angriffsmatrix“ in [ADR-0045](../Dokumentation/ADRs/0045-host-namen-des-eigenen-netzes-automatisch-oeffentliche-nur-ausdruecklich.md) (Gruppen K, A–J; Ergänzung „Angriffsmatrix Karte #35“, „Durchsetzung, ergänzt“) außer den Zeilen der Linie L2r ist ein Test über die angegebene Linie — L1 im Prozess mit Fake-Namensauskunft und Testuhr, L2 über das echte Programm mit `LOCALCHATA_TESTNAMEN`, L2 (Release) im Release-Build — und grün; jede Zeile, die einen Wächter betrifft, wird mit dessen Mutante rot. „V nein“ bzw. „R nein“ in L2 heißt: auf Stufe debug keine Zeile „Namensauskunft gefragt“ mit `art=vorwaerts` bzw. `art=rueckwaerts` für diesen Schlüssel (E6). Die Zeilen L2r (B7, B8, F1 echt) fährt der Entwicklertest von Hand auf einem Rechner im LAN. Zeile I14 gilt nach Entscheidung E7 und ADR-0043 Ergänzung Karte #26: `--alle-namen` + `LOCALCHATA_ALLE_NAMEN=ja` → Start mit `alle_namen: true` (der Wortlaut „Exit 2“ der Zeile ist überholt). Nach der Ergänzung „Karte #35 Nacharbeit“: in den Zeilen C1–C14 (lokaler Namensraum) gilt in der Spalte „gefragt?“ zusätzlich „R nein“; Zeile F3 gilt mit dem Fach Vorwärts „erlaubt“ (128): nach 200 erlaubten Namen ist der erste neu gefragt, die letzten 128 nicht (der Wortlaut „256“ der Zeile ist überholt).
- [ ] **Mutationsprobe je Wächter** (ADR-0045, Durchsetzung): abgeschaltet werden einzeln Normalform, Label-Grenze, „nur der lokale Namensraum löst auf“, „eigene Adresse“, Zeitgrenze, Cache-Ablauf und „Ursprungsregel bei `--alle-namen`“, dazu aus der Ergänzung „Angriffsmatrix Karte #35“ „Ausschluss unspezifiziert/Multicast/Broadcast“ (E1, Matrix C6, C7), „numerischer Name ohne Auflösung“ (E2, C13) und „ohne Erlaubnis kein Cache-Eintrag“ (E3, F6), sowie „Umgebung immer geprüft“ (ADR-0043 Ergänzung Karte #26, I14), aus der Nacharbeit Karte #35 „Frist je Frage neu“, „lokaler Name fragt auch rückwärts“, „Fach übergreifend verdrängt“, „Eimer abgeschaltet“, „Gnadenfrist abgeschaltet“, „Gnadenfrist ohne Ende“, „Auffrischung braucht ein Zeichen“, „Auflösung wieder in `spawn_blocking`“, „ohne `catch_unwind`“ und „Satz in der Hilfe klein“; jede Mutante macht mindestens einen Test rot. Das Ergebnis steht im Bericht des Entwicklers.

### F0077 Über den Rückwärtsnamen öffnen

- [ ] **Abhängigkeitsprobe zuerst** (Review-Punkt; Bedingung des architekt in ADR-0045): der erste Commit dieser Interaction ist die Probe `dns-lookup` nach Skill `dependency-probe`. Sie enthält mindestens einen Happy Path (`127.0.0.1` → `localhost`) und eine Fault-Injection (nicht vergebene Adresse oder Resolver ohne Antwort → leeres Ergebnis bzw. Fehler ohne Panik, der Faden endet). Messwerte, Lizenz und Urteil stehen im Bericht. Trägt die Probe nicht, endet der Bau mit `blockiert`, und der architekt schreibt eine neue ADR.
- [ ] **Host-Urteil** (Unit, reine Operation; erster Test aus A00004): Namensauskunft als Tabelle `192.168.178.65 → nuc5.fritz.box` (rückwärts) und `boese.example → 192.168.178.65` (vorwärts, eigen).
  - Zuerst: `boese.example` → abgewiesen und **nie vorwärts gefragt** — die Tabelle zählt für `boese.example` 0 Vorwärtsfragen (die Rückwärtsfrage hängt an der lokalen Adresse, E8).
  - `nuc5.fritz.box:7430` über die lokale Adresse `192.168.178.65` → erlaubt mit Grund „Rückwärtsname“.
  - `nuc5.fritz.box.` und `NUC5.Fritz.Box` → erlaubt.
  - `nuc5.fritz.box` über die lokale Adresse `127.0.0.1` (Tabelle ohne Eintrag dafür) → abgewiesen.
- [ ] **Normalform** (Unit, rein):
  - `Chat.LAN.` → `chat.lan`; `xn--bcher-kva.example` bleibt unverändert.
  - Abgewiesen ohne Auflösung: `a_b.example` (Unterstrich), `a..b` (leeres Label), ein Label aus 64 Zeichen, ein Name aus 254 Zeichen. Erlaubt als Form: ein Label aus 63 Zeichen, ein Name aus 253 Zeichen.
  - `evil-local` und `local.evil.example` enden nicht auf dem Label `.local`.
  - Numerisch (E2): `2130706433`, `0x7f000001`, `0X7F000001`, `127.1`, `1.2.3.4.5` → abgewiesen ohne Auflösung; `1.2.3.4` bleibt ein IP-Literal (feste Liste); `a1.local` und `1a.local` sind keine numerischen Namen.
- [ ] **Rückwärtsname über das echte Programm** (Integration, Testnamen):
  - Erlaubt: `Host: nuc5.fritz.box:<p>`, `Host: NUC5.Fritz.Box:<p>`, `Host: nuc5.fritz.box.:<p>`.
  - Abgewiesen: `Host: boese.example:<p>` (die Testnamen zeigen ihn auf `127.0.0.1`, eine eigene Adresse, trotzdem 403); `Host: a_b.example:<p>`.
  - Bleibt erlaubt: `Host: 127.0.0.1:<p>`, `Host: localhost:<p>`; mit `--erreichbar-als chat.firma.de` ist `Host: chat.firma.de:<p>` erlaubt.
  - Gefragt / nicht gefragt (E6, E8; Stufe debug über `LOCALCHATA_PROTOKOLL_STUFE=debug`): nach `Host: boese.example:<p>` enthält `heute.log` keine Zeile `DEBUG grenzschutz Namensauskunft gefragt` mit `art=vorwaerts` und `schluessel=boese.example`; nach zwei Anfragen mit `Host: nuc5.fritz.box:<p>` genau eine Zeile mit `art=rueckwaerts` und `schluessel=127.0.0.1`.
  - Origin wörtlich (E5; der Rechnername `<r>` aus `hostname`): `POST /api/beitreten {"name":"beta"}` mit `Host: <r>.local.:<p>` und `Origin: http://<r>.local.:<p>` → 201 (Matrix D11); mit `Origin: http://<r>.local:<p>` → 403 `application/json` `{fehler}`, kein Beitritt (D12).
- [ ] **Rückwärtsname im echten Netz** (Integration, echtes DNS; nur wenn der Rechner eine Nicht-Loopback-IPv4 `<lan-ip>` hat und `getent hosts <lan-ip>` einen Namen `<name>` liefert, sonst als übersprungen gemeldet): Start mit `--host 0.0.0.0` ohne Testnamen; `GET http://<lan-ip>:<p>/api/gesundheit` mit `Host: <name>:<p>` → erlaubt.
- [ ] **Cache, Gleichzeitigkeit und Zeitgrenze** (Unit, gestellte Uhr, blockierende Testauskunft):
  - Gleichzeitigkeit: 4 verschiedene Namen, je Auflösung 3 s → zu keinem Zeitpunkt laufen mehr als 2 Auflösungen.
  - Zeitgrenze, Rechenbeispiel: die Auflösung von `nuc5.fritz.box` dauert 3 s und ergibt „erlaubt“. Anfrage A bei t = 0 → bei t = 1 s abgewiesen. Die Auflösung endet bei t = 3 s. Anfrage B bei t = 3,1 s → erlaubt aus dem Cache, ohne neue Auflösung (die Testauskunft zählt 1 Frage).
  - Geteilte Auflösung: zwei Anfragen für denselben Schlüssel bei t = 0 und t = 0,2 s → 1 Frage.
  - Ablauf „erlaubt“: bei t = 4:59 min keine neue Frage, nach 5 min eine neue Frage.
  - Ablauf „nicht erlaubt“, Rechenbeispiel: `drucker.local` abgewiesen bei t = 0 (1 Frage), wieder bei t = 59 s (weiter 1 Frage), wieder bei t = 61 s (2 Fragen).
  - Größe (Fach Vorwärts „erlaubt“, 128): nach 129 verschiedenen erlaubten Namen ist der zuerst eingetragene nicht mehr im Cache (seine nächste Anfrage fragt neu), der zuletzt eingetragene schon. Die übrigen Fächer: Kriterium „Nacharbeit Karte #35: Flut verdrängt keinen rechtmäßigen Namen“.
  - Die Zeitgrenze ergibt genau eine `WARNING grenzschutz`-Episode „Namensauflösung zu langsam“.
  - Ohne Erlaubnis keine Auflösung (E3), Rechenbeispiel: zwei Auflösungen laufen ab t = 0 (je 3 s). Anfrage für `neu.local` bei t = 0,5 s → sofort abgewiesen, die Testauskunft zählt für `neu.local` 0 Fragen, der Cache hat keinen Eintrag `neu.local`. Anfrage für `neu.local` bei t = 3,1 s → 1 Frage, erlaubt.
  - Rückwärts nicht flutbar (E8, Matrix F7): 300 verschiedene öffentliche Namen über die lokale Adresse `127.0.0.1` innerhalb von 60 s → 0 Vorwärtsfragen, höchstens 1 Rückwärtsfrage; jede Anfrage abgewiesen nach ≤ 1 s.
  - Fragen beobachtbar (E6, Speicher-Senke auf Stufe debug): je Frage der Testauskunft steht davor genau eine Zeile `DEBUG grenzschutz Namensauskunft gefragt` mit `art` und `schluessel`; ein Cache-Treffer schreibt keine.
- [ ] **Protokoll** (Integration, Testnamen): zwei Anfragen mit `Host: nuc5.fritz.box:<p>` → `heute.log` enthält genau eine Zeile `INFORMATION grenzschutz Host-Name über Rückwärtsauflösung erlaubt` mit `name=nuc5.fritz.box` und `adresse=127.0.0.1`.
- [ ] **Testknopf nur im Debug-Build** (Integration):
  - Debug-Build: `LOCALCHATA_TESTNAMEN=kaputt` → Exit 2, eine Zeile auf stderr nennt `LOCALCHATA_TESTNAMEN`.
  - Release-Build mit denselben Testnamen: `Host: nuc5.fritz.box:<p>` → abgewiesen (die echte Rückwärtsauflösung von `127.0.0.1` liefert `localhost`); `LOCALCHATA_TESTNAMEN=kaputt` → die Instanz startet.

- [ ] **Nacharbeit Karte #35: eine Frist je Anfrage, ein Weg je Name** (Unit über `Grenzschutz::host_pruefen`, blockierende Testauskunft; Muss aus dem Code-Review #35, ADR-0045 Ergänzung „Karte #35 Nacharbeit“ Punkt 1). Abnahme-Probe des `code-reviewer`, wörtlich übernommen: „Abnahme-Probe B1 (L1, Grenzschutz::host_pruefen, blockierende Testauskunft, lokale Adresse 192.168.178.65): Rückwärtsfrage für 192.168.178.65 blockiert 3 s, Vorwärtsfrage blockiert 3 s; Host chat.local:7430, nas.home.arpa:7430, dienst.internal:7430, chat:7430 → je abgewiesen nach ≤ 1 s + 100 ms ab Aufruf; zweite Anfrage nas.home.arpa:7430 bei t = 0,5 s während laufender Rückwärtsfrage → ≤ 1 s + 100 ms ab eigenem Eingang; Kontrolle boese.example:7430 bei blockierter Rückwärtsfrage → ≤ 1,1 s, 0 Vorwärtsfragen; Gegenprobe ohne Blockade chat.local → 127.0.0.1 erlaubt; Matrix F1 über host_pruefen: 50 gleichzeitige r<n>.local, jede Frage hängt 5 s → jede ≤ 1,1 s, höchstens 2 Fragen gleichzeitig. Heute rot (2,003/1,999 s).“ Dazu: Rückwärtsfrage blockiert 3 s, `chat.local → 127.0.0.1` antwortet sofort → erlaubt nach < 100 ms, die Testauskunft zählt 0 Rückwärtsfragen (heute rot: erlaubt erst nach etwa 1 s).
- [ ] **Nacharbeit Karte #35: Flut verdrängt keinen rechtmäßigen Namen** (Unit mit Testauskunft und Testuhr, Integration; Security-Review #35 W1, ADR-0045 Ergänzung Punkt 2). Abnahme-Probe des `security-reviewer`, wörtlich übernommen: „Abnahme-Probe W1: (L1, Testauskunft/Testuhr) x.local=127.0.0.1 und 127.0.0.2>chat.fritz.box je einmal erlaubt; danach 1.000 verschiedene einteilige Namen f<i> sofort beantwortet und abgeschlossen → x.local weiter erlaubt mit fragen("vorwaerts","x.local") == 1, chat.fritz.box über 127.0.0.2 erlaubt mit fragen("rueckwaerts","127.0.0.2") == 1; Varianten Flut auf .local/.internal/.home.arpa; F3 bleibt (300 erlaubte Namen verdrängen sich gegenseitig). Regel: negative verdrängen nie positive; Rückwärtseinträge nie von Vorwärtseinträgen; Gesamtzahl fest begrenzt (Zahl: architekt). (L2, Debug, LOCALCHATA_TESTNAMEN='127.0.0.2>chat.fritz.box', --host 0.0.0.0) chat.fritz.box über 127.0.0.2 → 200; 10 s lang 20 parallele Anfragen über 127.0.0.1 mit Host r<zufall>:P; gleichzeitig alle 100 ms chat.fritz.box über 127.0.0.2 → jede 200. (L2r Entwicklertest) 0 Abweisungen, heute 32/90.“ Lesart: die Zahl ist entschieden (vier Fächer, zusammen 256); damit alle 1.000 Flutnamen gefragt werden, rückt die Testuhr je Name 0,1 s vor (Rate). Dazu, Rechenbeispiele:
  - 200 erlaubte Namen nacheinander → der erste wird neu gefragt, die letzten 128 nicht (F3 neu).
  - `x.local` und `chat.fritz.box` (über `127.0.0.2`) erlaubt, danach 200 abgewiesene Namen → der älteste abgewiesene wird neu gefragt, `x.local` und `127.0.0.2` nicht.
  - 40 lokale Adressen ohne Rückwärtsnamen → der Rückwärtseintrag `127.0.0.2 → chat.fritz.box` bleibt (keine neue Rückwärtsfrage für `127.0.0.2`).
  - Nach einer gemischten Flut sind nie mehr als 256 fertige Einträge im Cache.
- [ ] **Nacharbeit Karte #35: Rate neuer Auflösungen** (Unit, Testuhr; ADR-0045 Ergänzung Punkt 2). Rechenbeispiel: die Testuhr steht, 25 neue Namen → genau 20 Fragen, 5 sofort abgewiesen ohne Frage und ohne Cache-Eintrag; Testuhr +1 s, 25 weitere neue Namen → 10 Fragen.
- [ ] **Nacharbeit Karte #35: Gnadenfrist** (Unit, Testuhr, blockierende Testauskunft; Security-Review #35 Anmerkung 2, ADR-0045 Ergänzung Punkt 3). Rechenbeispiel: `x.local` erlaubt bei t = 0. Bei t = 5:01 min sind beide Erlaubnisse durch hängende Fragen belegt → `x.local` erlaubt nach < 100 ms, 0 neue Fragen. Sind die Erlaubnisse frei, ist die nächste Anfrage sofort erlaubt und startet genau eine Auffrischung, auch bei leerem Eimer; antwortet sie „fremd“, ist die folgende Anfrage abgewiesen. Ohne Auffrischung ist `x.local` bei t = 15:01 min nicht mehr aus der Gnadenfrist erlaubt.
- [ ] **Nacharbeit Karte #35: Panik** (Unit; Code-Review #35 A2, ADR-0045 Ergänzung Punkt 3). Probe des `code-reviewer`, wörtlich: „Panik beim ersten Mal, +59 s abgewiesen ohne Frage, +61 s erlaubt“; die Panik steht als EXCEPTION mit Ort im Protokoll; eine Panik während einer Auffrischung → die folgende Anfrage ist abgewiesen.
- [ ] **Nacharbeit Karte #35: Ende ohne Warten** (B0421; Unit und Integration; Code-Review #35 A3, ADR-0045 Ergänzung Punkt 3): eine Laufzeit mit hängender Frage (Testauskunft blockiert 10 s) wird fallen gelassen → nach < 200 ms zurück. Das Prozessende im echten Programm prüft das folgende Kriterium.
- [ ] **Nacharbeit Karte #35: Prozessende** (B0485; Integration, echtes Programm; ADR-0010 Ergänzung „Karte #35 Nacharbeit, Prozessende“; US-14). L2r (nur Linux mit `nss-mdns`, `#[ignore]`, im Entwicklertest ausdrücklich gefahren, nicht im Standard-Gate): Wegwerf-Ordner, `--port 0`, ohne `LOCALCHATA_TESTNAMEN`. Vorbedingung: eine Auflösung von `gibtsnicht-<zufall>.local` dauert ≥ 2 s — sonst ist der Test rot mit „Vorbedingung fehlt: nss-mdns“, nicht übersprungen. `GET /api/gesundheit` mit `Host: gibtsnicht-<zufall>.local:<p>` (einmal auch wörtlich `gibtsnicht.local` wie US-14) → 403 nach ≈ 1 s; sofort SIGTERM → Prozessende < 1 s nach dem Signal, Exit 0, die letzte Zeile von `logs/<heute>.log` enthält `INFORMATION ende Instanz beendet`, stdout enthält die Startzeile vollständig (heute 4,0 s). Exit-Codes bleiben (Standard-Gate): SIGTERM ohne hängende Auflösung → Exit 0 mit letzter Zeile „Instanz beendet“; `--hilfe` in eine Pipe → stdout gleich dem Hilfetext, Exit 0; falsche Option → Exit 2 mit Meldung auf stderr; gesperrter Ordner → Exit 1.

### F0078 Name im lokalen Namensraum

- [ ] **Urteil** (Unit, reine Operation; Rest des ersten Tests aus A00004): Tabelle `chat.local → 192.168.178.65` (eigen) und `drucker.local → 192.168.178.99` (fremd).
  - `chat.local` → erlaubt mit Grund „lokaler Namensraum“; `drucker.local` → abgewiesen.
  - `local.boese.example` → abgewiesen, und die Tabelle zählt dafür 0 Fragen.
  - Kanonisch und Ausschluss (E1): `null.local → 0.0.0.0`, `null6.local → ::`, `mc.local → 224.0.0.251`, `bc.local → 255.255.255.255` → abgewiesen, „eigen?“ wird für diese Adressen nicht gefragt; `map.local → ::ffff:127.0.0.1` → „eigen?“ für `127.0.0.1` → erlaubt; `mix.local → 192.0.2.1, 127.0.0.1` → erlaubt.
- [ ] **Über das echte Programm** (Integration, Testnamen):
  - Erlaubt: `chat.local`, `chat`, `nas.home.arpa`, `dienst.internal` (alle auf `127.0.0.1`).
  - Abgewiesen: `drucker.local` (auf `192.0.2.1`, nicht eigen); `local.boese.example`; `evil-local`.
  - Abgewiesen trotz Eintrag auf `127.0.0.1`: `chat.fritz.box` (kein abgeleitetes Suffix); mit `--erreichbar-als chat.fritz.box` erlaubt.
  - Gefragt / nicht gefragt (E6; Stufe debug): zwei Anfragen mit `Host: chat.local:<p>`, dann `X.LOCAL:<p>` und `chat.local.:<p>` → insgesamt genau eine Zeile „Namensauskunft gefragt“ mit `art=vorwaerts` und `schluessel=chat.local`; `local.boese.example`, `x.localhost`, `2130706433` → keine Zeile mit `art=vorwaerts`.
- [ ] **Protokoll** (Integration, Testnamen): zwei Anfragen mit `Host: chat.local:<p>` → genau eine Zeile `INFORMATION grenzschutz Host-Name über lokalen Namensraum erlaubt` mit `name=chat.local` und `adresse=127.0.0.1`.

### F0079 Abweisung als verständliche Seite

- [ ] **Seite** (Unit, rein): Name `boese.example`, lokale Adresse `192.168.178.65`, Port 7430.
  - Die Seite enthält `<html lang="de">`, die Überschrift „Diese Adresse ist für localchatA nicht freigegeben“ und einen Link `<a href="http://192.168.178.65:7430/">http://192.168.178.65:7430/</a>`.
  - Sie enthält den Satz „Der Betreiber kann den Namen boese.example mit `--erreichbar-als boese.example` beim Start freigeben.“ und einen Satz, der „fremde Websites“ als Grund nennt.
  - Sie enthält kein `<style`, kein `style=`, kein `<script` und kein `on…=`-Attribut.
  - Lokale Adresse `::1` → Link `http://[::1]:7430/`.
  - Name `<script>x` → in der Seite nur als `&lt;script&gt;x`.
  - Ohne Namen (E4): Überschrift und Link wie oben, Satz zum Warum; kein Satz mit `--erreichbar-als`.
- [ ] **Browser bekommt die Seite** (Integration):
  - `GET /` mit `Host: boese.example:<p>` und `Accept: text/html` → 403, `Content-Type: text/html; charset=utf-8`, die Seite aus dem Unit-Kriterium mit dem Link `http://127.0.0.1:<p>/`.
  - Ebenso mit `Sec-Fetch-Dest: document` und `Accept: */*`.
  - Die Antwort trägt den Header-Satz aus ADR-0014 Regel 5 mit unveränderter `Content-Security-Policy` und keinen `Location`-Kopf.
- [ ] **Agent bekommt Text** (Integration): `GET /api/gesundheit` mit `Host: boese.example:<p>` und `Accept: application/json` → 403 mit `Content-Type: text/plain; charset=utf-8`. Der Text enthält `boese.example`, `http://127.0.0.1:<p>/`, `--erreichbar-als boese.example` und „Namen des eigenen Netzes gelten automatisch“. Ohne `Accept`-Kopf gilt dasselbe.
- [ ] **Ohne `Host`** (Integration, roh über Socket; E4, Matrix G5, D9, D10): HTTP/1.0 `GET /` ohne `Host` mit `Accept: text/html` → 403 `text/html; charset=utf-8`, Überschrift und Link `http://127.0.0.1:<p>/`, kein `--erreichbar-als`; dasselbe mit `Accept: */*` → 403 `text/plain` „nennt keinen Host“ mit `http://127.0.0.1:<p>/`; zwei `Host`-Köpfe mit `Accept: text/html` → 403 ohne Seite.
- [ ] **Im Browser** (E2E, Playwright, Chromium mit `--host-resolver-rules=MAP boese.example 127.0.0.1`):
  - `http://boese.example:<p>/` öffnen → die Überschrift „Diese Adresse ist für localchatA nicht freigegeben“ ist sichtbar.
  - Ein Klick auf den Link öffnet `http://127.0.0.1:<p>/` mit dem Titel `localchatA`.
  - Konsole: keine unbehandelte Ausnahme und kein CSP-Verstoß. Erlaubt sind nur die Meldungen „Failed to load resource“ zu den 403 auf das Dokument unter `boese.example` und auf `/favicon.ico` unter `boese.example`.

### F0080 Schalter `--alle-namen`

- [ ] **Aufruf** (Unit, rein; erster Test aus A00003):
  - `["/tmp/x", "--alle-namen"]` → `alle_namen: true`; ohne die Option → `false`.
  - Umgebung `LOCALCHATA_ALLE_NAMEN=1` → `true`; `0` oder leer → `false`; `ja` → Aufruffehler (Exit 2), der die Variable nennt.
  - `--alle-namen` mit `LOCALCHATA_ALLE_NAMEN=0` → `true` (die Option geht vor); `--alle-namen` mit `LOCALCHATA_ALLE_NAMEN=ja` → `true`, die Umgebung wird nicht gelesen (ADR-0043 Ergänzung Karte #26, E7; Muster `--protokoll-stufe`). Gegenprobe ohne die Option: `LOCALCHATA_ALLE_NAMEN=ja` → Exit 2. Mutante „Umgebung immer geprüft“ wird rot.
  - Integration: `LOCALCHATA_ALLE_NAMEN=ja localchatA <ordner> --alle-namen …` → Startzeile, `/api/gesundheit` meldet `alle_namen: true`, WARNING vor der Startzeile; ohne `--alle-namen` → Exit 2, kein Listener.
  - `--hilfe` nennt `--alle-namen` und `LOCALCHATA_ALLE_NAMEN`, auf eigener Zeile „Risiko: Eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.“, und dass Namen des eigenen Netzes automatisch gelten.
- [ ] **Jeder Name** (Integration, `--alle-namen`, Testnamen gesetzt):
  - `Host: boese.example:<p>`, `chat.lan:<p>`, `x.local:<p>` → erlaubt; die Namensauskunft wird nie gefragt (Unit mit zählender Tabelle: 0 Fragen; im echten Programm auf Stufe debug keine Zeile „Namensauskunft gefragt“, Matrix I1).
  - Abgewiesen bleibt: `Host: a_b.example:<p>`, eine Anfrage ohne `Host`-Kopf und `POST /api/beitreten` mit `Host: boese.example:<p>` und `Origin: http://andere.example:<p>`.
  - Bewusst offen (Rebinding-Weg, ADR-0045 Regel 9): `POST /api/beitreten {"name":"beta"}` mit `Host: boese.example:<p>` und `Origin: http://boese.example:<p>` → 201.
- [ ] **Sichtbar** (Integration):
  - Mit `--alle-namen`, ebenso mit `LOCALCHATA_ALLE_NAMEN=1`, steht vor der Startzeile in `heute.log` und auf stderr eine `WARNING grenzschutz`-Zeile mit dem Text „--alle-namen ist aktiv: Jeder Host-Name wird angenommen — eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.“ (escaped nach ADR-0040).
  - `/api/gesundheit` meldet `alle_namen: true`; ohne den Schalter `alle_namen: false` und keine solche Zeile.
- [ ] **Nacharbeit Karte #35: Wortlaut der Risikosätze** (Integration und Unit; Muss aus dem Rechtschreib-Review #35, ADR-0045 Ergänzung Punkt 5). Abnahme-Probe des `rechtschreib-reviewer`, wörtlich übernommen: „Probe: `cargo run -- --hilfe | grep -c 'Risiko: eine fremde'` = 0 und `… | grep -c 'Risiko: Eine fremde Website, die ein Teilnehmer öffnet, kann über DNS-Rebinding Channels mitlesen, Nachrichten schreiben und über die Frist Verläufe löschen.'` = 1; Unit-Test die_hilfe_nennt_alle_namen_… mit anpassen; ADR-0045 Regel 9 Wortlaut per Ergänzung angleichen.“ (Der Wortlaut der ADR ist angeglichen.) Dazu: die WARNING beginnt mit „--alle-namen ist aktiv: Jeder Host-Name wird angenommen“; der Unit-Test, der Hilfe und Startwarnung vergleicht, prüft die Gleichheit ab „fremde Website“.

## Betroffene Verzeichnisstruktur

- `src/http/`: neu `namensauskunft.rs` (Port, echte Implementierung, Tabelle, Testknopf-Parser), `grenzschutz.rs` (Normalform, Urteil, Cache mit Semaphore und Zeitgrenze, Abweisungsseite, `alle_namen`), `gesundheit.rs` (Feld `alle_namen`); Unit-Tests im Modul.
- `src/start/`: `ausliefern.rs` (lokale Adresse je Verbindung), `aufruf.rs` (`--alle-namen`, Hilfe), `mod.rs` (Namensauskunft erzeugen, Startwarnung), ggf. ein Modul für `LOCALCHATA_ALLE_NAMEN` nach dem Muster `protokollstufe.rs`.
- `tests/`: Integration über das echte Programm (Muster `tests/gemeinsam/mod.rs`), Probe `dns-lookup`; `e2e/`: Abweisungsseite im Browser.
- `Cargo.toml`: neue Abhängigkeit `dns-lookup`.

## Technische Überlegungen

### Ablauf

1. **Start**: Aufruf lesen (`--alle-namen`, Umgebung) → … → Protokoll öffnen → bei `alle_namen` WARNING → Namensauskunft erzeugen (echt; im Debug-Build mit `LOCALCHATA_TESTNAMEN` als Tabelle) → Grenzschutz mit Rechnername, `--erreichbar-als`, Namensauskunft, `alle_namen` → Binden → Startzeile.
2. **Verbindung**: Annahmeschleife setzt `ConnectInfo` (Gegenstelle) und die lokale Adresse als Extensions.
3. **Anfrage im Grenzschutz**:
   - 3.1 Host-Anteil lesen; fehlt er oder ist er unlesbar → 403.
   - 3.2 Normalform; verletzt → 403 ohne Auflösung.
   - 3.3 Wenn `alle_namen`: weiter zu 3.6.
   - 3.4 Feste Liste (Regel 2) → erlaubt.
   - 3.5 Auskunft mit Cache, eine Frist je Anfrage (1 s ab Eingang): außerhalb des lokalen Namensraums nur rückwärts für die lokale Adresse (Regel 3), im lokalen Namensraum nur vorwärts für den Namen und je Adresse „eigen?“ (Regel 4); eine neue Auflösung nur mit Erlaubnis und Zeichen, auf eigenem Faden; positiver Eintrag in der Gnadenfrist → sofort erlaubt, Auffrischung im Hintergrund → Urteil.
     - 3.5.1 Wenn abgewiesen: Episode `grenzschutz`, Antwort 403 als Seite (Dokument) oder Text.
   - 3.6 Ursprungsregel (ADR-0014 Regel 2) → Route; Header-Satz auf jede Antwort.

### Grobentwurf (Vorplanung)

Die Bubbles der WBS sind Vorplanung; welche Bubbles es am Ende werden, entscheidet der Entwickler.

- F0077: B0364 Abhängigkeitsprobe `dns-lookup` · B0365 Host in Normalform · B0366 Lokale Adresse je Verbindung · B0367 Namensauskunft als Tabelle und Testknopf · B0368 Echte Namensauskunft · B0369 Host-Urteil · B0370 Auskunft mit Cache, Gleichzeitigkeit und Zeitgrenze · B0371 Rückwärtsname im Grenzschutz
- F0078: B0372 Urteil im lokalen Namensraum · B0373 Vorwärtsauflösung im Grenzschutz
- F0079: B0374 Abweisungsseite bilden · B0375 Seite oder Text ausliefern
- F0080: B0376 Aufruf um `--alle-namen` · B0377 Alle Namen annehmen · B0378 Alle Namen sichtbar machen
- Nacharbeit Karte #35 (in dieser Reihenfolge): F0077 B0417 Eine Frist je Anfrage, ein Weg je Name · B0418 Cache in vier Fächern · B0419 Rate neuer Auflösungen · B0420 Gnadenfrist mit Auffrischen im Hintergrund · B0421 Auflösung auf eigenem Faden · B0422 Panik in der Auflösung belegt · B0485 Prozessende ohne Aufräumen der C-Bibliothek (braucht B0421); F0080 B0423 Risikosätze angeglichen

Elemente nach ADR-0045, „Schnitt“:

- `Namensauskunft` (Port, Provider) — Rückwärtsnamen, Adressen eines Namens, eigene Adresse; echte Implementierung und Tabelle.
  - `namen_der_adresse(IpAddr) -> Vec<String>`
  - `adressen_des_namens(&str) -> Vec<IpAddr>`
  - `ist_eigene_adresse(IpAddr) -> bool`
- `host_urteil(normalisierter Name, feste Liste, Rückwärtsnamen, Vorwärtsadressen mit „eigen?“) -> Erlaubt(Grund) | Abgewiesen` (Operation, rein).
- `Grund` (Aufzählung: fest, Rückwärtsname, lokaler Namensraum, alle Namen).
- Auskunfts-Cache (Operation mit Uhr; einziger Schreiber ist der Grenzschutz) — ab Nacharbeit Karte #35 mit vier Fächern, Eimer und Gnadenfrist; die Auflösung läuft auf einem eigenen Faden `namensauskunft`.
- `LokaleAdresse` (Request-Extension, immutable).

### Änderungen an bestehenden Klassen

- `Grenzschutz::neu` bekommt Namensauskunft und `alle_namen`; `host_erlaubt` wird asynchron (Auflösung) oder teilt sich in Urteil und Auskunft.
- `abweisung` in `grenzschutz.rs`: Seite oder Text je `Sec-Fetch-Dest`/`Accept`, mit lokaler Adresse und Port.
- `Startauftrag` + `alle_namen`; `HILFE` + Zeilen.
- Gesundheitsantwort + `alle_namen`.
- Annahmeschleife: zweite Extension.
- `start::ausfuehren` liefert den Exit-Code als Zahl (`u8`) statt `ExitCode`; `main` endet auf Unix nach `drop(laufzeit)` und `flush` mit `_exit` (B0485, ADR-0010 Ergänzung „Karte #35 Nacharbeit, Prozessende“); `libc` direkte Abhängigkeit (nur Unix), `unsafe_code` in `Cargo.toml` `"deny"`, `src/lib.rs` `#![forbid(unsafe_code)]`.

## Tests

Nach Skill `test-pyramide` und ADR-0009; die Szenarien der User Story werden Tests.

- **Unit (rein)**: Normalform, Host-Urteil (Matrix aus A00004), Urteil im lokalen Namensraum, Cache/Semaphore/Zeitgrenze mit gestellter Uhr und blockierender Testauskunft, Testknopf-Parser, Abweisungsseite und Text, Aufruf `--alle-namen`.
- **Integration (echtes Programm)**: Testnamen über `LOCALCHATA_TESTNAMEN`, Rückwärtsname im echten Netz (bedingt), Release-Probe, Seite und Text, `--alle-namen`, Protokollzeilen, Health-Check, Angriffsmatrix ADR-0045; Prozessende bei hängender Auflösung als L2r (`#[ignore]`, Vorbedingung `nss-mdns` rot statt übersprungen) und Exit-Codes 0, 1, 2 im Standard-Gate (B0485).
- **E2E**: Abweisungsseite unter `boese.example` mit Link zur Oberfläche; die bestehenden Playwright-Tests bleiben grün.
- **Probe**: `dns-lookup` als erster Commit (Skill `dependency-probe`), danach als Regressionsschutz im Repo, soweit sie ohne Netz läuft.

## Abhängigkeiten

- Abhängig von: I0004 Namen wählen und beitreten (gruen, R00003; darin F0006 Grenzschutz); F0051 Protokoll schreibt und F0055 Grenzen, Episoden und Tagesbudget (beide gruen, R00014).
- Blockiert: keine.
- Vor dem Bau: die Angriffsmatrix des `security-reviewer` in ADR-0045 — liegt vor (Commit 2fc7c7e, Entscheidungen E1–E8; E7 nach ADR-0043 Ergänzung Karte #26, Commit 5f3b3c5).

## Offene Fragen

- Keine für den Bau. Für R00024 entschieden (den Wortlaut der ADR gleicht der architekt an):
  - (1) `LOCALCHATA_TESTNAMEN` ersetzt Rückwärts- und Vorwärtsauflösung; `ist_eigene_adresse` bleibt echt. So ist „eigen“ über `127.0.0.1` und „fremd“ über `192.0.2.1` deterministisch.
  - (2) Der Text für Nicht-Browser ist nur in seinen Bestandteilen festgelegt (Name, Adresse, `--erreichbar-als <name>`, „Namen des eigenen Netzes gelten automatisch“), nicht im Wortlaut.
  - (3) ~~Ohne `Host`-Kopf bleibt die Abweisung `text/plain` wie heute, auch für Browser.~~ Ersetzt durch Entscheidung E4 (ADR-0045 Ergänzung „Angriffsmatrix Karte #35“): ein Browser-Dokument ohne `Host` bekommt die Seite ohne Namen.
  - (4) ~~`--alle-namen` mit einem falschen Wert in `LOCALCHATA_ALLE_NAMEN` ergibt Exit 2.~~ Ersetzt durch ADR-0043 Ergänzung Karte #26 (E7): steht der Schalter im Aufruf, wird die Umgebung nicht gelesen; Exit 2 nur ohne den Schalter.
- Angriffsmatrix: liegt vor (ADR-0045, Abschnitt „Angriffsmatrix“, E1–E8 entschieden). Die Matrix-Zeile I14 und Regel 9 sagen noch „Exit 2“; den Wortlaut gleicht der architekt an — für diese Anforderung gilt E7.

## Manuelle Vorbereitungstätigkeiten

- ~~Der `security-reviewer` ergänzt ADR-0045 um den Abschnitt „Angriffsmatrix“, bevor gebaut wird (Director).~~ Erledigt 2026-10-29 (Commit 2fc7c7e).

## Manuelle Nachbereitungstätigkeiten

- `AI-Team/umgebung.md` (Director): Testknopf `LOCALCHATA_TESTNAMEN` (nur Debug-Build) und Schalter `--alle-namen`/`LOCALCHATA_ALLE_NAMEN` eintragen (ADR-0045, Folgeaufgaben).
- Abnahme auf dem Rechner des Betreibers über den echten Namen (`http://nuc5.fritz.box:<port>/`, User Story US-1).

## Umfang

Gezählt aus der WBS (Bubbles, 2026-10-29). 0,4 h ist die Untergrenze je Bubble nach dem Skill `work-breakdown-structure`; die Ist-Zeiten des Projekts liegen darunter. Der Wert ist deshalb ein Richtwert, für diese Bubble-Typen ungemessen.

- F0077 Über den Rückwärtsnamen öffnen: 14 Bubbles · 9 klar (3,6 h) · 5 unklar (10–20 h); davon offen die Nacharbeit Karte #35 B0417–B0422: 6 Bubbles · 4 klar (1,6 h) · 2 unklar (B0420, B0421; 4–8 h)
- F0078 Name im lokalen Namensraum: 2 Bubbles · 2 klar (0,8 h)
- F0079 Abweisung als verständliche Seite: 2 Bubbles · 1 klar (0,4 h) · 1 unklar (2–4 h)
- F0080 Schalter `--alle-namen`: 4 Bubbles · 4 klar (1,6 h); davon offen B0423 (0,4 h)
- Nachtrag 2026-10-29 (Prozessende, ADR-0010 Ergänzung): + B0485 unter F0077, unklar (2–4 h; Vorbedingung `nss-mdns` im Test ungemessen). F0077 damit 15 Bubbles · 9 klar (3,6 h) · 6 unklar (12–24 h); zusammen 23 Bubbles · 6,4 h klar + 14–28 h unklar; B0485 braucht B0421.
- Zusammen (vor dem Nachtrag): 22 Bubbles · 6,4 h klar + 12–24 h unklar; Wellen F0077 → F0078 ‖ F0079 ‖ F0080. Offen nach dem Bau (Nacharbeit Karte #35): 7 Bubbles · 5 klar (2,0 h) · 2 unklar (4–8 h); B0423 ist unabhängig und parallel baubar

## Warum löst diese Anforderung das Problem? (Pflicht)

Auslöser ist die Meldung „unter dem Namen … nicht freigegeben“, wenn der Betreiber die Instanz über den Namen öffnet, den die Fritz!Box seinem Rechner gibt. Der Wunsch heißt: jeder Name, der auf die richtige IP zeigt. Genau das ist aber der Weg des DNS-Rebinding. Ein Name ist nicht sicher, weil er auf die Instanz zeigt, sondern weil die Antwort aus dem eigenen Netz kommt.

Die Wirkungskette:

- Der Grenzschutz erlaubt den Rückwärtsnamen der angesprochenen Adresse und Namen im lokalen Namensraum. Deshalb funktioniert der Router-Name ohne Startoption, und ein Angreifer mit eigener Domain bekommt trotzdem keinen Zugriff.
- Öffentliche Namen lösen nie eine Auflösung aus, und Auflösungen sind begrenzt. Deshalb kann ein frei wählbarer `Host` die Instanz nicht blockieren.
- Die Abweisung wird für den Browser eine Seite mit Link. Deshalb kommt der Teilnehmer ohne Wissen über Startoptionen sofort weiter, und der Betreiber erfährt seinen Ausweg.
- `--alle-namen` ist sichtbar riskant. Deshalb bleibt die Ausnahme eine bewusste Entscheidung.

Der Hebel liegt im Grenzschutz, weil dort die Frage „welcher Name?“ schon heute entschieden wird. Eine Freigabeseite für den Betreiber (Option C der ADR) wäre die erste Verwaltungsfunktion ohne Rechte, und ein Redirect (Option E) nähme den Namen aus der Adresszeile.

## Missing-Docs

- Glossar fehlt (`Dokumentation/Domaenenwissen/` existiert nicht). Begriffe wie Namensauskunft, lokaler Namensraum, Rückwärtsname und Normalform stammen aus ADR-0045.
- Verhalten von `getnameinfo` mit `nss-mdns` und bei hängendem Resolver ist nur aus der Wegwerf-Probe des architekt bekannt; die Probe B0364 schließt die Lücke.

## Notizen

### Angriffsmatrix in den Bubbles (2026-10-29, Karte #35)

E1 → B0372 (kanonisch, Ausschluss), E2 → B0365 (numerische Namen), E3 → B0370 (`try_acquire`, kein Cache-Eintrag ohne Erlaubnis), E4 → B0375 (Seite ohne Namen), E5 → B0371 (Origin wörtlich, D11/D12), E6 → B0370 (DEBUG-Zeile), B0371 und B0373 (gefragt / nicht gefragt im echten Programm), E7 → B0376 (Option vor falscher Umgebung), E8 → B0369 (nie vorwärts gefragt) und B0370 (Rückwärts höchstens einmal je lokaler Adresse). Neue Mutanten: „Ausschluss unspezifiziert/Multicast/Broadcast“, „numerischer Name ohne Auflösung“, „ohne Erlaubnis kein Cache-Eintrag“. Keine neuen Bubbles; der Umfang bleibt bei 15 Bubbles, gezählt aus der WBS.

### Nacharbeit Karte #35 (2026-10-29)

Weiche nach den Auto-Reviews #35: Code-Review B1 (Wartezeit bis 2 s statt ≤ 1 s je Anfrage) und Rechtschreib-Review Muss (Risikosatz) halten Zusagen dieser Anforderung nicht — als Befund an einer Befundbehebung bzw. am eigenen Paket keine neue Interaction, sondern Nacharbeit unter derselben Anforderung (B0417, B0423). Security-Review W1 (Cache-Verdrängung) hält die Zusage aus F6 der Angriffsmatrix nicht → B0418, B0419. In dieselbe Nacharbeit genommen nach ADR-0045 Ergänzung Punkt 3 („dieselbe Stelle: mit hinein“): Gnadenfrist (Security Anmerkung 2, B0420), Ende ohne Warten (Code A3, B0421), Panik belegt (Code A2, B0422). Die übrigen Sollte und Optional der Reviews #35 sind Härtung in R00028.

Nachtrag 2026-10-29 (Befund des `entwickler` zu B0421): der eigene Faden lässt die Laufzeit in < 200 ms enden, das echte Programm wartet beim Verlassen von `main` aber in `exit()` der glibc auf den Faden (4,0 s). Entschieden in ADR-0010 Ergänzung „Karte #35 Nacharbeit, Prozessende“ (`_exit` nach dem Abbau der Laufzeit). Das Kriterium „Ende ohne Warten“ ist geteilt: B0421 behält „Laufzeit < 200 ms“, das L2r-Kriterium „Prozessende < 1 s“ gehört zur neuen Bubble B0485 (eigene Dateien, eigene Mutante, eigene Vorbedingung). US-14 bleibt unverändert; sie prüft beide zusammen von außen.

### Verworfene Alternativen

In ADR-0045 entschieden, hier nur genannt:

- Jeden Namen zur Laufzeit auflösen und annehmen, wenn er auf eine eigene Adresse zeigt (Option A, Wortlaut des Befunds): verworfen, weil damit DNS-Rebinding wieder vollständig offen wäre. Als Ausnahme lebt sie im Schalter `--alle-namen` weiter, ohne Auflösung.
- Nur der Rückwärtsname ohne lokalen Namensraum und ohne Seite (Option B): verworfen, weil `.local`-Aliase und der ratlose Teilnehmer bleiben.
- Freigabeseite für den Betreiber (Option C): verworfen, weil sie eine Verwaltungsfunktion wäre und gerade den Angreifernamen am leichtesten freigibt.
- Umleitung auf das IP-Literal (Option E): verworfen, weil der Name aus der Adresszeile verschwindet und Sitzung und Entwürfe am IP-Ursprung hängen.

### Bewusst außerhalb

- Öffentliche Namen und Aliase unter der Router-Domäne, die nicht der Rückwärtsname sind (`chat.fritz.box`, Split-DNS) — weiter `--erreichbar-als`.
- Umschalten von `--alle-namen` zur Laufzeit.
- Ein Artboard für die Abweisungsseite: sie ist ohne Stil (ADR-0045 Regel 7) und gehört zu keinem Dialog.
