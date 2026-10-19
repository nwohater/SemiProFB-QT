//! The humor text tables (inbox, gazette, conduct, surprise bills), extracted from the Dart source into `data/*.json`
//! by `tools/extract_league_text.py`. Pick with a seeded `Rng`, fill `{placeholders}` with [`fill`].

use serde::Deserialize;
use sp_core::Rng;
use std::collections::HashMap;
use std::sync::OnceLock;

const INBOX_JSON: &str = include_str!("../../../data/inbox_text.json");
const GAZETTE_JSON: &str = include_str!("../../../data/gazette_text.json");
const CONDUCT_JSON: &str = include_str!("../../../data/conduct_text.json");
const BILLS_JSON: &str = include_str!("../../../data/surprise_bills.json");

/// Replaces `{name}` with its value; a placeholder with no value is left as written.
pub fn fill(t: &str, vars: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(t.len() + 16);
    let mut rest = t;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if after[..close].chars().all(|c| c.is_alphanumeric() || c == '_') && close > 0 => {
                let key = &after[..close];
                match vars.iter().find(|(k, _)| *k == key) {
                    Some((_, v)) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whole dollars with thousands separators: 12345 -> "12,345".
pub fn commas(v: i64) -> String {
    let neg = v < 0;
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg { format!("-{out}") } else { out }
}

/// "$12,345", or "-$12,345".
pub fn money(v: i64) -> String {
    if v < 0 { format!("-${}", commas(-v)) } else { format!("${}", commas(v)) }
}

#[derive(Deserialize)]
pub struct JobCategory {
    pub keys: Vec<String>,
    pub injury: Vec<String>,
    pub trip: Vec<String>,
}

pub struct TextTables {
    lists: HashMap<String, Vec<String>>,
    pub job_categories: Vec<(String, JobCategory)>,
    pub sponsor_clause_events: HashMap<String, Vec<String>>,
}

impl TextTables {
    /// One table by its Dart name, e.g. `"reportWinBig"`. Panics on a typo (tables are fixed data).
    pub fn list(&self, name: &str) -> &[String] {
        self.lists.get(name).unwrap_or_else(|| panic!("no text table {name}"))
    }

    pub fn pick<'a>(&'a self, name: &str, rng: &mut Rng) -> &'a str {
        rng.pick(self.list(name))
    }
}

fn load(json: &str, keep_keys_order: bool) -> TextTables {
    let v: serde_json::Value = serde_json::from_str(json).expect("text json");
    let obj = v.as_object().expect("object");
    let mut lists = HashMap::new();
    let mut job_categories = Vec::new();
    let mut clause_events = HashMap::new();
    for (k, val) in obj {
        match (k.as_str(), val) {
            ("jobCategories", serde_json::Value::Object(m)) => {
                for (cat, c) in m {
                    job_categories.push((cat.clone(), serde_json::from_value::<JobCategory>(c.clone()).expect("job category")));
                }
            }
            ("sponsorClauseEvents", serde_json::Value::Object(m)) => {
                for (id, l) in m {
                    clause_events.insert(id.clone(), serde_json::from_value::<Vec<String>>(l.clone()).expect("clause events"));
                }
            }
            (_, serde_json::Value::Array(a)) if a.iter().all(|x| x.is_string()) => {
                lists.insert(k.clone(), a.iter().map(|x| x.as_str().unwrap().to_string()).collect());
            }
            _ => {}
        }
    }
    let _ = keep_keys_order;
    TextTables { lists, job_categories, sponsor_clause_events: clause_events }
}

/// Inbox letters, council letters, rumors, sponsor events, draft notes, job-keyed injury lines.
pub fn inbox_text() -> &'static TextTables {
    static T: OnceLock<TextTables> = OnceLock::new();
    T.get_or_init(|| load(INBOX_JSON, true))
}

/// Gazette headlines and copy.
pub fn gazette_text() -> &'static TextTables {
    static T: OnceLock<TextTables> = OnceLock::new();
    T.get_or_init(|| load(GAZETTE_JSON, true))
}

#[derive(Clone, Debug, Deserialize)]
pub struct Incident {
    pub tier: i32,
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
struct ConductData {
    asks: Vec<String>,
    incidents: Vec<Incident>,
}

fn conduct_data() -> &'static ConductData {
    static T: OnceLock<ConductData> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(CONDUCT_JSON).expect("conduct_text.json"))
}

pub fn conduct_asks() -> &'static [String] {
    &conduct_data().asks
}

pub fn conduct_incidents() -> &'static [Incident] {
    &conduct_data().incidents
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bill {
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
struct BillsData {
    welfare: Vec<Bill>,
    bills: Vec<Bill>,
}

fn bills_data() -> &'static BillsData {
    static T: OnceLock<BillsData> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(BILLS_JSON).expect("surprise_bills.json"))
}

pub fn welfare_bills() -> &'static [Bill] {
    &bills_data().welfare
}

pub fn surprise_bills() -> &'static [Bill] {
    &bills_data().bills
}

/// True when `text` was written from a `fanFavoriteLocal` template (a local hero who is a free agent).
pub fn is_local_hero(text: &str) -> bool {
    local_hero_name(text).is_some()
}

/// The player name filled into a `fanFavoriteLocal` template, or `None` when `text` is not one of those.
pub fn local_hero_name(text: &str) -> Option<String> {
    static PATTERNS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    let pats = PATTERNS.get_or_init(|| {
        let ph = regex::Regex::new(r"\{(\w+)\}").unwrap();
        inbox_text()
            .list("fanFavoriteLocal")
            .iter()
            .map(|t| {
                let mut b = String::from("^");
                let mut start = 0;
                for m in ph.captures_iter(t) {
                    let whole = m.get(0).unwrap();
                    b.push_str(&regex::escape(&t[start..whole.start()]));
                    b.push_str(if &m[1] == "p" { "(.+?)" } else { ".+?" });
                    start = whole.end();
                }
                b.push_str(&regex::escape(&t[start..]));
                b.push('$');
                regex::Regex::new(&b).unwrap()
            })
            .collect()
    });
    for r in pats {
        if let Some(c) = r.captures(text) {
            if let Some(n) = c.get(1) {
                if !n.as_str().is_empty() {
                    return Some(n.as_str().to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_known_and_keeps_unknown() {
        assert_eq!(fill("Hi {p}, {x} {p}!", &[("p", "Dale".into())]), "Hi Dale, {x} Dale!");
        assert_eq!(fill("no braces", &[]), "no braces");
        assert_eq!(fill("a { b } c", &[]), "a { b } c");
    }

    #[test]
    fn money_formats() {
        assert_eq!(commas(1234567), "1,234,567");
        assert_eq!(commas(12), "12");
        assert_eq!(money(-4500), "-$4,500");
        assert_eq!(money(900), "$900");
    }

    #[test]
    fn tables_load_with_the_expected_sizes() {
        assert_eq!(inbox_text().list("reportWinBig").len(), 12);
        assert_eq!(inbox_text().job_categories.len(), 10);
        assert_eq!(inbox_text().sponsor_clause_events.len(), 27);
        assert_eq!(gazette_text().list("spotHead").len(), 2);
        assert_eq!(conduct_incidents().len(), 75);
        assert_eq!(conduct_asks().len(), 4);
        assert_eq!(surprise_bills().len(), 43);
        assert_eq!(welfare_bills().len(), 4);
        assert!(conduct_incidents().iter().all(|i| (0..=2).contains(&i.tier)));
    }

    #[test]
    fn local_hero_lines_round_trip() {
        let t = inbox_text().list("fanFavoriteLocal")[0].clone();
        let line = fill(&t, &[("p", "Dale Brooks".into()), ("pos", "QB".into()), ("age", "21".into()), ("town", "Fresno".into())]);
        assert!(is_local_hero(&line));
        assert_eq!(local_hero_name(&line).as_deref(), Some("Dale Brooks"));
        assert!(!is_local_hero("A completely different sentence."));
    }
}
