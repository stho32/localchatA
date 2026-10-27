//! Freien Namen vorschlagen (B0047): `<name>-2`, `-3`, … — der erste, den niemand trägt, höchstens 32 Zeichen.

use crate::name::Name;

/// Endet immer: es gibt höchstens so viele belegte Namen wie Sitzungen, aber beliebig viele Zähler.
pub fn freier_name(name: &Name, belegt: impl Fn(&Name) -> bool) -> Name {
    let mut zaehler = 2u32;
    loop {
        let endung = format!("-{zaehler}");
        let stamm: String = name.als_text().chars().take(32 - endung.len()).collect();
        let kandidat = format!("{}{endung}", stamm.trim_end_matches('-'));
        if let Ok(kandidat) = Name::bilden(&kandidat)
            && !belegt(&kandidat)
        {
            return kandidat;
        }
        zaehler += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> Name {
        Name::bilden(text).expect("gültiger Name")
    }

    fn vorschlag(fuer: &str, belegt: &[&str]) -> String {
        freier_name(&name(fuer), |kandidat| {
            belegt.contains(&kandidat.als_text())
        })
        .to_string()
    }

    #[test]
    fn der_erste_freie_name_mit_zaehler_ab_zwei() {
        assert_eq!(vorschlag("claude", &["claude"]), "claude-2");
        assert_eq!(vorschlag("claude", &["claude", "claude-2"]), "claude-3");
        assert_eq!(vorschlag("claude", &["claude", "claude-3"]), "claude-2");
        assert_eq!(vorschlag("claude-2", &["claude-2"]), "claude-2-2");
    }

    #[test]
    fn an_der_laengengrenze_wird_der_stamm_gekuerzt() {
        let a32 = "a".repeat(32);
        assert_eq!(vorschlag(&a32, &[&a32]), format!("{}-2", "a".repeat(30)));
        let belegt_bis_neun: Vec<String> =
            (2..=9).map(|n| format!("{}-{n}", "a".repeat(30))).collect();
        let mut belegt: Vec<&str> = belegt_bis_neun.iter().map(String::as_str).collect();
        belegt.push(&a32);
        assert_eq!(vorschlag(&a32, &belegt), format!("{}-10", "a".repeat(29)));
    }

    #[test]
    fn ein_gekuerzter_stamm_endet_nie_mit_bindestrich() {
        let stamm = format!("{}-b", "a".repeat(29));
        assert_eq!(
            vorschlag(&stamm, &[&stamm]),
            format!("{}-2", "a".repeat(29))
        );
    }
}
