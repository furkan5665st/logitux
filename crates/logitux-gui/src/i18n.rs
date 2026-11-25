use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    Cs,
    Da,
    De,
    En,
    Es,
    Fi,
    Fr,
    Hu,
    It,
    Ja,
    Ko,
    Nb,
    Nl,
    Pl,
    PtBr,
    Ru,
    Sv,
    Tr,
    ZhCn,
    ZhTw,
}

impl Default for Lang {
    fn default() -> Self {
        Self::detect()
    }
}

impl Lang {
    pub fn all() -> &'static [Lang] {
        &[
            Lang::Cs,
            Lang::Da,
            Lang::De,
            Lang::En,
            Lang::Es,
            Lang::Fi,
            Lang::Fr,
            Lang::Hu,
            Lang::It,
            Lang::Ja,
            Lang::Ko,
            Lang::Nb,
            Lang::Nl,
            Lang::Pl,
            Lang::PtBr,
            Lang::Ru,
            Lang::Sv,
            Lang::Tr,
            Lang::ZhCn,
            Lang::ZhTw,
        ]
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Cs => "cs-CZ",
            Self::Da => "da-DK",
            Self::De => "de-DE",
            Self::En => "en-US",
            Self::Es => "es-ES",
            Self::Fi => "fi-FI",
            Self::Fr => "fr-FR",
            Self::Hu => "hu-HU",
            Self::It => "it-IT",
            Self::Ja => "ja-JP",
            Self::Ko => "ko-KR",
            Self::Nb => "nb-NO",
            Self::Nl => "nl-NL",
            Self::Pl => "pl-PL",
            Self::PtBr => "pt-BR",
            Self::Ru => "ru-RU",
            Self::Sv => "sv-SE",
            Self::Tr => "tr-TR",
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Self::Cs => "Čeština",
            Self::Da => "Dansk",
            Self::De => "Deutsch",
            Self::En => "English",
            Self::Es => "Español",
            Self::Fi => "Suomi",
            Self::Fr => "Français",
            Self::Hu => "Magyar",
            Self::It => "Italiano",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Nb => "Norsk Bokmål",
            Self::Nl => "Nederlands",
            Self::Pl => "Polski",
            Self::PtBr => "Português (Brasil)",
            Self::Ru => "Русский",
            Self::Sv => "Svenska",
            Self::Tr => "Türkçe",
            Self::ZhCn => "简体中文",
            Self::ZhTw => "繁體中文",
        }
    }

    pub fn from_code(raw: &str) -> Self {
        let lower = raw.trim().replace('_', "-").to_lowercase();
        let base = lower.split(['.', '@']).next().unwrap_or("");
        let primary = base.split('-').next().unwrap_or("");
        match primary {
            "cs" => Self::Cs,
            "da" => Self::Da,
            "de" => Self::De,
            "en" => Self::En,
            "es" => Self::Es,
            "fi" => Self::Fi,
            "fr" => Self::Fr,
            "hu" => Self::Hu,
            "it" => Self::It,
            "ja" => Self::Ja,
            "ko" => Self::Ko,
            "nb" | "no" | "nn" => Self::Nb,
            "nl" => Self::Nl,
            "pl" => Self::Pl,
            "pt" => Self::PtBr,
            "ru" => Self::Ru,
            "sv" => Self::Sv,
            "tr" => Self::Tr,
            "zh" => {
                if lower.contains("tw") || lower.contains("hk") || lower.contains("hant") {
                    Self::ZhTw
                } else {
                    Self::ZhCn
                }
            }
            _ => Self::En,
        }
    }

    #[cfg(windows)]
    pub fn detect() -> Self {
        extern "system" {
            fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
        }
        let mut buf = [0u16; 85];
        let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
        if n > 0 {
            let text = String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)]);
            return Self::from_code(&text);
        }
        Self::En
    }

    #[cfg(not(windows))]
    pub fn detect() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                if !value.is_empty() {
                    return Self::from_code(&value);
                }
            }
        }
        Self::En
    }

    pub fn pick<'a>(self, tr: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Tr => tr,
            Self::En => en,
            other => crate::i18n_data::translate(other.code(), en).unwrap_or(en),
        }
    }
}
