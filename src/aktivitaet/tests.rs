//! B0684 (R00045 F0161, ADR-0051 P1, P2): Tabelle, Gültigkeit, Ablauf — mit gestellter Uhr und aufzeichnender
//! Veröffentlichung.

use super::bremse::Grenze;
use super::*;
use crate::uhr::GestellteUhr;

const T0: &str = "2026-10-30T09:00:00.000+01:00";

fn t(ms: i64) -> Zeit {
    Zeit::parse_from_rfc3339(T0).expect("Zeit") + TimeDelta::milliseconds(ms)
}

fn kennung(text: &str) -> Kennung {
    Kennung::bilden(text).expect("Kennung")
}

fn anna() -> Kennung {
    kennung("anna@10.0.0.7")
}

fn ben() -> Kennung {
    kennung("ben@10.0.0.8")
}

fn claude() -> Kennung {
    kennung("claude@10.0.0.5")
}

/// Alle Kennungen der Tests kommen von einer Adresse — der Eimer je Herkunft ist einer.
fn herkunft() -> IpAddr {
    "10.0.0.1".parse().expect("IP")
}

fn allgemein() -> Gespraech {
    Gespraech::channel("allgemein")
}

/// Aktivität mit gestellter Uhr bei t0 und einer Liste aller Veröffentlichungen samt „unter der Sperre“.
struct Aufbau {
    uhr: Arc<GestellteUhr>,
    aktivitaet: Arc<Aktivitaet>,
    gemeldet: Arc<Mutex<Vec<(Aenderung, bool)>>>,
}

impl Aufbau {
    fn neu() -> Aufbau {
        let uhr = Arc::new(GestellteUhr::neu(t(0)));
        let selbst: Arc<std::sync::OnceLock<std::sync::Weak<Aktivitaet>>> = Arc::default();
        let gemeldet: Arc<Mutex<Vec<(Aenderung, bool)>>> = Arc::default();
        let (selbst_im_aufruf, gemeldet_im_aufruf) = (Arc::clone(&selbst), Arc::clone(&gemeldet));
        let aktivitaet = Arc::new(
            Aktivitaet::neu(Arc::clone(&uhr) as Arc<dyn Uhr>).mit_veroeffentlichen(Arc::new(
                move |aenderung: &Aenderung| {
                    let unter_sperre = selbst_im_aufruf
                        .get()
                        .and_then(std::sync::Weak::upgrade)
                        .is_some_and(|aktivitaet| aktivitaet.inneres.try_lock().is_err());
                    gemeldet_im_aufruf
                        .lock()
                        .expect("Liste")
                        .push((aenderung.clone(), unter_sperre));
                },
            )),
        );
        selbst
            .set(Arc::downgrade(&aktivitaet))
            .expect("einmal gesetzt");
        Aufbau {
            uhr,
            aktivitaet,
            gemeldet,
        }
    }

    fn um(&self, ms: i64) -> &Aufbau {
        self.uhr.stellen(t(ms));
        self
    }

    fn setzen(
        &self,
        wer: &Kennung,
        zustand: Zustand,
        gueltig_s: Option<u32>,
    ) -> Result<Gesetzt, Abweisung> {
        self.aktivitaet
            .setzen(wer, herkunft(), &allgemein(), zustand, gueltig_s)
            .inspect(|gesetzt| {
                assert_eq!(gesetzt.jetzt, self.uhr.jetzt(), "jetzt ist der Eingang");
            })
    }

    /// Kennungen, die `ben` in `# allgemein` liest.
    fn liest_ben(&self) -> Vec<String> {
        self.aktivitaet
            .lesen(&allgemein(), &ben())
            .aktivitaeten
            .iter()
            .map(|aktiv| aktiv.kennung.als_text().to_string())
            .collect()
    }

    fn gemeldet(&self) -> Vec<(Aenderung, bool)> {
        self.gemeldet.lock().expect("Liste").clone()
    }
}

/// P1 erster Satz: `tippt` ohne Angabe gilt 6 s ab Eingang — bei t0 + 5,999 s gelesen, bei t0 + 6,000 s nicht mehr.
#[test]
fn p1_tippt_gilt_sechs_sekunden_ab_eingang() {
    let aufbau = Aufbau::neu();

    let gesetzt = aufbau.setzen(&anna(), Zustand::Tippt, None);

    assert_eq!(
        gesetzt,
        Ok(Gesetzt {
            zustand: Zustand::Tippt,
            bis: t(6_000),
            jetzt: t(0)
        })
    );
    assert_eq!(aufbau.um(5_999).liest_ben(), ["anna@10.0.0.7"]);
    assert_eq!(aufbau.um(6_000).liest_ben(), Vec::<String>::new());
}

/// P1 zweiter Satz (Mutante M4 „`bis` addiert“): `denkt_nach` gilt 30 s; die Erneuerung bei t0 + 20 s gilt ab ihrem
/// Eingang bis t0 + 50 s, nicht bis t0 + 60 s.
#[test]
fn p1_erneuerung_gilt_ab_eingang_nicht_addiert() {
    let aufbau = Aufbau::neu();

    let erst = aufbau.setzen(&claude(), Zustand::DenktNach, None);
    let erneuert = aufbau
        .um(20_000)
        .setzen(&claude(), Zustand::DenktNach, None);

    assert_eq!(erst.map(|g| g.bis), Ok(t(30_000)));
    assert_eq!(erneuert.map(|g| g.bis), Ok(t(50_000)));
    assert_eq!(
        aufbau
            .um(49_999)
            .aktivitaet
            .lesen(&allgemein(), &ben())
            .aktivitaeten,
        [Aktiv {
            kennung: claude(),
            zustand: Zustand::DenktNach,
            bis: t(50_000)
        }]
    );
    assert_eq!(aufbau.um(50_000).liest_ben(), Vec::<String>::new());
}

/// P1 dritter Satz (Mutante M5 „begrenzt statt abgewiesen“): 120 s gehen, 121 und 0 werden ohne Wirkung abgewiesen —
/// kein Eintrag, kein Signal, der bisherige Zustand bleibt.
#[test]
fn p1_gueltigkeit_eins_bis_hundertzwanzig_sonst_abgewiesen_ohne_wirkung() {
    let aufbau = Aufbau::neu();

    assert_eq!(
        aufbau
            .setzen(&claude(), Zustand::DenktNach, Some(120))
            .map(|g| g.bis),
        Ok(t(120_000))
    );
    assert_eq!(
        aufbau
            .setzen(&anna(), Zustand::Tippt, Some(1))
            .map(|g| g.bis),
        Ok(t(1_000))
    );
    let vorher = aufbau.gemeldet().len();
    for falsch in [121, 0, 500, u32::MAX] {
        assert_eq!(
            aufbau.setzen(&claude(), Zustand::Tippt, Some(falsch)),
            Err(Abweisung::Gueltigkeit),
            "{falsch}"
        );
        assert_eq!(
            aufbau.setzen(&ben(), Zustand::DenktNach, Some(falsch)),
            Err(Abweisung::Gueltigkeit),
            "{falsch}"
        );
    }

    assert_eq!(aufbau.gemeldet().len(), vorher, "abgewiesen meldet nichts");
    let gelesen = aufbau.aktivitaet.lesen(&allgemein(), &anna()).aktivitaeten;
    assert_eq!(
        gelesen,
        [Aktiv {
            kennung: claude(),
            zustand: Zustand::DenktNach,
            bis: t(120_000)
        }],
        "claude behält den alten Zustand, ben hat keinen"
    );
}

/// P1 letzter Satz (Mutante M14 „Uhr rückwärts verlängert“): geht die Uhr auf t0 − 10 s zurück, liegt `bis` 16 s
/// vorn, mehr als die Gültigkeit von 6 s — der Eintrag gilt als abgelaufen.
#[test]
fn p1_geht_die_uhr_zurueck_gilt_der_eintrag_als_abgelaufen() {
    let aufbau = Aufbau::neu();
    aufbau
        .setzen(&anna(), Zustand::Tippt, None)
        .expect("gesetzt");

    assert_eq!(aufbau.um(-10_000).liest_ben(), Vec::<String>::new());
    assert_eq!(
        aufbau.um(-1).liest_ben(),
        Vec::<String>::new(),
        "1 ms zurück: bis − jetzt = 6,001 s"
    );
    assert_eq!(
        aufbau.um(0).liest_ben(),
        ["anna@10.0.0.7"],
        "als Regel, nicht als Löschung"
    );
}

/// P2 erster Satz: `tippt`, dann `denkt_nach` derselben Kennung im selben Gespräch → ein Eintrag `denkt_nach`; die
/// Veröffentlichung zählt zwei Signale, jedes unter der Sperre.
#[test]
fn p2_ein_eintrag_je_kennung_und_gespraech_jede_aenderung_unter_der_sperre() {
    let aufbau = Aufbau::neu();

    aufbau
        .setzen(&claude(), Zustand::Tippt, None)
        .expect("tippt");
    aufbau
        .um(1_000)
        .setzen(&claude(), Zustand::DenktNach, None)
        .expect("denkt_nach");

    assert_eq!(
        aufbau.aktivitaet.lesen(&allgemein(), &ben()).aktivitaeten,
        [Aktiv {
            kennung: claude(),
            zustand: Zustand::DenktNach,
            bis: t(31_000)
        }]
    );
    let gemeldet = aufbau.gemeldet();
    assert_eq!(gemeldet.len(), 2, "{gemeldet:#?}");
    assert!(
        gemeldet.iter().all(|(_, unter_sperre)| *unter_sperre),
        "{gemeldet:#?}"
    );
    assert_eq!(
        gemeldet[1].0,
        Aenderung {
            gespraech: allgemein(),
            kennung: claude(),
            aktiv: Some(Gesetzt {
                zustand: Zustand::DenktNach,
                bis: t(31_000),
                jetzt: t(1_000)
            })
        }
    );
}

/// P2 zweiter Satz: 16 Gespräche je Kennung gehen, das 17. wird ohne Wirkung abgewiesen; Erneuern in einem der 16
/// geht weiter, und ist eines abgelaufen, wird sein Platz frei.
#[test]
fn p2_sechzehn_gespraeche_je_kennung_das_siebzehnte_abgewiesen() {
    let aufbau = Aufbau::neu();
    let gespraech = |n: usize| Gespraech::channel(&format!("c{n}"));
    for n in 0..JE_KENNUNG_HOECHSTENS {
        aufbau
            .aktivitaet
            .setzen(&anna(), herkunft(), &gespraech(n), Zustand::Tippt, None)
            .unwrap_or_else(|abweisung| panic!("{n}: {abweisung:?}"));
    }
    let vorher = aufbau.gemeldet().len();

    let siebzehntes =
        aufbau
            .aktivitaet
            .setzen(&anna(), herkunft(), &gespraech(16), Zustand::Tippt, None);
    let erneuert =
        aufbau
            .aktivitaet
            .setzen(&anna(), herkunft(), &gespraech(3), Zustand::Tippt, None);
    let andere_kennung =
        aufbau
            .aktivitaet
            .setzen(&ben(), herkunft(), &gespraech(16), Zustand::Tippt, None);

    assert_eq!(siebzehntes, Err(Abweisung::ZuViele));
    assert!(erneuert.is_ok() && andere_kennung.is_ok());
    assert_eq!(aufbau.gemeldet().len(), vorher + 2, "das 17. meldet nichts");
    assert!(
        aufbau
            .aktivitaet
            .lesen(&gespraech(16), &ben())
            .aktivitaeten
            .is_empty()
    );
    aufbau.um(6_000);
    assert!(
        aufbau
            .aktivitaet
            .setzen(&anna(), herkunft(), &gespraech(16), Zustand::Tippt, None)
            .is_ok(),
        "abgelaufene zählen nicht"
    );
}

/// `beenden` meldet ein Ende nur, wenn vorher etwas galt (Regel 5); `beenden_alle` beendet jedes Gespräch der
/// Kennung, andere Kennungen bleiben.
#[test]
fn beenden_meldet_nur_ein_ende_wenn_etwas_galt() {
    let aufbau = Aufbau::neu();
    let technik = Gespraech::channel("technik");
    aufbau.setzen(&anna(), Zustand::Tippt, None).expect("anna");
    aufbau
        .aktivitaet
        .setzen(&anna(), herkunft(), &technik, Zustand::Tippt, None)
        .expect("anna technik");
    aufbau
        .setzen(&claude(), Zustand::DenktNach, None)
        .expect("claude");
    let vorher = aufbau.gemeldet().len();

    aufbau.aktivitaet.beenden(&ben(), &allgemein());
    aufbau.aktivitaet.beenden(&anna(), &allgemein());
    aufbau.aktivitaet.beenden(&anna(), &allgemein());

    let ende = aufbau.gemeldet()[vorher..].to_vec();
    assert_eq!(
        ende,
        [(
            Aenderung {
                gespraech: allgemein(),
                kennung: anna(),
                aktiv: None
            },
            true
        )]
    );
    assert_eq!(aufbau.liest_ben(), ["claude@10.0.0.5"]);
    assert_eq!(
        aufbau.aktivitaet.lesen(&technik, &ben()).aktivitaeten.len(),
        1,
        "das Ende gilt nur im Gespräch"
    );

    aufbau.aktivitaet.beenden_alle(&claude());
    aufbau.um(6_000).aktivitaet.beenden_alle(&anna());

    let alle = aufbau.gemeldet()[vorher + 1..].to_vec();
    assert_eq!(
        alle,
        [(
            Aenderung {
                gespraech: allgemein(),
                kennung: claude(),
                aktiv: None
            },
            true
        )],
        "abgelaufen (anna in technik bei t0 + 6 s) meldet kein Ende"
    );
    assert!(
        aufbau.um(0).liest_ben().is_empty(),
        "auch nach Rückstellen weg"
    );
}

/// Lesen nennt nur gültige Einträge des Gesprächs, nach Kennung sortiert, ohne den Lesenden (Mutante M2).
#[test]
fn lesen_sortiert_ohne_den_lesenden() {
    let aufbau = Aufbau::neu();
    for wer in [claude(), ben(), anna()] {
        aufbau.setzen(&wer, Zustand::Tippt, None).expect("gesetzt");
    }
    aufbau
        .aktivitaet
        .setzen(
            &kennung("dora@10.0.0.9"),
            herkunft(),
            &Gespraech::channel("technik"),
            Zustand::Tippt,
            None,
        )
        .expect("dora");

    let gelesen = aufbau.um(10).aktivitaet.lesen(&allgemein(), &ben());

    assert_eq!(gelesen.jetzt, t(10));
    assert_eq!(
        gelesen
            .aktivitaeten
            .iter()
            .map(|a| a.kennung.als_text())
            .collect::<Vec<_>>(),
        ["anna@10.0.0.7", "claude@10.0.0.5"]
    );
}

#[test]
fn zustand_nur_mit_festem_wortlaut() {
    assert_eq!(Zustand::lesen("tippt"), Some(Zustand::Tippt));
    assert_eq!(Zustand::lesen("denkt_nach"), Some(Zustand::DenktNach));
    for falsch in ["schreibt", "denkt nach", "Tippt", "", " tippt"] {
        assert_eq!(Zustand::lesen(falsch), None, "{falsch:?}");
    }
    assert_eq!(Zustand::DenktNach.als_text(), "denkt_nach");
}

/// Ein abgelaufener Zustand erzeugt beim Beenden kein Ende-Signal (Regel 5: „nur, wenn vorher etwas aktiv und nicht
/// abgelaufen war“).
#[test]
fn beenden_nach_ablauf_meldet_kein_ende() {
    let aufbau = Aufbau::neu();
    aufbau.setzen(&anna(), Zustand::Tippt, None).expect("anna");
    let vorher = aufbau.gemeldet().len();

    aufbau.um(6_000).aktivitaet.beenden(&anna(), &allgemein());

    assert_eq!(aufbau.gemeldet().len(), vorher);
}

/// B0686 Sichtbarkeitsregel (Mutante M11), bis I0012 und I0022 gebaut sind: Channel für jeden, Direktgespräch nur für
/// die Beteiligten (genaue Kennung, kein Teilstück), archiviert für niemanden.
#[test]
fn sichtbar_channel_fuer_jeden_direkt_nur_fuer_beteiligte_archiviert_nie() {
    let direkt = Gespraech::Direkt("anna@10.0.0.7+ben@10.0.0.8".into());
    let fall = |gespraech: &Gespraech, wer: &str, archiviert: bool| {
        sichtbar(gespraech, &kennung(wer), archiviert)
    };

    assert!(fall(&allgemein(), "cora@10.0.0.9", false));
    assert!(fall(&direkt, "anna@10.0.0.7", false));
    assert!(fall(&direkt, "ben@10.0.0.8", false));
    assert!(!fall(&direkt, "cora@10.0.0.9", false));
    assert!(!fall(&direkt, "anna@10.0.0.70", false), "kein Teilstück");
    assert!(!fall(&direkt, "nna@10.0.0.7", false), "kein Teilstück");
    assert!(!fall(&allgemein(), "anna@10.0.0.7", true));
    assert!(!fall(&direkt, "anna@10.0.0.7", true));
}

/// B0692 P3 Bremse (gestellte Uhr, Mutante M9): 10 × Setzen `(anna, allgemein)` bei t0 gehen, das 11. ist
/// `Gebremst{gespraech, 1.000 ms}` ohne Wirkung; bei t0 + 999 ms noch gebremst, bei t0 + 1.000 ms wieder frei; 10 ×
/// in `# technik` danach gehen (eigener Eimer je Gespräch). `beenden` und `lesen` sind nie gebremst.
#[test]
fn p3_bremse_je_kennung_und_gespraech_zehn_dann_eine_je_sekunde() {
    let aufbau = Aufbau::neu();
    let technik = Gespraech::channel("technik");
    for n in 0..10 {
        aufbau
            .setzen(&anna(), Zustand::Tippt, None)
            .unwrap_or_else(|abweisung| panic!("{n}: {abweisung:?}"));
    }
    let vorher = aufbau.gemeldet().len();

    let elftes = aufbau.setzen(&anna(), Zustand::DenktNach, None);
    let nach_999 = aufbau.um(999).setzen(&anna(), Zustand::DenktNach, None);
    let gemeldet_gebremst = aufbau.gemeldet().len();
    let gelesen_gebremst = aufbau.liest_ben();
    aufbau.aktivitaet.beenden(&anna(), &allgemein());
    let beendet = aufbau.gemeldet().len();
    let nach_1000 = aufbau.um(1_000).setzen(&anna(), Zustand::Tippt, None);
    let in_technik: Vec<_> = (0..10)
        .map(|_| {
            aufbau
                .aktivitaet
                .setzen(&anna(), herkunft(), &technik, Zustand::Tippt, None)
        })
        .collect();

    let gebremst = |ms| {
        Err(Abweisung::Gebremst(Gebremst {
            grenze: Grenze::Gespraech,
            wieder_in: std::time::Duration::from_millis(ms),
        }))
    };
    assert_eq!(elftes, gebremst(1_000));
    assert_eq!(nach_999, gebremst(1));
    assert_eq!(gemeldet_gebremst, vorher, "gebremst meldet nichts");
    assert_eq!(gelesen_gebremst, ["anna@10.0.0.7"], "Lesen nie gebremst");
    assert_eq!(beendet, vorher + 1, "Beenden nie gebremst");
    assert_eq!(nach_1000.map(|g| g.bis), Ok(t(7_000)));
    assert!(in_technik.iter().all(Result::is_ok), "{in_technik:?}");
}

/// B0692 P3 Herkunft (Mutante M9 „Eimer je Herkunft aus“): 120 Setzen über 12 Kennungen × 10 Gespräche einer
/// Adresse gehen, das 121. — eine 13. Kennung in einem frischen Gespräch — ist `Gebremst{herkunft}`; ein Setzen von
/// einer anderen Adresse geht. Volle Eimer werden nicht gehalten.
#[test]
fn p3_bremse_je_herkunft_hundertzwanzig_dann_zwoelf_je_sekunde() {
    let aufbau = Aufbau::neu();
    let wer = |n: usize| kennung(&format!("k{n}@10.0.0.1"));
    let gespraech = |n: usize| Gespraech::channel(&format!("c{n}"));
    for k in 0..12 {
        for g in 0..10 {
            aufbau
                .aktivitaet
                .setzen(&wer(k), herkunft(), &gespraech(g), Zustand::Tippt, None)
                .unwrap_or_else(|abweisung| panic!("{k}/{g}: {abweisung:?}"));
        }
    }

    let einhunderteinundzwanzigstes =
        aufbau
            .aktivitaet
            .setzen(&wer(12), herkunft(), &gespraech(0), Zustand::Tippt, None);
    let andere_adresse = aufbau.aktivitaet.setzen(
        &kennung("zoe@10.0.0.2"),
        "10.0.0.2".parse().expect("IP"),
        &gespraech(0),
        Zustand::Tippt,
        None,
    );
    let nach_84 =
        aufbau
            .um(84)
            .aktivitaet
            .setzen(&wer(12), herkunft(), &gespraech(0), Zustand::Tippt, None);

    assert_eq!(
        einhunderteinundzwanzigstes,
        Err(Abweisung::Gebremst(Gebremst {
            grenze: Grenze::Herkunft,
            wieder_in: std::time::Duration::from_millis(84),
        }))
    );
    assert!(andere_adresse.is_ok(), "{andere_adresse:?}");
    assert!(nach_84.is_ok(), "1 / 12 s ≈ 84 ms: {nach_84:?}");
}
