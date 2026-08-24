//! How a subcommand's `<game-id-or-path>` argument becomes an instance.
//!
//! Every command used to hardcode `Instance::global(id)`, which made portable
//! instances unreachable from the terminal: create one in the GUI and `eidos
//! play` still mounted the XDG-global folder. Now the positional argument may
//! be either form:
//!
//! - a game id (`skyrimse`) - the central instance, as before;
//! - a path (`/mnt/games/EidosSkyrim`, `~/Eidos/skyrimse`, `.`) - a portable
//!   instance, whose `eidos-instance.ini` names the game.
//!
//! The two are distinguished syntactically: game ids never contain `/` and
//! never start with `~` or `.`, so a bare id can never be shadowed by a
//! same-named directory in the CWD.
//!
//! `EIDOS_INSTANCE=<root>` redirects a game-id argument to that root instead -
//! the transport the GUI uses to make its `eidos play <id>` child act on the
//! portable instance the user has open, and handy in Steam launch options. An
//! explicit path argument still wins over the variable: what the user typed
//! last is what they mean.

use std::path::PathBuf;
use std::process::exit;

use eidos_instance::Instance;

/// A resolved target: the instance to operate on and the game it mods.
// Debug is for the tests' `unwrap_err` on `Result<Target, _>`.
#[derive(Debug)]
pub(crate) struct Target {
    pub inst: Instance,
    pub game_id: String,
}

/// Whether the argument is a path rather than a game id.
pub(crate) fn looks_like_path(arg: &str) -> bool {
    arg.contains('/') || arg.starts_with('~') || arg == "." || arg == ".."
}

/// `~` expanded, relative paths anchored at the CWD (this is a terminal;
/// `eidos play .` from inside the instance folder should just work).
pub(crate) fn expand(arg: &str) -> PathBuf {
    let home = || {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
    };
    let p = if arg == "~" {
        home()
    } else if let Some(rest) = arg.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(arg)
    };
    // Canonical when possible so the registry never holds two spellings of one
    // folder; the raw absolute path when not (open_at will say why).
    std::fs::canonicalize(&p).unwrap_or_else(|_| {
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir().map(|c| c.join(&p)).unwrap_or(p)
        }
    })
}

/// Resolve, or exit with the reason - callers are subcommands, and a target
/// that cannot be resolved has nothing to fall back to.
pub(crate) fn resolve(arg: &str) -> Target {
    match try_resolve(arg) {
        Ok(t) => t,
        Err(e) => {
            eidos_log::info!("eidos: {e}");
            exit(1);
        }
    }
}

fn try_resolve(arg: &str) -> Result<Target, String> {
    try_resolve_from(arg, std::env::var_os("EIDOS_INSTANCE"))
}

/// The resolution behind [`try_resolve`], with the environment injected so it
/// can be tested without mutating the process env under parallel tests.
fn try_resolve_from(arg: &str, env_instance: Option<std::ffi::OsString>) -> Result<Target, String> {
    if looks_like_path(arg) {
        let root = expand(arg);
        let (inst, m) = Instance::open_at(&root)?;
        return Ok(Target { inst, game_id: m.game_id });
    }
    if let Some(root) = env_instance.filter(|v| !v.is_empty()) {
        let root = expand(&root.to_string_lossy());
        let (inst, m) = Instance::open_at(&root)
            .map_err(|e| format!("EIDOS_INSTANCE: {e}"))?;
        // The variable redirects the id, it must not overrule it: acting on a
        // Fallout folder because a Skyrim command ran under a stale variable
        // is the kind of surprise that costs a mod list.
        if m.game_id != arg {
            return Err(format!(
                "EIDOS_INSTANCE points at a '{}' instance ({}), but the command names '{arg}'. \
                 Unset the variable or pass the instance path directly.",
                m.game_id,
                root.display()
            ));
        }
        return Ok(Target { inst, game_id: arg.to_string() });
    }
    Ok(Target { inst: Instance::global(arg), game_id: arg.to_string() })
}

/// Record that an instance was just USED, so the GUI's welcome screen and the
/// `nxm://` handler follow the user to it. Registry trouble is never a reason
/// to fail the command that did the actual work.
pub(crate) fn remember_use(inst: &Instance, game_id: &str) {
    let mut reg = eidos_instance::Registry::load();
    let r = if inst.root == Instance::global(game_id).root {
        eidos_instance::InstanceRef::Global(game_id.to_string())
    } else {
        eidos_instance::InstanceRef::Portable(inst.root.clone())
    };
    reg.set_last(r);
    let _ = reg.save();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_paths_are_told_apart_syntactically() {
        for id in ["skyrimse", "fallout4", "skyrimvr"] {
            assert!(!looks_like_path(id), "{id} is an id");
        }
        for p in ["/abs/x", "rel/x", "~/Eidos", "~", ".", "..", "./x"] {
            assert!(looks_like_path(p), "{p} is a path");
        }
    }

    #[test]
    fn a_game_id_resolves_to_the_global_instance() {
        let t = try_resolve_from("skyrimse", None).unwrap();
        assert_eq!(t.inst.root, Instance::global("skyrimse").root);
        assert_eq!(t.game_id, "skyrimse");
    }

    #[test]
    fn a_path_argument_requires_a_self_describing_folder() {
        let root = std::env::temp_dir().join(format!("eidos-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // A bare directory: not an instance.
        let err = try_resolve_from(&root.display().to_string(), None).unwrap_err();
        assert!(err.contains("not an Eidos instance"), "{err}");
        // With a manifest: resolves, and the game id comes from the manifest.
        eidos_instance::Manifest::new("skyrimse", eidos_instance::InstanceKind::Portable)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        let t = try_resolve_from(&root.display().to_string(), None).unwrap();
        assert_eq!(t.game_id, "skyrimse");
        assert_eq!(t.inst.root, std::fs::canonicalize(&root).unwrap());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_env_var_redirects_an_id_but_may_not_overrule_it() {
        let root = std::env::temp_dir().join(format!("eidos-resolve-env-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        eidos_instance::Manifest::new("skyrimse", eidos_instance::InstanceKind::Portable)
            .write(&root.join("eidos-instance.ini"))
            .unwrap();
        let env = Some(std::ffi::OsString::from(root.display().to_string()));
        // Matching id: the id is served by the redirected root.
        let t = try_resolve_from("skyrimse", env.clone()).unwrap();
        assert_eq!(t.inst.root, std::fs::canonicalize(&root).unwrap());
        // Mismatched id: refuse loudly rather than act on the wrong game.
        let err = try_resolve_from("fallout4", env).unwrap_err();
        assert!(err.contains("skyrimse") && err.contains("fallout4"), "{err}");
        // An explicit path argument ignores the variable entirely.
        let other = std::env::temp_dir().join(format!("eidos-resolve-env2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&other);
        std::fs::create_dir_all(&other).unwrap();
        eidos_instance::Manifest::new("fallout4", eidos_instance::InstanceKind::Portable)
            .write(&other.join("eidos-instance.ini"))
            .unwrap();
        let t = try_resolve_from(
            &other.display().to_string(),
            Some(std::ffi::OsString::from(root.display().to_string())),
        )
        .unwrap();
        assert_eq!(t.game_id, "fallout4", "the typed path wins over the variable");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&other);
    }
}
