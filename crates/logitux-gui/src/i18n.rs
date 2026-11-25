use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Tr,
    En,
}

impl Lang {
    pub fn pick<'a>(self, tr: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Tr => tr,
            Self::En => en,
        }
    }
}
