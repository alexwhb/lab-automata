//! The lab series' apps (the header's app switcher, the Series screen) and the injectable
//! launcher that opens a sibling app (NAV-3).
//!
//! The real launcher ([`Launcher::system`]) looks for the app by bundle id with Spotlight
//! (`mdfind`), then in `/Applications` and `~/Applications`, then in the lab-series checkout's
//! `../<repo>/dist/*.app`, and opens what it finds with `open`. Tests use
//! [`Launcher::recording`], which records every call and pretends a chosen set of apps is
//! installed.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One app of the lab series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppId {
    /// Circuits · SPICE.
    Breadbox,
    /// Reactions · kinetics.
    Chem,
    /// Gene circuits · populations (planned, not built yet).
    Bio,
    /// This app.
    Automata,
}

/// Whether an app can be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Built and shipped: open it if it's installed.
    Released,
    /// Announced but not built yet (Bio Playground): nothing to open.
    Planned,
    /// The app you're in.
    Current,
}

impl AppId {
    /// In switcher order.
    pub const ALL: [AppId; 4] = [AppId::Breadbox, AppId::Chem, AppId::Bio, AppId::Automata];
    /// The other three.
    pub const SIBLINGS: [AppId; 3] = [AppId::Breadbox, AppId::Chem, AppId::Bio];

    /// Name, domain and colour ramp.
    pub fn identity(self) -> ap_theme::Identity {
        match self {
            AppId::Breadbox => ap_theme::BREADBOX,
            AppId::Chem => ap_theme::CHEM,
            AppId::Bio => ap_theme::BIO,
            AppId::Automata => ap_theme::AUTOMATA,
        }
    }

    /// Display name ("Chem Playground").
    pub fn name(self) -> &'static str {
        self.identity().name
    }

    /// The switcher button's accessible name, as in the mockup's `aria-label`.
    pub fn switcher_label(self) -> &'static str {
        match self {
            AppId::Breadbox => "Breadbox circuit app",
            AppId::Chem => "Chem Playground",
            AppId::Bio => "Bio Playground",
            AppId::Automata => "Automata Playground (current)",
        }
    }

    /// The macOS bundle identifier (`None` for apps that don't exist yet).
    pub fn bundle_id(self) -> Option<&'static str> {
        match self {
            // ../breadbox/scripts/bundle-macos.sh
            AppId::Breadbox => Some("app.breadbox.Breadbox"),
            AppId::Chem => Some("dev.labseries.ChemPlayground"),
            AppId::Bio => None,
            AppId::Automata => Some(crate::APP_ID),
        }
    }

    /// The app's folder in the lab-series checkout (`lab-series/<repo>/dist/*.app`).
    pub fn repo_dir(self) -> &'static str {
        match self {
            AppId::Breadbox => "breadbox",
            AppId::Chem => "chem-playground",
            AppId::Bio => "bio-playground",
            AppId::Automata => "automata-playground",
        }
    }

    /// Released, planned or current.
    pub fn availability(self) -> Availability {
        match self {
            AppId::Breadbox | AppId::Chem => Availability::Released,
            AppId::Bio => Availability::Planned,
            AppId::Automata => Availability::Current,
        }
    }

    /// The Series card's tag ("existing", "planned", "new").
    pub fn tag(self) -> &'static str {
        match self.availability() {
            Availability::Released => "existing",
            Availability::Planned => "planned",
            Availability::Current => "new",
        }
    }
}

/// What happened when asked to open an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// It was found and `open`ed (the path or bundle id that was opened).
    Opened(String),
    /// Not found anywhere.
    NotInstalled,
    /// The app doesn't exist yet (Bio Playground).
    Planned,
    /// That's this app.
    AlreadyHere,
    /// Found, but `open` failed (the error).
    Failed(String),
}

impl Launch {
    /// The toast that reports this outcome for `app`.
    pub fn message(&self, app: AppId) -> String {
        let name = app.name();
        match self {
            Launch::Opened(_) => format!("Opening {name}…"),
            Launch::NotInstalled => format!("{name} isn't installed on this Mac"),
            Launch::Planned => format!("{name} is planned: it isn't built yet"),
            Launch::AlreadyHere => format!("You're already in {name}"),
            Launch::Failed(e) => format!("Couldn't open {name}: {e}"),
        }
    }
}

type LaunchFn = dyn Fn(AppId) -> Launch + Send + Sync;

/// Opens sibling apps. Cheap to clone: clones share the call log, so a test can keep one and
/// hand the other to the app.
#[derive(Clone)]
pub struct Launcher {
    open: Arc<LaunchFn>,
    calls: Arc<Mutex<Vec<AppId>>>,
}

impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Launcher")
            .field("calls", &self.calls())
            .finish_non_exhaustive()
    }
}

impl Default for Launcher {
    fn default() -> Self {
        Self::system()
    }
}

impl Launcher {
    /// The real launcher: finds the app (Spotlight by bundle id, `/Applications`,
    /// `~/Applications`, the lab-series `dist/` folders) and `open`s it.
    pub fn system() -> Self {
        Self::from_fn(system_launch)
    }

    /// A test launcher: records every call; apps in `installed` "open", the planned one says
    /// planned, the others aren't installed.
    pub fn recording(installed: &[AppId]) -> Self {
        let installed = installed.to_vec();
        Self::from_fn(move |app| match app.availability() {
            Availability::Planned => Launch::Planned,
            Availability::Current => Launch::AlreadyHere,
            Availability::Released if installed.contains(&app) => {
                Launch::Opened(format!("test:{}", app.repo_dir()))
            }
            Availability::Released => Launch::NotInstalled,
        })
    }

    /// Any function.
    pub fn from_fn(f: impl Fn(AppId) -> Launch + Send + Sync + 'static) -> Self {
        Self {
            open: Arc::new(f),
            calls: Arc::default(),
        }
    }

    /// Opens `app` (and records the call).
    pub fn launch(&self, app: AppId) -> Launch {
        if let Ok(mut c) = self.calls.lock() {
            c.push(app);
        }
        (self.open)(app)
    }

    /// Every app asked for so far, in order.
    pub fn calls(&self) -> Vec<AppId> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

fn system_launch(app: AppId) -> Launch {
    match app.availability() {
        Availability::Planned => return Launch::Planned,
        Availability::Current => return Launch::AlreadyHere,
        Availability::Released => {}
    }
    let Some(path) = locate(app) else {
        return Launch::NotInstalled;
    };
    match std::process::Command::new("open").arg(&path).status() {
        Ok(s) if s.success() => Launch::Opened(path.display().to_string()),
        Ok(s) => Launch::Failed(format!("open exited with {s}")),
        Err(e) => Launch::Failed(e.to_string()),
    }
}

/// Where `app` is installed, if anywhere (see the module docs for the search order).
pub fn locate(app: AppId) -> Option<PathBuf> {
    if let Some(p) = app.bundle_id().and_then(spotlight) {
        return Some(p);
    }
    let bundle = format!("{}.app", app.name());
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    if let Some(p) = dirs.iter().map(|d| d.join(&bundle)).find(|p| p.is_dir()) {
        return Some(p);
    }
    lab_series_roots()
        .iter()
        .find_map(|root| first_app_in(&root.join(app.repo_dir()).join("dist")))
}

/// Spotlight lookup by bundle id (macOS only).
fn spotlight(bundle_id: &str) -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{bundle_id}'"))
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(PathBuf::from)
        .find(|p| p.is_dir())
}

/// The lab-series checkout folders that may hold the siblings: the one this crate was built
/// in, and any ancestor of the running executable that has a sibling repo in it.
fn lab_series_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if let Ok(p) = built.canonicalize() {
        roots.push(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            let is_root = AppId::SIBLINGS
                .iter()
                .any(|a| dir.join(a.repo_dir()).is_dir());
            if is_root && !roots.iter().any(|r| r == dir) {
                roots.push(dir.to_path_buf());
            }
        }
    }
    roots
}

fn first_app_in(dir: &Path) -> Option<PathBuf> {
    let mut apps: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "app") && p.is_dir())
        .collect();
    apps.sort();
    apps.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_and_labels() {
        assert_eq!(AppId::ALL.map(AppId::name)[1], "Chem Playground");
        assert_eq!(
            AppId::Automata.switcher_label(),
            "Automata Playground (current)"
        );
        assert_eq!(
            AppId::Chem.bundle_id(),
            Some("dev.labseries.ChemPlayground")
        );
        assert_eq!(AppId::Bio.bundle_id(), None);
        assert_eq!(AppId::Bio.availability(), Availability::Planned);
        assert_eq!(
            AppId::ALL.map(AppId::tag),
            ["existing", "existing", "planned", "new"]
        );
    }

    #[test]
    fn recording_launcher_records_and_answers() {
        let l = Launcher::recording(&[AppId::Chem]);
        let shared = l.clone();
        assert!(matches!(l.launch(AppId::Chem), Launch::Opened(_)));
        assert_eq!(l.launch(AppId::Breadbox), Launch::NotInstalled);
        assert_eq!(l.launch(AppId::Bio), Launch::Planned);
        assert_eq!(
            shared.calls(),
            [AppId::Chem, AppId::Breadbox, AppId::Bio],
            "clones share the log"
        );
    }

    #[test]
    fn messages() {
        assert_eq!(
            Launch::NotInstalled.message(AppId::Breadbox),
            "Breadbox isn't installed on this Mac"
        );
        assert!(Launch::Planned.message(AppId::Bio).contains("planned"));
    }

    #[test]
    fn system_launcher_never_opens_planned_or_self() {
        // no process is spawned for these
        assert_eq!(system_launch(AppId::Bio), Launch::Planned);
        assert_eq!(system_launch(AppId::Automata), Launch::AlreadyHere);
    }

    #[test]
    fn finds_apps_in_a_dist_folder() {
        let dir = std::env::temp_dir().join(format!("ap-launcher-{}", std::process::id()));
        let app = dir.join("Chem Playground.app");
        std::fs::create_dir_all(&app).unwrap();
        assert_eq!(first_app_in(&dir), Some(app));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(first_app_in(&dir), None);
    }
}
