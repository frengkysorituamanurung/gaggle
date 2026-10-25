use std::collections::HashMap;
use std::fmt;

/// Supported locales for the Gaggle UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    En,
    Es,
    Ja,
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Locale::En => write!(f, "en"),
            Locale::Es => write!(f, "es"),
            Locale::Ja => write!(f, "ja"),
        }
    }
}

/// Minimal internationalization container holding the current locale and
/// a flat map of message keys to localized strings.
#[derive(Debug, Clone)]
pub struct I18n {
    locale: Locale,
    messages: HashMap<String, String>,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            messages: HashMap::new(),
        }
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn t(&self, key: &str) -> String {
        self.messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.messages.insert(key.into(), value.into());
    }
}

/// Load a built-in locale with a small set of translated messages.
pub fn load_locale(locale: Locale) -> I18n {
    let mut i18n = I18n::new(locale);
    match locale {
        Locale::En => {
            i18n.insert("app.title", "Gaggle");
            i18n.insert("health.ok", "OK");
        }
        Locale::Es => {
            i18n.insert("app.title", "Gaggle");
            i18n.insert("health.ok", "OK");
        }
        Locale::Ja => {
            i18n.insert("app.title", "Gaggle");
            i18n.insert("health.ok", "OK");
        }
    }
    i18n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_is_en() {
        assert_eq!(Locale::default(), Locale::En);
    }

    #[test]
    fn t_returns_key_when_missing() {
        let i18n = I18n::new(Locale::En);
        assert_eq!(i18n.t("missing.key"), "missing.key");
    }

    #[test]
    fn load_locale_has_builtin_keys() {
        let i18n = load_locale(Locale::En);
        assert_eq!(i18n.t("app.title"), "Gaggle");
        assert_eq!(i18n.t("health.ok"), "OK");
    }

    #[test]
    fn set_locale_updates_locale() {
        let mut i18n = I18n::new(Locale::En);
        i18n.set_locale(Locale::Ja);
        assert_eq!(i18n.locale(), Locale::Ja);
    }
}
