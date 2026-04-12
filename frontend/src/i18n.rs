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
    ]
}
