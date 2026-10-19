/// Quality tier / source (features/player-generation.md sec 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tier {
    WalkOn,
    LocalAmateur,
    Regular,
    ExCollege,
    ExPro,
}

impl Tier {
    pub const ALL: [Tier; 5] = [Tier::WalkOn, Tier::LocalAmateur, Tier::Regular, Tier::ExCollege, Tier::ExPro];

    pub fn q_shift(self) -> f64 {
        match self {
            Tier::WalkOn => -12.0,
            Tier::LocalAmateur => -6.0,
            Tier::Regular => 0.0,
            Tier::ExCollege => 6.0,
            Tier::ExPro => 10.0,
        }
    }

    pub fn sd_mult(self) -> f64 {
        match self {
            Tier::WalkOn => 1.4,
            Tier::LocalAmateur => 1.1,
            _ => 1.0,
        }
    }
}
