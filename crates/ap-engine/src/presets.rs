//! Named rules: the Tray's six cartridges (SPEC TRAY-4), the Terrarium's names map and starting
//! jars (TERR-1), and the Tray's starting garden (TRAY-7).

use crate::grid::{Anchor, Grid};
use crate::pattern::library;
use crate::rule::{Boundary, LifeRule, Neighbourhood};

/// A rule cartridge in the Tray's Rules list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cartridge {
    /// Stable id (`life`, `high`, `seeds`, `dn`, `maze`, `coral`), as in the mockup.
    pub id: &'static str,
    /// Display name ("Life").
    pub name: &'static str,
    /// The one-line note ("The classic").
    pub note: &'static str,
    /// The rule (Moore r1).
    pub rule: LifeRule,
}

/// The six cartridges in the Tray's order.
pub const CARTRIDGES: [Cartridge; 6] = [
    Cartridge {
        id: "life",
        name: "Life",
        note: "The classic",
        rule: LifeRule::moore(&[3], &[2, 3]),
    },
    Cartridge {
        id: "high",
        name: "HighLife",
        note: "Has a replicator",
        rule: LifeRule::moore(&[3, 6], &[2, 3]),
    },
    Cartridge {
        id: "seeds",
        name: "Seeds",
        note: "Explodes",
        rule: LifeRule::moore(&[2], &[]),
    },
    Cartridge {
        id: "dn",
        name: "Day & Night",
        note: "Light–dark symmetric",
        rule: LifeRule::moore(&[3, 6, 7, 8], &[3, 4, 6, 7, 8]),
    },
    Cartridge {
        id: "maze",
        name: "Maze",
        note: "Grows corridors",
        rule: LifeRule::moore(&[3], &[1, 2, 3, 4, 5]),
    },
    Cartridge {
        id: "coral",
        name: "Coral",
        note: "Slow reef",
        rule: LifeRule::moore(&[3], &[4, 5, 6, 7, 8]),
    },
];

/// The cartridge with this id.
pub fn cartridge(id: &str) -> Option<&'static Cartridge> {
    CARTRIDGES.iter().find(|c| c.id == id)
}

/// The cartridge whose rule equals `rule` (the highlighted one), or `None` for a custom rule.
pub fn matching_cartridge(rule: &LifeRule) -> Option<&'static Cartridge> {
    CARTRIDGES.iter().find(|c| c.rule == *rule)
}

/// The Terrarium's names map (Moore r1 rules), in the mockup's order.
pub const NAMED: [(&str, LifeRule); 8] = [
    ("Life", LifeRule::moore(&[3], &[2, 3])),
    ("HighLife", LifeRule::moore(&[3, 6], &[2, 3])),
    (
        "Day & Night",
        LifeRule::moore(&[3, 6, 7, 8], &[3, 4, 6, 7, 8]),
    ),
    ("Maze", LifeRule::moore(&[3], &[1, 2, 3, 4, 5])),
    ("2×2", LifeRule::moore(&[3, 6], &[1, 2, 5])),
    ("Move", LifeRule::moore(&[3, 6, 8], &[2, 4, 5])),
    ("Seeds", LifeRule::moore(&[2], &[])),
    ("Coral", LifeRule::moore(&[3], &[4, 5, 6, 7, 8])),
];

/// The rule's name from the names map ("Life", "2×2", …), or `None` (the Terrarium shows
/// "Mutant", the Tray "Custom rule"). Only Moore r1 rules have names.
pub fn rule_name(rule: &LifeRule) -> Option<&'static str> {
    NAMED.iter().find(|(_, r)| r == rule).map(|(name, _)| *name)
}

/// The Terrarium's six starting rules: Life, HighLife, Day & Night, Maze, 2×2, Move.
pub const TERRARIUM_START: [LifeRule; 6] = [
    NAMED[0].1, NAMED[1].1, NAMED[2].1, NAMED[3].1, NAMED[4].1, NAMED[5].1,
];

/// The Tray's grid width (40).
pub const TRAY_W: usize = 40;
/// The Tray's grid height (26).
pub const TRAY_H: usize = 26;
/// The Tray's boundary (a torus).
pub const TRAY_BOUNDARY: Boundary = Boundary::Torus;

/// The Tray's starting garden (TRAY-7 Restart): a Gosper gun at (1, 1) and a pulsar at
/// (26, 12), top-left anchored, on the 40 × 26 torus. Run it with [`LifeRule::LIFE`].
pub fn starting_garden() -> Grid {
    let mut grid = Grid::new(TRAY_W, TRAY_H);
    if let Some(gun) = library::pattern("gun") {
        grid.place(&gun, 1, 1, Anchor::TopLeft, TRAY_BOUNDARY);
    }
    if let Some(pulsar) = library::pattern("pulsar") {
        grid.place(&pulsar, 26, 12, Anchor::TopLeft, TRAY_BOUNDARY);
    }
    grid
}

/// Whether `rule` uses the Moore r1 neighbourhood (the only one the Tray and the Terrarium use).
pub fn is_moore(rule: &LifeRule) -> bool {
    rule.neighbourhood() == Neighbourhood::MOORE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cartridges_match_the_mockup() {
        let codes: Vec<String> = CARTRIDGES.iter().map(|c| c.rule.code()).collect();
        assert_eq!(
            codes,
            [
                "B3/S23",
                "B36/S23",
                "B2/S",
                "B3678/S34678",
                "B3/S12345",
                "B3/S45678"
            ]
        );
        assert_eq!(
            matching_cartridge(&LifeRule::LIFE).map(|c| c.name),
            Some("Life")
        );
        assert_eq!(matching_cartridge(&LifeRule::moore(&[3], &[2])), None);
        assert_eq!(
            cartridge("dn").map(|c| c.note),
            Some("Light–dark symmetric")
        );
    }

    #[test]
    fn names_map_includes_2x2_and_move() {
        assert_eq!(
            rule_name(&LifeRule::parse("B36/S125").unwrap()),
            Some("2×2")
        );
        assert_eq!(
            rule_name(&LifeRule::parse("B368/S245").unwrap()),
            Some("Move")
        );
        assert_eq!(rule_name(&LifeRule::parse("B1/S1").unwrap()), None);
        let vn = LifeRule::LIFE.with_neighbourhood(Neighbourhood::VON_NEUMANN);
        assert_eq!(rule_name(&vn), None, "names are for Moore rules");
    }

    #[test]
    fn starting_garden_has_the_gun_and_the_pulsar() {
        let g = starting_garden();
        assert_eq!(g.population(), 36 + 48);
    }
}
