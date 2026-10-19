/// First and last names for generated players, built from plain one-column CSVs (a header row, no weights).
/// Every name is equally likely.
#[derive(Clone, Debug)]
pub struct NameBank {
    pub first: Vec<String>,
    pub last: Vec<String>,
}

const FIRST_CSV: &str = include_str!("../../../data/first_names.csv");
const LAST_CSV: &str = include_str!("../../../data/last_names.csv");

impl NameBank {
    pub fn new(first: Vec<String>, last: Vec<String>) -> Self {
        assert!(!first.is_empty() && !last.is_empty(), "Name lists cannot be empty");
        Self { first, last }
    }

    /// One name per line; the first line is a header and is skipped. Blank lines, quotes, CRLF and duplicates are dropped.
    pub fn parse_column(csv: &str) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for raw in csv.replace('\r', "").split('\n').skip(1) {
            let name = raw.split(',').next().unwrap_or("").replace('"', "");
            let name = name.trim();
            if !name.is_empty() && seen.insert(name.to_string()) {
                out.push(name.to_string());
            }
        }
        out
    }

    pub fn from_csv(first_csv: &str, last_csv: &str) -> Self {
        Self::new(Self::parse_column(first_csv), Self::parse_column(last_csv))
    }

    /// The full bundled lists (the same CSVs the Flutter app shipped).
    pub fn bundled() -> Self {
        Self::from_csv(FIRST_CSV, LAST_CSV)
    }

    /// A tiny list for tests.
    pub fn built_in() -> Self {
        let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect();
        Self::new(
            s(&["Cody", "Marcus", "Tyler", "Dale", "Luis", "Jamal", "Ricky", "Hector", "Brent", "Darnell", "Travis", "Mateo", "Gus", "Wade", "Tomas", "Eli"]),
            s(&["Ramirez", "Johnson", "Nguyen", "Walker", "Garcia", "Pickett", "Okafor", "Lindqvist", "Hayes", "Delgado", "Mooney", "Tran", "Brooks", "Castillo", "Bledsoe", "Kowalski"]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_quotes_crlf_and_duplicates() {
        let v = NameBank::parse_column("name\r\n\"Cody\"\r\nCody\r\n\r\nDale,5\r\n");
        assert_eq!(v, ["Cody", "Dale"]);
    }

    #[test]
    fn bundled_lists_are_big() {
        let b = NameBank::bundled();
        assert!(b.first.len() > 300, "{}", b.first.len());
        assert!(b.last.len() > 300, "{}", b.last.len());
    }
}
