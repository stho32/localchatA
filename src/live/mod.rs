//! Live-Kanal (A00011): Ereignisse an alle offenen Ströme; ab I0005 die Anwesenheit.

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Brücke bis B0083: Takt und Beitreten veröffentlichen"
    )
)]
pub mod bus;
pub mod herzschlag;
