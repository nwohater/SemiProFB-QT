use crate::enums::Attr;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Index;

/// A full set of the 8 attributes, indexed by [`Attr`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ratings([i32; 8]);

impl Ratings {
    pub fn new(values: [i32; 8]) -> Self {
        Self(values)
    }

    pub fn from_slice(values: &[i32]) -> Self {
        let mut a = [0; 8];
        a.copy_from_slice(values);
        Self(a)
    }

    pub fn to_array(self) -> [i32; 8] {
        self.0
    }
}

impl Index<Attr> for Ratings {
    type Output = i32;
    fn index(&self, a: Attr) -> &i32 {
        &self.0[a.index()]
    }
}

impl fmt::Display for Ratings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = Attr::ALL.iter().map(|&a| format!("{}{}", a.code(), self[a])).collect();
        write!(f, "{}", parts.join(" "))
    }
}
