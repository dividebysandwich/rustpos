use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct I18n {
    translations: HashMap<String, String>,
    pub lang: String,
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        let json_str = match lang {
            "es" => include_str!("../locales/es.json"),
            "de" => include_str!("../locales/de.json"),
            "fr" => include_str!("../locales/fr.json"),
            "pt" => include_str!("../locales/pt.json"),
            "hi" => include_str!("../locales/hi.json"),
            "cs" => include_str!("../locales/cs.json"),
            "it" => include_str!("../locales/it.json"),
            "hu" => include_str!("../locales/hu.json"),
            "pl" => include_str!("../locales/pl.json"),
            "ro" => include_str!("../locales/ro.json"),
            "uk" => include_str!("../locales/uk.json"),
            "sw" => include_str!("../locales/sw.json"),
            "am" => include_str!("../locales/am.json"),
            "ha" => include_str!("../locales/ha.json"),
            "yo" => include_str!("../locales/yo.json"),
            "af" => include_str!("../locales/af.json"),
            "ar" => include_str!("../locales/ar.json"),
            _ => include_str!("../locales/en.json"),
        };
        let translations: HashMap<String, String> =
            serde_json::from_str(json_str).unwrap_or_default();
        Self {
            translations,
            lang: lang.to_string(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

/// Preset currencies: (symbol, label).
/// The list is based on the supported localization regions.
pub fn available_currencies() -> Vec<(&'static str, &'static str)> {
    vec![
        ("\u{20ac}", "EUR \u{20ac}"),   // Euro
        ("$", "USD $"),                  // US Dollar
        ("\u{00a3}", "GBP \u{00a3}"),   // British Pound
        ("R$", "BRL R$"),               // Brazilian Real
        ("\u{20b9}", "INR \u{20b9}"),   // Indian Rupee
        ("K\u{10d}", "CZK K\u{10d}"),  // Czech Koruna
        ("Ft", "HUF Ft"),              // Hungarian Forint
        ("z\u{142}", "PLN z\u{142}"),  // Polish Zloty
        ("lei", "RON lei"),             // Romanian Leu
        ("\u{20b4}", "UAH \u{20b4}"),  // Ukrainian Hryvnia
        ("CHF", "CHF"),                 // Swiss Franc
        // South American
        ("AR$", "ARS AR$"),             // Argentine Peso
        ("CL$", "CLP CL$"),            // Chilean Peso
        ("CO$", "COP CO$"),            // Colombian Peso
        ("S/", "PEN S/"),              // Peruvian Sol
        // African
        ("KSh", "KES KSh"),            // Kenyan Shilling
        ("\u{20a6}", "NGN \u{20a6}"),  // Nigerian Naira
        ("R", "ZAR R"),                 // South African Rand
        ("Br", "ETB Br"),              // Ethiopian Birr
        ("GH\u{20b5}", "GHS GH\u{20b5}"), // Ghanaian Cedi
        ("CFA", "XOF CFA"),            // West African CFA Franc
        ("E\u{00a3}", "EGP E\u{00a3}"), // Egyptian Pound
    ]
}

/// Return the default currency symbol for a given language code.
pub fn default_currency_for_language(lang: &str) -> &'static str {
    match lang {
        "en" => "$",
        "es" => "\u{20ac}",
        "de" => "\u{20ac}",
        "fr" => "\u{20ac}",
        "pt" => "R$",
        "hi" => "\u{20b9}",
        "cs" => "K\u{10d}",
        "it" => "\u{20ac}",
        "hu" => "Ft",
        "pl" => "z\u{142}",
        "ro" => "lei",
        "uk" => "\u{20b4}",
        "sw" => "KSh",
        "am" => "Br",
        "ha" => "\u{20a6}",
        "yo" => "\u{20a6}",
        "af" => "R",
        "ar" => "E\u{00a3}",
        _ => "\u{20ac}",
    }
}

pub fn available_languages() -> Vec<(&'static str, &'static str)> {
    vec![
        ("en", "English"),
        ("es", "Espa\u{f1}ol"),
        ("de", "Deutsch"),
        ("fr", "Fran\u{e7}ais"),
        ("pt", "Portugu\u{ea}s"),
        ("hi", "\u{939}\u{93f}\u{928}\u{94d}\u{926}\u{940}"),
        ("cs", "\u{10c}e\u{161}tina"),
        ("it", "Italiano"),
        ("hu", "Magyar"),
        ("pl", "Polski"),
        ("ro", "Rom\u{e2}n\u{103}"),
        ("uk", "\u{423}\u{43a}\u{440}\u{430}\u{457}\u{43d}\u{441}\u{44c}\u{43a}\u{430}"),
        ("sw", "Kiswahili"),
        ("am", "\u{12a0}\u{121b}\u{122d}\u{129b}"),
        ("ha", "Hausa"),
        ("yo", "Yor\u{f9}b\u{e1}"),
        ("af", "Afrikaans"),
        ("ar", "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}"),
    ]
}
