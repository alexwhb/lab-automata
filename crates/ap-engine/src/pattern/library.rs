//! The Tray's eight stamps (SPEC TRAY-5), verbatim from the mockup's `P` table.

use super::Pattern;

/// A built-in stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    /// Stable id, as in the mockup (`glider`, `lwss`, …).
    pub id: &'static str,
    /// Display name ("Spaceship" for the LWSS, "Glider gun" for the Gosper gun).
    pub name: &'static str,
    /// The cells as text rows (`O` alive, `.` dead), exactly the mockup's.
    pub rows: &'static [&'static str],
}

impl Stamp {
    /// The stamp as a [`Pattern`] (width = longest row, height = number of rows).
    pub fn pattern(&self) -> Pattern {
        Pattern::from_rows(self.rows)
    }
}

/// The stamps in the Tray's order.
pub const STAMPS: [Stamp; 8] = [
    Stamp {
        id: "glider",
        name: "Glider",
        rows: &[".O.", "..O", "OOO"],
    },
    Stamp {
        id: "lwss",
        name: "Spaceship",
        rows: &[".O..O", "O....", "O...O", "OOOO."],
    },
    Stamp {
        id: "pulsar",
        name: "Pulsar",
        rows: &[
            "..OOO...OOO..",
            ".............",
            "O....O.O....O",
            "O....O.O....O",
            "O....O.O....O",
            "..OOO...OOO..",
            ".............",
            "..OOO...OOO..",
            "O....O.O....O",
            "O....O.O....O",
            "O....O.O....O",
            ".............",
            "..OOO...OOO..",
        ],
    },
    Stamp {
        id: "penta",
        name: "Pentadecathlon",
        rows: &["..O....O..", "OO.OOOO.OO", "..O....O.."],
    },
    Stamp {
        id: "rpent",
        name: "R-pentomino",
        rows: &[".OO", "OO.", ".O."],
    },
    Stamp {
        id: "acorn",
        name: "Acorn",
        rows: &[".O.....", "...O...", "OO..OOO"],
    },
    Stamp {
        id: "gun",
        name: "Glider gun",
        rows: &[
            "........................O",
            "......................O.O",
            "............OO......OO............OO",
            "...........O...O....OO............OO",
            "OO........O.....O...OO",
            "OO........O...O.OO....O.O",
            "..........O.....O.......O",
            "...........O...O",
            "............OO",
        ],
    },
    Stamp {
        id: "rep",
        name: "Replicator",
        rows: &["..OOO", ".O..O", "O...O", "O..O.", "OOO.."],
    },
];

/// The stamp with this id.
pub fn stamp(id: &str) -> Option<&'static Stamp> {
    STAMPS.iter().find(|s| s.id == id)
}

/// The pattern of the stamp with this id.
pub fn pattern(id: &str) -> Option<Pattern> {
    stamp(id).map(Stamp::pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_sizes_and_populations() {
        let sizes: Vec<(usize, usize, usize)> = STAMPS
            .iter()
            .map(|s| {
                let p = s.pattern();
                (p.width(), p.height(), p.population())
            })
            .collect();
        assert_eq!(
            sizes,
            [
                (3, 3, 5),
                (5, 4, 9),
                (13, 13, 48),
                (10, 3, 12),
                (3, 3, 5),
                (7, 3, 7),
                (36, 9, 36),
                (5, 5, 12)
            ]
        );
        assert_eq!(stamp("lwss").map(|s| s.name), Some("Spaceship"));
        assert!(stamp("nope").is_none());
    }
}
