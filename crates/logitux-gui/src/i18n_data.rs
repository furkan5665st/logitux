use std::collections::HashMap;
use std::sync::OnceLock;

type Table = HashMap<String, HashMap<String, String>>;

static TABLE: OnceLock<Table> = OnceLock::new();

fn table() -> &'static Table {
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("i18n_data.json")).unwrap_or_default()
    })
}

pub fn translate(locale: &str, english: &str) -> Option<&'static str> {
    table()
        .get(locale)
        .and_then(|section| section.get(english))
        .map(String::as_str)
}
