//! Life-like rules: neighbourhoods inside 5 × 5, birth / survival count sets, boundaries and
//! rulestrings (SPEC ENG-1, ENG-2, LAB-1).

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// The most neighbours a cell can have: every cell of the 5 × 5 square except the centre.
pub const MAX_NEIGHBOURS: usize = 24;

const ALL_COUNTS: u32 = (1 << (MAX_NEIGHBOURS + 1)) - 1;

/// A set of neighbour counts in `0..=24`, stored as a bitmask.
///
/// Used for the "born with" and "survives with" rows of a [`LifeRule`]. Counts above 24 are
/// never members; the editing methods ignore them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CountSet(u32);

impl CountSet {
    /// The empty set.
    pub const EMPTY: CountSet = CountSet(0);

    /// Builds a set from a list of counts (counts above 24 are ignored). `const`, so presets can
    /// be constants: `CountSet::of(&[2, 3])`.
    pub const fn of(counts: &[usize]) -> CountSet {
        let mut bits = 0u32;
        let mut i = 0;
        while i < counts.len() {
            if counts[i] <= MAX_NEIGHBOURS {
                bits |= 1 << counts[i];
            }
            i += 1;
        }
        CountSet(bits)
    }

    /// Builds a set from raw bits (bit `n` = count `n`); bits above 24 are dropped.
    pub const fn from_bits(bits: u32) -> CountSet {
        CountSet(bits & ALL_COUNTS)
    }

    /// The raw bits (bit `n` set when count `n` is a member).
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether `n` is in the set.
    pub const fn contains(self, n: usize) -> bool {
        n <= MAX_NEIGHBOURS && (self.0 >> n) & 1 == 1
    }

    /// Adds or removes `n`. Returns `false` (and changes nothing) when `n` is above 24.
    pub fn set(&mut self, n: usize, on: bool) -> bool {
        if n > MAX_NEIGHBOURS {
            return false;
        }
        if on {
            self.0 |= 1 << n;
        } else {
            self.0 &= !(1 << n);
        }
        true
    }

    /// Flips membership of `n`; returns the new membership (`false` for counts above 24).
    pub fn toggle(&mut self, n: usize) -> bool {
        let on = !self.contains(n);
        self.set(n, on) && on
    }

    /// Number of counts in the set.
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The largest count, if any.
    pub const fn max(self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(31 - self.0.leading_zeros() as usize)
        }
    }

    /// The counts in ascending order.
    pub fn iter(self) -> impl Iterator<Item = usize> {
        (0..=MAX_NEIGHBOURS).filter(move |&n| self.contains(n))
    }

    /// The set with every count above `max` removed.
    pub const fn up_to(self, max: usize) -> CountSet {
        if max >= MAX_NEIGHBOURS {
            self
        } else {
            CountSet(self.0 & ((1 << (max + 1)) - 1))
        }
    }
}

impl FromIterator<usize> for CountSet {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        let mut set = CountSet::EMPTY;
        for n in iter {
            set.set(n, true);
        }
        set
    }
}

impl fmt::Debug for CountSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// Bit index of offset `(dx, dy)` inside the 5 × 5 square, row-major from (−2, −2).
const fn bit_of(dx: i32, dy: i32) -> Option<u32> {
    if dx < -2 || dx > 2 || dy < -2 || dy > 2 || (dx == 0 && dy == 0) {
        None
    } else {
        Some(((dy + 2) * 5 + (dx + 2)) as u32)
    }
}

const fn mask_of(offsets: &[(i32, i32)]) -> u32 {
    let mut bits = 0;
    let mut i = 0;
    while i < offsets.len() {
        if let Some(b) = bit_of(offsets[i].0, offsets[i].1) {
            bits |= 1 << b;
        }
        i += 1;
    }
    bits
}

const CENTRE_BIT: u32 = 12;
const ALL_OFFSETS: u32 = ((1 << 25) - 1) & !(1 << CENTRE_BIT);

/// Which cells count as a cell's neighbours: any subset of the 5 × 5 square around it, never
/// the cell itself (SPEC LAB-1). Stored as a 24-bit mask, so it is `Copy`, cheap to compare and
/// hash.
///
/// Offsets are `(dx, dy)` with `dx` to the right and `dy` down, each in `-2..=2`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Neighbourhood(u32);

impl Neighbourhood {
    /// Moore radius 1: the 8 surrounding cells ("Moore r1", key `moore`).
    pub const MOORE: Neighbourhood = Neighbourhood(mask_of(&[
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ]));
    /// von Neumann radius 1: the 4 orthogonal cells ("von Neumann", key `vn`).
    pub const VON_NEUMANN: Neighbourhood =
        Neighbourhood(mask_of(&[(0, -1), (-1, 0), (1, 0), (0, 1)]));
    /// Hexagonal: Moore without the (1, −1) and (−1, 1) corners ("Hexagonal", key `hex`).
    pub const HEXAGONAL: Neighbourhood = Neighbourhood(mask_of(&[
        (-1, -1),
        (0, -1),
        (-1, 0),
        (1, 0),
        (0, 1),
        (1, 1),
    ]));
    /// Moore radius 2: all 24 cells of the 5 × 5 square ("Moore r2", key `moore2`).
    pub const MOORE2: Neighbourhood = Neighbourhood(ALL_OFFSETS);

    /// No neighbours at all.
    pub const fn empty() -> Neighbourhood {
        Neighbourhood(0)
    }

    /// Builds a neighbourhood from offsets; duplicates are fine. Errors on `(0, 0)` or an offset
    /// outside the 5 × 5 square.
    pub fn from_offsets<I: IntoIterator<Item = (i32, i32)>>(
        offsets: I,
    ) -> Result<Neighbourhood, NeighbourhoodError> {
        let mut bits = 0;
        for (dx, dy) in offsets {
            if dx == 0 && dy == 0 {
                return Err(NeighbourhoodError::Centre);
            }
            match bit_of(dx, dy) {
                Some(b) => bits |= 1 << b,
                None => return Err(NeighbourhoodError::OutOfRange { dx, dy }),
            }
        }
        Ok(Neighbourhood(bits))
    }

    /// Rebuilds a neighbourhood from [`bits`](Self::bits); the centre bit and bits above 24 are
    /// dropped.
    pub const fn from_bits(bits: u32) -> Neighbourhood {
        Neighbourhood(bits & ALL_OFFSETS)
    }

    /// The raw mask: bit `(dy + 2) * 5 + (dx + 2)` per offset.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether `(dx, dy)` is a neighbour.
    pub const fn contains(self, dx: i32, dy: i32) -> bool {
        match bit_of(dx, dy) {
            Some(b) => (self.0 >> b) & 1 == 1,
            None => false,
        }
    }

    /// Adds or removes `(dx, dy)`. Returns `false` (no change) for the centre or an offset outside
    /// the square.
    pub fn set(&mut self, dx: i32, dy: i32, on: bool) -> bool {
        match bit_of(dx, dy) {
            Some(b) => {
                if on {
                    self.0 |= 1 << b;
                } else {
                    self.0 &= !(1 << b);
                }
                true
            }
            None => false,
        }
    }

    /// Flips `(dx, dy)` (the 5 × 5 editor's click); returns the new membership. The centre and
    /// offsets outside the square are ignored and return `false`.
    pub fn toggle(&mut self, dx: i32, dy: i32) -> bool {
        let on = !self.contains(dx, dy);
        self.set(dx, dy, on) && on
    }

    /// Number of neighbours, `N` (0–24).
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Whether there are no neighbours.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The offsets in row-major order (top row first, left to right) — the same order as the
    /// mockup's presets.
    pub fn offsets(self) -> impl Iterator<Item = (i32, i32)> {
        (0..25u32)
            .filter(move |b| (self.0 >> b) & 1 == 1)
            .map(|b| (b as i32 % 5 - 2, b as i32 / 5 - 2))
    }

    /// The largest |dx| or |dy| (0 when empty, else 1 or 2).
    pub fn radius(self) -> usize {
        self.offsets()
            .map(|(dx, dy)| dx.unsigned_abs().max(dy.unsigned_abs()) as usize)
            .max()
            .unwrap_or(0)
    }

    /// The preset this set equals, if any (a custom set equal to a preset is recognised).
    pub fn preset(self) -> Option<NeighbourhoodPreset> {
        NeighbourhoodPreset::ALL
            .into_iter()
            .find(|p| p.neighbourhood() == self)
    }

    /// The preset's key (`moore`, `vn`, `hex`, `moore2`) or `custom`.
    pub fn key(self) -> &'static str {
        self.preset().map_or("custom", NeighbourhoodPreset::key)
    }

    /// The display name: the preset's name ("Moore r1", …) or "Custom (N cells)", as the Rule
    /// lab shows it.
    pub fn name(self) -> String {
        match self.preset() {
            Some(p) => p.name().to_string(),
            None => format!("Custom ({} cells)", self.len()),
        }
    }
}

impl Default for Neighbourhood {
    /// Moore r1.
    fn default() -> Self {
        Neighbourhood::MOORE
    }
}

impl fmt::Debug for Neighbourhood {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.preset() {
            Some(p) => write!(f, "Neighbourhood::{}", p.key()),
            None => f
                .debug_tuple("Neighbourhood")
                .field(&self.offsets().collect::<Vec<_>>())
                .finish(),
        }
    }
}

/// Errors from [`Neighbourhood::from_offsets`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NeighbourhoodError {
    /// `(0, 0)` is the cell itself, never a neighbour.
    #[error("(0,0) is the cell itself, not a neighbour")]
    Centre,
    /// The offset lies outside the 5 × 5 square.
    #[error("({dx},{dy}) is outside the 5 × 5 square (offsets run from -2 to 2)")]
    OutOfRange {
        /// Horizontal offset.
        dx: i32,
        /// Vertical offset.
        dy: i32,
    },
}

/// The Rule lab's neighbourhood presets (SPEC LAB-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighbourhoodPreset {
    /// Moore r1, 8 cells.
    Moore,
    /// von Neumann, 4 cells.
    VonNeumann,
    /// Hexagonal, 6 cells.
    Hexagonal,
    /// Moore r2, 24 cells.
    Moore2,
}

impl NeighbourhoodPreset {
    /// All presets in the Rule lab's button order.
    pub const ALL: [NeighbourhoodPreset; 4] = [
        NeighbourhoodPreset::Moore,
        NeighbourhoodPreset::VonNeumann,
        NeighbourhoodPreset::Hexagonal,
        NeighbourhoodPreset::Moore2,
    ];

    /// The offsets.
    pub const fn neighbourhood(self) -> Neighbourhood {
        match self {
            NeighbourhoodPreset::Moore => Neighbourhood::MOORE,
            NeighbourhoodPreset::VonNeumann => Neighbourhood::VON_NEUMANN,
            NeighbourhoodPreset::Hexagonal => Neighbourhood::HEXAGONAL,
            NeighbourhoodPreset::Moore2 => Neighbourhood::MOORE2,
        }
    }

    /// The script key: `moore`, `vn`, `hex`, `moore2`.
    pub const fn key(self) -> &'static str {
        match self {
            NeighbourhoodPreset::Moore => "moore",
            NeighbourhoodPreset::VonNeumann => "vn",
            NeighbourhoodPreset::Hexagonal => "hex",
            NeighbourhoodPreset::Moore2 => "moore2",
        }
    }

    /// The display name: "Moore r1", "von Neumann", "Hexagonal", "Moore r2".
    pub const fn name(self) -> &'static str {
        match self {
            NeighbourhoodPreset::Moore => "Moore r1",
            NeighbourhoodPreset::VonNeumann => "von Neumann",
            NeighbourhoodPreset::Hexagonal => "Hexagonal",
            NeighbourhoodPreset::Moore2 => "Moore r2",
        }
    }

    /// Looks a preset up by key, case-insensitively; also accepts a few spellings
    /// (`moore1`, `von_neumann`, `vonneumann`, `hexagonal`, `moore_r2`).
    pub fn from_key(key: &str) -> Option<NeighbourhoodPreset> {
        match key.trim().to_ascii_lowercase().as_str() {
            "moore" | "moore1" | "moore_r1" | "moore-r1" => Some(NeighbourhoodPreset::Moore),
            "vn" | "von_neumann" | "vonneumann" | "von-neumann" => {
                Some(NeighbourhoodPreset::VonNeumann)
            }
            "hex" | "hexagonal" => Some(NeighbourhoodPreset::Hexagonal),
            "moore2" | "moore_r2" | "moore-r2" => Some(NeighbourhoodPreset::Moore2),
            _ => None,
        }
    }
}

/// What happens at the grid's edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Boundary {
    /// Edges wrap around (a torus).
    #[default]
    Torus,
    /// Cells beyond the edge are always dead.
    Dead,
}

impl Boundary {
    /// Both boundaries in the Rule lab's button order.
    pub const ALL: [Boundary; 2] = [Boundary::Torus, Boundary::Dead];

    /// The script key: `torus` / `dead`.
    pub const fn key(self) -> &'static str {
        match self {
            Boundary::Torus => "torus",
            Boundary::Dead => "dead",
        }
    }

    /// The button label: "Torus" / "Dead edge".
    pub const fn label(self) -> &'static str {
        match self {
            Boundary::Torus => "Torus",
            Boundary::Dead => "Dead edge",
        }
    }

    /// The Rule lab's description: "wraps at edges" / "dead edges".
    pub const fn description(self) -> &'static str {
        match self {
            Boundary::Torus => "wraps at edges",
            Boundary::Dead => "dead edges",
        }
    }

    /// Parses `torus` / `wrap` or `dead` / `dead_edge` / `dead-edge` (case-insensitive).
    pub fn from_key(key: &str) -> Option<Boundary> {
        match key.trim().to_ascii_lowercase().as_str() {
            "torus" | "wrap" | "wraps" => Some(Boundary::Torus),
            "dead" | "dead_edge" | "dead-edge" | "deadedge" => Some(Boundary::Dead),
            _ => None,
        }
    }
}

/// A Life-like (outer-totalistic, two-state) rule: a dead cell is born when its live-neighbour
/// count is in [`birth`](Self::birth), a live cell survives when it is in
/// [`survive`](Self::survive). Counts never exceed the neighbourhood size `N`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LifeRule {
    birth: CountSet,
    survive: CountSet,
    neighbourhood: Neighbourhood,
}

impl LifeRule {
    /// Conway's Life, B3/S23 on Moore r1.
    pub const LIFE: LifeRule = LifeRule::new(
        CountSet::of(&[3]),
        CountSet::of(&[2, 3]),
        Neighbourhood::MOORE,
    );

    /// A rule; counts above the neighbourhood size are dropped.
    pub const fn new(birth: CountSet, survive: CountSet, neighbourhood: Neighbourhood) -> Self {
        let n = neighbourhood.len();
        LifeRule {
            birth: birth.up_to(n),
            survive: survive.up_to(n),
            neighbourhood,
        }
    }

    /// A Moore r1 rule from count lists, e.g. `LifeRule::moore(&[3, 6], &[2, 3])` for HighLife.
    pub const fn moore(birth: &[usize], survive: &[usize]) -> Self {
        LifeRule::new(
            CountSet::of(birth),
            CountSet::of(survive),
            Neighbourhood::MOORE,
        )
    }

    /// Parses a rulestring for Moore r1 (see [`parse_with`](Self::parse_with)).
    pub fn parse(text: &str) -> Result<LifeRule, RuleParseError> {
        LifeRule::parse_with(text, Neighbourhood::MOORE)
    }

    /// Parses a rulestring for the given neighbourhood (SPEC ENG-2).
    ///
    /// Accepted forms: `B3/S23`, `b3s23`, `B3/S23/` (empty third part), `S23/B3`, whitespace
    /// anywhere, the old `23/3` survive/birth form, a Golly topology suffix (`:T40,26`, ignored),
    /// and comma-separated counts when any count is 10 or more (`B3,10/S2,3`; a comma anywhere
    /// switches the whole string to comma mode). Counts above the neighbourhood size are errors.
    pub fn parse_with(
        text: &str,
        neighbourhood: Neighbourhood,
    ) -> Result<LifeRule, RuleParseError> {
        let text = text.split(':').next().unwrap_or("").trim();
        let text = text.trim_end_matches(|c: char| c == '/' || c.is_whitespace());
        if text.is_empty() {
            return Err(RuleParseError::Empty);
        }
        let comma_mode = text.contains(',');
        let tagged = text.chars().any(|c| c.is_ascii_alphabetic());
        let (birth_text, survive_text) = if tagged {
            split_tagged(text)?
        } else {
            let parts: Vec<&str> = text.split('/').collect();
            match parts.len() {
                2 => (parts[1].to_string(), parts[0].to_string()),
                1 => return Err(RuleParseError::NotARule(text.to_string())),
                _ => return Err(RuleParseError::Generations),
            }
        };
        let n = neighbourhood.len();
        let birth = parse_counts(&birth_text, comma_mode, n)?;
        let survive = parse_counts(&survive_text, comma_mode, n)?;
        Ok(LifeRule {
            birth,
            survive,
            neighbourhood,
        })
    }

    /// The birth counts.
    pub const fn birth(&self) -> CountSet {
        self.birth
    }

    /// The survival counts.
    pub const fn survive(&self) -> CountSet {
        self.survive
    }

    /// The neighbourhood.
    pub const fn neighbourhood(&self) -> Neighbourhood {
        self.neighbourhood
    }

    /// Whether a dead cell with `n` live neighbours is born.
    pub const fn born_with(&self, n: usize) -> bool {
        self.birth.contains(n)
    }

    /// Whether a live cell with `n` live neighbours survives.
    pub const fn survives_with(&self, n: usize) -> bool {
        self.survive.contains(n)
    }

    /// Sets "born with `n`". Ignored (returns `false`) when `n` exceeds the neighbourhood size.
    pub fn set_birth(&mut self, n: usize, on: bool) -> bool {
        n <= self.neighbourhood.len() && self.birth.set(n, on)
    }

    /// Sets "survives with `n`". Ignored (returns `false`) when `n` exceeds the neighbourhood
    /// size.
    pub fn set_survive(&mut self, n: usize, on: bool) -> bool {
        n <= self.neighbourhood.len() && self.survive.set(n, on)
    }

    /// Flips "born with `n`" (a bit button); returns the new state. No-op above `N`.
    pub fn toggle_birth(&mut self, n: usize) -> bool {
        let on = !self.born_with(n);
        self.set_birth(n, on) && on
    }

    /// Flips "survives with `n`"; returns the new state. No-op above `N`.
    pub fn toggle_survive(&mut self, n: usize) -> bool {
        let on = !self.survives_with(n);
        self.set_survive(n, on) && on
    }

    /// The same counts on another neighbourhood, dropping counts above its size — exactly what
    /// the Rule lab does when a preset is picked or a neighbour is toggled.
    pub const fn with_neighbourhood(&self, neighbourhood: Neighbourhood) -> LifeRule {
        LifeRule::new(self.birth, self.survive, neighbourhood)
    }

    /// Changes the neighbourhood in place (see [`with_neighbourhood`](Self::with_neighbourhood)).
    pub fn set_neighbourhood(&mut self, neighbourhood: Neighbourhood) {
        *self = self.with_neighbourhood(neighbourhood);
    }

    /// The 5 × 5 editor's click: flips neighbour `(dx, dy)` and drops counts above the new `N`.
    /// Returns the neighbour's new membership.
    pub fn toggle_neighbour(&mut self, dx: i32, dy: i32) -> bool {
        let mut nb = self.neighbourhood;
        let on = nb.toggle(dx, dy);
        self.set_neighbourhood(nb);
        on
    }

    /// The canonical rulestring: `B` + birth counts ascending + `/S` + survival counts, digits
    /// concatenated (`B36/S23`). When any count is 10 or more, counts are comma-separated in
    /// both parts (`B3,10/S2,3`); if that leaves no comma at all (one count per part), a
    /// trailing comma marks the part holding the big count (`B10,/S2`), so the string parses
    /// back unambiguously.
    pub fn code(&self) -> String {
        let big = self.birth.max().unwrap_or(0) >= 10 || self.survive.max().unwrap_or(0) >= 10;
        let part = |set: CountSet| -> String {
            let counts: Vec<String> = set.iter().map(|n| n.to_string()).collect();
            counts.join(if big { "," } else { "" })
        };
        let mut b = part(self.birth);
        let mut s = part(self.survive);
        if big && !b.contains(',') && !s.contains(',') {
            if self.birth.max().unwrap_or(0) >= 10 {
                b.push(',');
            } else {
                s.push(',');
            }
        }
        format!("B{b}/S{s}")
    }
}

impl Default for LifeRule {
    /// Life, B3/S23.
    fn default() -> Self {
        LifeRule::LIFE
    }
}

impl fmt::Display for LifeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.code())
    }
}

impl fmt::Debug for LifeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LifeRule({}, {:?})", self.code(), self.neighbourhood)
    }
}

impl FromStr for LifeRule {
    type Err = RuleParseError;

    /// [`LifeRule::parse`] (Moore r1).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LifeRule::parse(s)
    }
}

/// Why a rulestring didn't parse.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleParseError {
    /// Nothing to parse.
    #[error("the rule is empty (try B3/S23)")]
    Empty,
    /// Not in any known form.
    #[error("`{0}` isn't a rule; write it like B3/S23 (or the old 23/3 form)")]
    NotARule(String),
    /// A character that has no place in a rulestring.
    #[error("unexpected `{0}` in the rule (only B, S, digits, `/` and `,` are allowed)")]
    UnexpectedChar(char),
    /// Counts before any `B` / `S` tag.
    #[error("counts must follow `B` or `S` (as in B3/S23)")]
    MissingTag,
    /// `B` or `S` given twice.
    #[error("`{0}` appears twice in the rule")]
    Duplicate(char),
    /// A third part: a Generations rule.
    #[error("a third part makes a Generations rule; only Life-like rules (B…/S…) are supported")]
    Generations,
    /// A comma-separated piece that isn't a number.
    #[error("`{0}` isn't a neighbour count")]
    BadNumber(String),
    /// A count larger than the neighbourhood.
    #[error("{count} neighbours is more than this neighbourhood has ({max})")]
    CountTooLarge {
        /// The count.
        count: usize,
        /// The neighbourhood size.
        max: usize,
    },
}

/// Splits a tagged rulestring into (birth text, survive text).
fn split_tagged(text: &str) -> Result<(String, String), RuleParseError> {
    #[derive(PartialEq)]
    enum Part {
        None,
        Birth,
        Survive,
    }
    let mut cur = Part::None;
    let (mut b, mut s) = (None::<String>, None::<String>);
    for c in text.chars() {
        match c {
            'B' | 'b' => {
                if b.is_some() {
                    return Err(RuleParseError::Duplicate('B'));
                }
                b = Some(String::new());
                cur = Part::Birth;
            }
            'S' | 's' => {
                if s.is_some() {
                    return Err(RuleParseError::Duplicate('S'));
                }
                s = Some(String::new());
                cur = Part::Survive;
            }
            '/' => cur = Part::None,
            c if c.is_whitespace() => {}
            c if c.is_ascii_digit() || c == ',' => match cur {
                Part::Birth => b.get_or_insert_with(String::new).push(c),
                Part::Survive => s.get_or_insert_with(String::new).push(c),
                Part::None if b.is_some() && s.is_some() => {
                    return Err(RuleParseError::Generations);
                }
                Part::None => return Err(RuleParseError::MissingTag),
            },
            c => return Err(RuleParseError::UnexpectedChar(c)),
        }
    }
    Ok((b.unwrap_or_default(), s.unwrap_or_default()))
}

fn parse_counts(text: &str, comma_mode: bool, max: usize) -> Result<CountSet, RuleParseError> {
    let mut set = CountSet::EMPTY;
    let mut add = |count: usize| {
        if count > max {
            Err(RuleParseError::CountTooLarge { count, max })
        } else {
            set.set(count, true);
            Ok(())
        }
    };
    if comma_mode {
        for piece in text.split(',') {
            let piece: String = piece.chars().filter(|c| !c.is_whitespace()).collect();
            if piece.is_empty() {
                continue;
            }
            match piece.parse::<usize>() {
                Ok(n) => add(n)?,
                Err(_) => return Err(RuleParseError::BadNumber(piece)),
            }
        }
    } else {
        for c in text.chars() {
            match c.to_digit(10) {
                Some(d) => add(d as usize)?,
                None if c.is_whitespace() => {}
                None => return Err(RuleParseError::UnexpectedChar(c)),
            }
        }
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_the_mockup_sizes_and_offsets() {
        assert_eq!(Neighbourhood::MOORE.len(), 8);
        assert_eq!(Neighbourhood::VON_NEUMANN.len(), 4);
        assert_eq!(Neighbourhood::HEXAGONAL.len(), 6);
        assert_eq!(Neighbourhood::MOORE2.len(), 24);
        let hex: Vec<_> = Neighbourhood::HEXAGONAL.offsets().collect();
        assert_eq!(
            hex,
            vec![(-1, -1), (0, -1), (-1, 0), (1, 0), (0, 1), (1, 1)]
        );
        assert!(!Neighbourhood::MOORE2.contains(0, 0));
        assert_eq!(Neighbourhood::MOORE.radius(), 1);
        assert_eq!(Neighbourhood::MOORE2.radius(), 2);
    }

    #[test]
    fn custom_sets_equal_to_a_preset_are_recognised() {
        let vn = Neighbourhood::from_offsets([(1, 0), (0, 1), (-1, 0), (0, -1), (1, 0)]).unwrap();
        assert_eq!(vn.preset(), Some(NeighbourhoodPreset::VonNeumann));
        assert_eq!(vn.key(), "vn");
        let mut n = Neighbourhood::MOORE;
        assert!(!n.toggle(1, 1));
        assert_eq!(n.preset(), None);
        assert_eq!(n.name(), "Custom (7 cells)");
        assert_eq!(n.key(), "custom");
        assert!(!n.toggle(0, 0), "the centre never toggles");
        assert_eq!(n.len(), 7);
        assert_eq!(
            Neighbourhood::from_offsets([(0, 0)]),
            Err(NeighbourhoodError::Centre)
        );
        assert!(Neighbourhood::from_offsets([(3, 0)]).is_err());
    }

    #[test]
    fn parses_every_form() {
        let life = LifeRule::LIFE;
        for s in [
            "B3/S23",
            "b3s23",
            "B3/S23/",
            "S23/B3",
            " B3 / S23 ",
            "23/3",
            "B3S23",
            "B3/S23:T40,26",
        ] {
            assert_eq!(LifeRule::parse(s), Ok(life), "{s}");
        }
        let seeds = LifeRule::parse("B2/S").unwrap();
        assert_eq!(seeds.code(), "B2/S");
        assert_eq!(LifeRule::parse("/2").unwrap(), seeds);
        assert_eq!(LifeRule::parse("B2").unwrap(), seeds);
        assert_eq!(
            LifeRule::parse("B33/S32").unwrap(),
            life,
            "duplicates collapse"
        );
    }

    #[test]
    fn rejects_bad_rules_with_reasons() {
        assert_eq!(LifeRule::parse(""), Err(RuleParseError::Empty));
        assert_eq!(
            LifeRule::parse("B9/S"),
            Err(RuleParseError::CountTooLarge { count: 9, max: 8 })
        );
        assert_eq!(
            LifeRule::parse("B3/S23/2"),
            Err(RuleParseError::Generations)
        );
        assert_eq!(LifeRule::parse("1/2/3"), Err(RuleParseError::Generations));
        assert_eq!(
            LifeRule::parse("3"),
            Err(RuleParseError::NotARule("3".into()))
        );
        assert_eq!(
            LifeRule::parse("B3/B3"),
            Err(RuleParseError::Duplicate('B'))
        );
        assert_eq!(
            LifeRule::parse("B3/S2x"),
            Err(RuleParseError::UnexpectedChar('x'))
        );
        assert_eq!(LifeRule::parse("3B/S"), Err(RuleParseError::MissingTag));
        assert_eq!(
            LifeRule::parse_with("B5/S", Neighbourhood::VON_NEUMANN),
            Err(RuleParseError::CountTooLarge { count: 5, max: 4 })
        );
    }

    #[test]
    fn big_counts_are_comma_separated_and_round_trip() {
        let mut r = LifeRule::new(
            CountSet::of(&[3, 10]),
            CountSet::of(&[2, 3]),
            Neighbourhood::MOORE2,
        );
        assert_eq!(r.code(), "B3,10/S2,3");
        assert_eq!(
            LifeRule::parse_with(&r.code(), Neighbourhood::MOORE2),
            Ok(r)
        );
        r = LifeRule::new(CountSet::of(&[10]), CountSet::EMPTY, Neighbourhood::MOORE2);
        assert_eq!(r.code(), "B10,/S");
        assert_eq!(
            LifeRule::parse_with(&r.code(), Neighbourhood::MOORE2),
            Ok(r)
        );
        r = LifeRule::new(
            CountSet::of(&[2]),
            CountSet::of(&[24]),
            Neighbourhood::MOORE2,
        );
        assert_eq!(r.code(), "B2/S24,");
        assert_eq!(
            LifeRule::parse_with(&r.code(), Neighbourhood::MOORE2),
            Ok(r)
        );
    }

    #[test]
    fn changing_the_neighbourhood_drops_counts_above_n() {
        let dn = LifeRule::moore(&[3, 6, 7, 8], &[3, 4, 6, 7, 8]);
        let vn = dn.with_neighbourhood(Neighbourhood::VON_NEUMANN);
        assert_eq!(vn.code(), "B3/S34");
        // Growing back does not restore them (the mockup filters, it doesn't remember).
        assert_eq!(vn.with_neighbourhood(Neighbourhood::MOORE).code(), "B3/S34");
        let mut r = LifeRule::LIFE.with_neighbourhood(Neighbourhood::VON_NEUMANN);
        assert!(
            !r.toggle_birth(5),
            "count 5 doesn't exist with 4 neighbours"
        );
        assert!(r.toggle_birth(4));
        assert_eq!(r.code(), "B34/S23");
        r.toggle_neighbour(0, -1);
        assert_eq!(r.code(), "B3/S23");
        assert_eq!(r.neighbourhood().len(), 3);
    }
}
