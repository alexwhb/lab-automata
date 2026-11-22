//! The Rule lab's model, without any GUI: the spec in effect (the last valid [`RuleScript`]),
//! the editor's text and its errors, the [`Run`] with its [`Report`] and the scrubbed
//! generation (SPEC LAB-1…5).
//!
//! ## Script ⇄ spec sync (LAB-4)
//!
//! The script text is the netlist of the run and both sides edit it:
//!
//! - **Typing** ([`Lab::edit_text`]): the text is parsed on every change. When it parses, its
//!   spec takes effect at once (the run is redone if the spec changed) and the text is left
//!   exactly as typed. When it doesn't, the errors are kept per line and the last valid spec
//!   stays in effect.
//! - **The spec controls** ([`Lab::set_spec`]): the spec changes at once. The text follows by
//!   rewriting only the directive lines whose canonical form changed ([`patch_text`]); every
//!   other line (comments, spacing, a line being fixed) stays. For canonical text that is
//!   exactly `RuleScript::to_text()`. The line of the directive you changed is replaced even
//!   if it held an error (that control owns the line); errors elsewhere stay.
//! - **Never while typing into an error**: if the editor has keyboard focus and the text has
//!   errors, a spec change leaves the text alone and the patch is applied when the editor
//!   loses focus ([`Lab::flush_deferred`]).
//!
//! Runs are synchronous while they are small (the Rule lab's 64 × 40 × 400 takes a few ms);
//! bigger specs typed into the script (up to 256 × 256 × 2000) run on a worker thread so the
//! editor never stalls ([`Lab::poll`] adopts the result; the old run stays on screen meanwhile).

use ap_engine::analysis;
use ap_engine::{LifeRule, Report, RuleScript, Run, RunSpec, ScriptError, Seed};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// The generation the Rule lab opens scrubbed to (the mockup's 120).
pub const START_GENERATION: usize = 120;

/// The directives the spec controls own, in script order.
pub const DIRECTIVES: [&str; 5] = [".rule", ".neigh", ".grid", ".seed", ".run"];

/// Runs with more cell-generations than this go to a worker thread (the default
/// 64 × 40 × 400 is 1 024 000).
pub const SYNC_BUDGET: usize = 4_000_000;

/// Characters of RLE body the export panel shows before "…" (the mockup's 420).
pub const RLE_PREVIEW: usize = 420;

/// A run computing on a worker thread.
#[derive(Debug)]
struct Pending {
    spec: RunSpec,
    rx: Receiver<Run>,
}

/// The Rule lab's state. See the module docs for how the text and the spec stay in sync.
#[derive(Debug)]
pub struct Lab {
    script: RuleScript,
    text: String,
    errors: Vec<ScriptError>,
    deferred: Option<RuleScript>,
    run: Run,
    report: Report,
    pending: Option<Pending>,
    generation: usize,
}

impl Default for Lab {
    fn default() -> Self {
        Lab::new()
    }
}

impl Lab {
    /// The mockup's lab: `glider-garden.rule` (B3/S23, Moore r1, 64 × 40 torus, R-pentomino,
    /// 400 gens), run, scrubbed to generation 120.
    pub fn new() -> Self {
        let script = RuleScript::default();
        let run = analysis::run(&script.spec());
        let report = run.report();
        Lab {
            text: script.to_text(),
            generation: START_GENERATION.min(run.gens()),
            script,
            errors: Vec::new(),
            deferred: None,
            run,
            report,
            pending: None,
        }
    }

    // ---- reading

    /// The spec in effect (the last valid script's).
    pub fn spec(&self) -> RunSpec {
        self.script.spec()
    }

    /// The last valid script (name, spec, probes).
    pub fn script(&self) -> &RuleScript {
        &self.script
    }

    /// The editor's text (may have errors).
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The text's errors, sorted by line (empty when it parses).
    pub fn errors(&self) -> &[ScriptError] {
        &self.errors
    }

    /// The run on screen (the spec's, or the previous one while a big run computes).
    pub fn run(&self) -> &Run {
        &self.run
    }

    /// The analysis of [`Lab::run`].
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// True while a big run computes on the worker thread.
    pub fn is_running(&self) -> bool {
        self.pending.is_some()
    }

    /// The scrubbed generation.
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// The last generation of the run on screen.
    pub fn gens(&self) -> usize {
        self.run.gens()
    }

    /// Population at the scrubbed generation.
    pub fn alive(&self) -> usize {
        self.run.population(self.generation).unwrap_or(0)
    }

    /// The scrubbed generation as RLE with `rule =` (full, untruncated).
    pub fn rle(&self) -> String {
        self.run
            .rle(self.generation)
            .unwrap_or_else(|| "x = 0, y = 0\n!".into())
    }

    /// The export panel's text: the RLE header line, then the body on one line, cut after
    /// [`RLE_PREVIEW`] characters with "…" like the mockup.
    pub fn rle_preview(&self) -> String {
        preview(&self.rle())
    }

    /// What "Copy script" / "Save .rule…" use: the text as typed when it parses, else the
    /// last valid script printed canonically.
    pub fn script_for_export(&self) -> String {
        if self.errors.is_empty() {
            self.text.clone()
        } else {
            self.script.to_text()
        }
    }

    /// The document's file stem (`glider-garden` for `glider-garden.rule`).
    pub fn file_stem(&self) -> String {
        file_stem(&self.script.name)
    }

    // ---- scrubbing

    /// Scrubs to `generation` (clamped to the run).
    pub fn set_generation(&mut self, generation: usize) {
        self.generation = generation.min(self.gens());
    }

    /// One generation on, looping from the last back to 0 (Play).
    pub fn advance(&mut self) {
        self.generation = if self.generation >= self.gens() {
            0
        } else {
            self.generation + 1
        };
    }

    // ---- editing

    /// The spec controls changed the spec: it takes effect now and the text follows (module
    /// docs). `editing` is true while the editor has keyboard focus.
    pub fn set_spec(&mut self, spec: RunSpec, editing: bool) {
        if spec == self.spec() {
            return;
        }
        let before = self.script.clone();
        let mut next = self.script.clone();
        next.set_spec(&spec);
        self.adopt(next, true);
        if editing && !self.errors.is_empty() {
            // the user is typing into an error: patch the text once they leave the editor
            self.deferred.get_or_insert(before);
        } else {
            self.patch_from(&before);
        }
    }

    /// Changes only the rule (and its neighbourhood).
    pub fn set_rule(&mut self, rule: LifeRule, editing: bool) {
        let mut spec = self.spec();
        spec.rule = rule;
        self.set_spec(spec, editing);
    }

    /// The editor's text changed (typing, paste): parse it and apply it when valid.
    pub fn edit_text(&mut self, text: String) {
        if text == self.text {
            return;
        }
        self.text = text;
        self.deferred = None;
        match RuleScript::parse(&self.text) {
            Ok(script) => {
                self.errors.clear();
                self.adopt(script, true);
            }
            Err(errors) => self.errors = errors,
        }
    }

    /// Replaces the whole script if it parses (the control API): the text becomes `text`, its
    /// spec takes effect and runs synchronously. On errors nothing changes.
    pub fn set_script(&mut self, text: &str) -> Result<(), Vec<ScriptError>> {
        let script = RuleScript::parse(text)?;
        self.text = text.to_string();
        self.errors.clear();
        self.deferred = None;
        self.adopt(script, false);
        Ok(())
    }

    /// The editor lost focus: apply a text patch postponed while the user was typing.
    pub fn flush_deferred(&mut self) {
        if let Some(before) = self.deferred.take() {
            self.patch_from(&before);
        }
    }

    /// Adopts a finished background run (call every frame). Returns true when the run on
    /// screen changed.
    pub fn poll(&mut self) -> bool {
        let Some(p) = &self.pending else {
            return false;
        };
        match p.rx.try_recv() {
            Ok(run) => {
                self.pending = None;
                self.install(run);
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                // the worker panicked (it never should): run here instead
                let spec = p.spec;
                self.pending = None;
                self.install(analysis::run(&spec));
                true
            }
        }
    }

    /// Waits for a background run, if any (the control API reads results right away).
    pub fn finish(&mut self) {
        if let Some(p) = self.pending.take() {
            let run = p.rx.recv().unwrap_or_else(|_| analysis::run(&p.spec));
            self.install(run);
        }
    }

    // ---- internals

    /// Makes `script` the one in effect, re-running if its spec differs from the run's.
    fn adopt(&mut self, script: RuleScript, allow_async: bool) {
        let spec = script.spec();
        self.script = script;
        let current = self.pending.as_ref().map_or(*self.run.spec(), |p| p.spec);
        if spec.clamped() == current {
            return;
        }
        let cost = spec.w * spec.h * (spec.gens + 1);
        if allow_async && cost > SYNC_BUDGET {
            let (tx, rx) = channel();
            let job = spec;
            let spawned = std::thread::Builder::new()
                .name("rule-lab-run".into())
                .spawn(move || {
                    let _ = tx.send(analysis::run(&job));
                });
            if spawned.is_ok() {
                // a newer request replaces an older one (its result is dropped)
                self.pending = Some(Pending {
                    spec: spec.clamped(),
                    rx,
                });
                return;
            }
        }
        self.pending = None;
        self.install(analysis::run(&spec));
    }

    fn install(&mut self, run: Run) {
        self.report = run.report();
        self.run = run;
        self.generation = self.generation.min(self.run.gens());
    }

    /// Rewrites the text for the change from `before` to the script in effect.
    fn patch_from(&mut self, before: &RuleScript) {
        let patched = patch_text(&self.text, before, &self.script);
        match RuleScript::parse(&patched) {
            Ok(script) if script.spec() == self.script.spec() => {
                self.text = patched;
                self.errors.clear();
                self.script = script;
            }
            Ok(script) => {
                // valid text says something else (lines the user typed but never applied): the
                // text is the source of truth once it parses
                self.text = patched;
                self.errors.clear();
                self.adopt(script, true);
            }
            Err(errors) if self.errors.is_empty() => {
                // can't happen for text that parsed before; fall back to the canonical script
                let _ = errors;
                self.text = self.script.to_text();
            }
            Err(errors) => {
                self.text = patched;
                self.errors = errors;
            }
        }
    }
}

/// The directive a script line starts with (lower-case), if it is one of [`DIRECTIVES`].
pub fn directive_of(line: &str) -> Option<&'static str> {
    let first = line.split_whitespace().next()?.to_ascii_lowercase();
    DIRECTIVES.into_iter().find(|d| *d == first)
}

/// Rewrites the directive lines of `text` whose canonical form differs between `old` and
/// `new` with `new`'s line (the first occurrence of each); a directive missing from `text` is
/// inserted before `.end` (or at the end). Every other line is kept as it is.
pub fn patch_text(text: &str, old: &RuleScript, new: &RuleScript) -> String {
    let canonical = |s: &RuleScript| -> Vec<(&'static str, String)> {
        s.to_text()
            .lines()
            .filter_map(|l| directive_of(l).map(|d| (d, l.to_string())))
            .collect()
    };
    let (was, now) = (canonical(old), canonical(new));
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for (directive, line) in now {
        let unchanged = was.iter().any(|(d, l)| *d == directive && *l == line);
        if unchanged {
            continue;
        }
        match lines
            .iter()
            .position(|l| directive_of(l) == Some(directive))
        {
            Some(i) => lines[i] = line,
            None => {
                let end = lines
                    .iter()
                    .position(|l| l.trim().eq_ignore_ascii_case(".end"))
                    .unwrap_or(lines.len());
                lines.insert(end, line);
            }
        }
    }
    lines.join("\n")
}

/// The export panel's preview of an RLE text: the header line, then the body joined on one
/// line and cut after [`RLE_PREVIEW`] characters with "…".
pub fn preview(rle: &str) -> String {
    let mut header = Vec::new();
    let mut body = String::new();
    for line in rle.lines() {
        let t = line.trim();
        if body.is_empty() && (t.starts_with('#') || ap_engine::pattern::rle::is_header(t)) {
            header.push(t);
        } else {
            body.push_str(t);
        }
    }
    let cut: String = body.chars().take(RLE_PREVIEW).collect();
    let more = body.chars().count() > RLE_PREVIEW;
    let mut out = header.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&cut);
    if more {
        out.push('…');
    }
    out
}

/// `glider-garden` for `glider-garden.rule` (safe as a file name).
pub fn file_stem(name: &str) -> String {
    let base = name.trim();
    let base = base.strip_suffix(".rule").unwrap_or(base);
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    if clean.is_empty() {
        "untitled".into()
    } else {
        clean
    }
}

/// The seed a Seed button picks: the soup keeps its parameters when it already is one.
pub fn seed_choice(current: Seed, pick: usize) -> Seed {
    match pick {
        0 => Seed::RPentomino,
        1 => Seed::Acorn,
        _ => match current {
            s @ Seed::Soup { .. } => s,
            _ => Seed::MOCKUP_SOUP,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ap_engine::{Boundary, Neighbourhood};

    #[test]
    fn opens_on_the_mockup() {
        let lab = Lab::new();
        assert_eq!(lab.generation(), 120);
        assert_eq!(lab.alive(), 148);
        let r = lab.report();
        assert_eq!(r.peak_text(), "221 @ 148");
        assert_eq!(r.final_pop, 46);
        assert_eq!(r.first_repeat_text(), "gen 390");
        assert_eq!(r.period_text(), "2");
        assert_eq!(r.verdict, "Settles into a period-2 cycle at gen 388.");
        assert_eq!(lab.text(), RuleScript::default().to_text());
        assert!(lab.rle().starts_with("x = 64, y = 34, rule = B3/S23"));
    }

    #[test]
    fn spec_changes_patch_only_their_lines() {
        let mut lab = Lab::new();
        let custom = format!("* my notes\n{}", lab.text().replace(".run    ", ".run "));
        lab.edit_text(custom.clone());
        assert!(lab.errors().is_empty());
        let mut spec = lab.spec();
        spec.boundary = Boundary::Dead;
        lab.set_spec(spec, false);
        assert!(lab.text().starts_with("* my notes\n"), "comments survive");
        assert!(lab.text().contains(".run gens=400"), "spacing survives");
        assert!(lab.text().contains(".grid   64x40  boundary=dead"));
        assert_eq!(lab.spec().boundary, Boundary::Dead);
        // canonical text stays canonical
        let mut lab = Lab::new();
        lab.set_spec(spec, false);
        let mut want = RuleScript::default();
        want.set_spec(&spec);
        assert_eq!(lab.text(), want.to_text());
    }

    #[test]
    fn errors_keep_the_last_valid_spec_and_recover() {
        let mut lab = Lab::new();
        let good = lab.text().to_string();
        lab.edit_text(good.replace("B3/S23", "B3/Q23"));
        assert_eq!(lab.errors().len(), 1);
        assert_eq!(lab.errors()[0].line, 2);
        assert_eq!(lab.spec().rule, LifeRule::LIFE, "last valid spec stays");
        assert_eq!(lab.report().final_pop, 46);
        lab.edit_text(good.replace("B3/S23", "B36/S23"));
        assert!(lab.errors().is_empty());
        assert_eq!(lab.spec().rule.code(), "B36/S23");
    }

    #[test]
    fn controls_never_rewrite_an_error_being_typed() {
        let mut lab = Lab::new();
        let broken = lab.text().replace("boundary=torus", "boundary=tor");
        lab.edit_text(broken.clone());
        assert!(!lab.errors().is_empty());
        let mut spec = lab.spec();
        spec.rule.toggle_birth(6);
        lab.set_spec(spec, true);
        assert_eq!(lab.text(), broken, "untouched while typing");
        assert_eq!(lab.spec().rule.code(), "B36/S23", "the spec changed anyway");
        lab.flush_deferred();
        assert!(
            lab.text().contains("B36/S23"),
            "patched on leaving the editor"
        );
        assert!(
            lab.text().contains("boundary=tor\n"),
            "the broken line stays"
        );
        assert_eq!(lab.errors().len(), 1);
        // fixing the line keeps the control's change
        let fixed = lab.text().replace("boundary=tor\n", "boundary=torus\n");
        lab.edit_text(fixed);
        assert!(lab.errors().is_empty());
        assert_eq!(lab.spec().rule.code(), "B36/S23");
    }

    #[test]
    fn a_control_owns_its_line_even_when_broken() {
        let mut lab = Lab::new();
        lab.edit_text(lab.text().replace("boundary=torus", "boundary=tor"));
        let mut spec = lab.spec();
        spec.boundary = Boundary::Dead;
        lab.set_spec(spec, false);
        assert!(lab.errors().is_empty(), "{:?}", lab.errors());
        assert!(lab.text().contains("boundary=dead"));
    }

    #[test]
    fn neighbourhood_changes_rewrite_rule_and_neigh() {
        let mut lab = Lab::new();
        let mut rule = lab.spec().rule;
        rule.set_neighbourhood(Neighbourhood::VON_NEUMANN);
        lab.set_rule(rule, false);
        assert!(lab.text().contains(".rule   life_like  B3/S23"));
        assert!(lab.text().contains(".neigh  vn  n=4"));
        let mut rule = lab.spec().rule;
        rule.toggle_neighbour(0, -1);
        rule.toggle_neighbour(-1, 0);
        lab.set_rule(rule, false);
        assert_eq!(lab.spec().rule.code(), "B/S2", "counts above N=2 dropped");
        assert!(lab.text().contains(".neigh  custom  n=2"), "{}", lab.text());
    }

    #[test]
    fn missing_directives_are_inserted_before_end() {
        let old = RuleScript::default();
        let mut new = old.clone();
        new.boundary = Boundary::Dead;
        let text = old.to_text().replace(".grid   64x40  boundary=torus\n", "");
        let patched = patch_text(&text, &old, &new);
        let lines: Vec<&str> = patched.lines().collect();
        assert_eq!(lines[lines.len() - 2], ".grid   64x40  boundary=dead");
    }

    #[test]
    fn set_script_is_atomic() {
        let mut lab = Lab::new();
        let before = lab.text().to_string();
        let errs = lab.set_script(".rule life_like B3/S23\n.end").unwrap_err();
        assert!(!errs.is_empty());
        assert_eq!(lab.text(), before);
        assert!(lab.errors().is_empty());
        lab.set_script(&before.replace("r_pentomino @ (32,20)", "acorn"))
            .unwrap();
        assert_eq!(lab.spec().seed, Seed::Acorn);
    }

    #[test]
    fn big_runs_go_to_a_worker() {
        let mut lab = Lab::new();
        let text = lab
            .text()
            .replace("64x40", "200x200")
            .replace("r_pentomino @ (32,20)", "r_pentomino")
            .replace("gens=400", "gens=300");
        lab.edit_text(text);
        assert!(lab.errors().is_empty(), "{:?}", lab.errors());
        assert!(lab.is_running());
        assert_eq!(lab.run().spec().w, 64, "old run on screen meanwhile");
        lab.finish();
        assert!(!lab.is_running());
        assert_eq!(lab.run().spec().w, 200);
        assert_eq!(lab.gens(), 300);
    }

    #[test]
    fn previews_cut_like_the_mockup() {
        let lab = Lab::new();
        let rle = lab.rle();
        let body: String = rle.lines().skip(1).collect();
        assert!(body.chars().count() <= RLE_PREVIEW, "gen 120 fits");
        assert_eq!(
            lab.rle_preview(),
            format!("x = 64, y = 34, rule = B3/S23\n{body}")
        );
        let long = format!("x = 500, y = 1, rule = B3/S23\n{}!", "ob".repeat(300));
        let p = preview(&long);
        let mut lines = p.lines();
        assert_eq!(lines.next(), Some("x = 500, y = 1, rule = B3/S23"));
        let cut = lines.next().unwrap();
        assert_eq!(cut.chars().count(), RLE_PREVIEW + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(preview("x = 1, y = 1\no!"), "x = 1, y = 1\no!");
    }

    #[test]
    fn play_loops_and_scrub_clamps() {
        let mut lab = Lab::new();
        lab.set_generation(9999);
        assert_eq!(lab.generation(), 400);
        lab.advance();
        assert_eq!(lab.generation(), 0);
        lab.advance();
        assert_eq!(lab.generation(), 1);
    }

    #[test]
    fn file_stems() {
        assert_eq!(file_stem("glider-garden.rule"), "glider-garden");
        assert_eq!(file_stem("a/b:c.rule"), "a-b-c");
        assert_eq!(file_stem("  "), "untitled");
    }
}
