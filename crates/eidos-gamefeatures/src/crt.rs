//! Detecting a split Visual C++ runtime in a Proton prefix.

use std::path::Path;

/// One dependency edge whose two halves came from different implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    /// `system32` or `syswow64`.
    pub arch: &'static str,
    /// The DLL that calls into `provider`.
    pub consumer: String,
    /// The DLL it delegates to.
    pub provider: String,
}

/// What a prefix's Visual C++ runtime looks like next to Proton's own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub splits: Vec<Split>,
}

/// Proton's own untouched prefix, the reference every check compares against.
/// `proton_script` is the `proton` entry script (`ProtonRun::proton`). `None`
/// when that layout is not there, so a non-Proton runner is simply not checked.
pub fn reference_windows(proton_script: &Path) -> Option<std::path::PathBuf> {
    let win = proton_script
        .parent()?
        .join("files/share/default_pfx/drive_c/windows");
    win.is_dir().then_some(win)
}

/// The Proton entry script inside a Steam `%command%` argv.
///
/// The `eidos play` path never resolves Proton itself - Steam hands it the whole
/// command - so the reference prefix has to be recovered from the argv.
pub fn proton_script_in(command: &[String]) -> Option<&Path> {
    command
        .iter()
        .map(Path::new)
        .find(|p| p.file_name().is_some_and(|n| n == "proton"))
}

/// The two Windows system directories a 64-bit prefix carries.
const ARCHES: [&str; 2] = ["system32", "syswow64"];

/// DLL pairs where one half calls into the other, so both must come from the
/// same implementation.
const PAIRS: &[(&str, &str)] = &[
    // Wine's `vcruntime140_1` implements `__CxxFrameHandler4` - the unwinder every
    // plugin built with a modern MSVC uses - and hands the exception bookkeeping
    // back to `vcruntime140` (`__CxxRegisterExceptionObject`, `__current_exception`,
    // `__processing_throw`, ...). The names resolve against any implementation, so
    // nothing fails at load; the structures behind them are not interchangeable.
    ("vcruntime140_1.dll", "vcruntime140.dll"),
    // Same shape, one type over: `_Thrd_hardware_concurrency`.
    ("msvcp140_atomic_wait.dll", "msvcp140.dll"),
];

/// Whether `dir/name` exists and is byte-identical to `reference/name`.
/// `None` when either side is absent, which is not a judgement we can make.
fn matches_reference(dir: &Path, reference: &Path, name: &str) -> Option<bool> {
    let a = std::fs::read(dir.join(name)).ok()?;
    let b = std::fs::read(reference.join(name)).ok()?;
    Some(a == b)
}

/// Compare a prefix's `drive_c/windows` against the reference one.
pub fn check(prefix_windows: &Path, reference_windows: &Path) -> Report {
    let mut splits = Vec::new();
    for arch in ARCHES {
        let (pfx, reference) = (prefix_windows.join(arch), reference_windows.join(arch));
        for (consumer, provider) in PAIRS {
            let c = matches_reference(&pfx, &reference, consumer);
            let p = matches_reference(&pfx, &reference, provider);
            if let (Some(c), Some(p)) = (c, p) {
                if c != p {
                    splits.push(Split {
                        arch,
                        consumer: (*consumer).to_string(),
                        provider: (*provider).to_string(),
                    });
                }
            }
        }
    }
    Report { splits }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "eidos-crt-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(root: &Path, arch: &str, name: &str, body: &str) {
        let d = root.join(arch);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(name), body).unwrap();
    }

    /// The exact shape that broke Skyrim: Wine's `vcruntime140_1` (which owns
    /// `__CxxFrameHandler4`) delegating its exception bookkeeping to a foreign
    /// `vcruntime140`.
    #[test]
    fn a_runtime_whose_two_halves_disagree_is_reported_as_split() {
        let (pfx, reference) = (tmp(), tmp());
        put(&reference, "system32", "vcruntime140.dll", "wine builtin");
        put(&reference, "system32", "vcruntime140_1.dll", "wine builtin _1");
        // The prefix kept Proton's `_1` but had `vcruntime140` replaced.
        put(&pfx, "system32", "vcruntime140.dll", "microsoft 2015");
        put(&pfx, "system32", "vcruntime140_1.dll", "wine builtin _1");

        let report = check(&pfx, &reference);

        assert_eq!(
            report.splits,
            vec![Split {
                arch: "system32",
                consumer: "vcruntime140_1.dll".into(),
                provider: "vcruntime140.dll".into(),
            }]
        );
    }

    /// The other edge found in the same prefix: Wine's `msvcp140_atomic_wait`
    /// imports `_Thrd_hardware_concurrency` from `msvcp140`.
    #[test]
    fn the_atomic_wait_shim_and_its_msvcp_must_also_agree() {
        let (pfx, reference) = (tmp(), tmp());
        put(&reference, "system32", "msvcp140.dll", "wine builtin");
        put(&reference, "system32", "msvcp140_atomic_wait.dll", "wine builtin aw");
        put(&pfx, "system32", "msvcp140.dll", "microsoft 2015");
        put(&pfx, "system32", "msvcp140_atomic_wait.dll", "wine builtin aw");

        let report = check(&pfx, &reference);

        assert_eq!(report.splits.len(), 1, "{report:?}");
        assert_eq!(report.splits[0].consumer, "msvcp140_atomic_wait.dll");
        assert_eq!(report.splits[0].provider, "msvcp140.dll");
    }

    /// The `vcrun2022` case, and the reason this checks pairs rather than simply
    /// flagging anything foreign: a runtime replaced WHOLE is consistent with
    /// itself. Noise here would train people to ignore the warning.
    #[test]
    fn a_runtime_replaced_whole_is_consistent_and_stays_quiet() {
        let (pfx, reference) = (tmp(), tmp());
        for (name, body) in [
            ("vcruntime140.dll", "wine builtin"),
            ("vcruntime140_1.dll", "wine builtin _1"),
            ("msvcp140.dll", "wine builtin p"),
            ("msvcp140_atomic_wait.dll", "wine builtin aw"),
        ] {
            put(&reference, "system32", name, body);
            put(&pfx, "system32", name, "microsoft 2022");
        }

        assert_eq!(check(&pfx, &reference).splits, vec![]);
    }

    /// And the untouched prefix, the overwhelmingly common case.
    #[test]
    fn an_untouched_prefix_stays_quiet() {
        let (pfx, reference) = (tmp(), tmp());
        for (name, body) in [
            ("vcruntime140.dll", "wine builtin"),
            ("vcruntime140_1.dll", "wine builtin _1"),
        ] {
            put(&reference, "system32", name, body);
            put(&pfx, "system32", name, body);
        }

        assert_eq!(check(&pfx, &reference).splits, vec![]);
    }

    /// Verbatim shape of a real Steam launch option, reaper and runtime wrappers
    /// included.
    #[test]
    fn the_proton_script_is_found_inside_a_steam_command() {
        let command: Vec<String> = [
            "/home/u/.steam/ubuntu12_32/reaper",
            "SteamLaunch",
            "AppId=489830",
            "--",
            "/games/SteamLinuxRuntime_4/_v2-entry-point",
            "--verb=waitforexitandrun",
            "--",
            "/home/u/.steam/compatibilitytools.d/proton-cachyos-11.0/proton",
            "waitforexitandrun",
            "/games/Skyrim Special Edition/skse64_loader.exe",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert_eq!(
            proton_script_in(&command),
            Some(Path::new(
                "/home/u/.steam/compatibilitytools.d/proton-cachyos-11.0/proton"
            ))
        );
    }

    /// A command with no Proton at all (a plain `eidos play id -- ls`) is not a
    /// launch we can judge.
    #[test]
    fn a_command_without_proton_yields_nothing() {
        let command: Vec<String> = ["ls", "/tmp"].iter().map(|s| s.to_string()).collect();
        assert_eq!(proton_script_in(&command), None);
    }

    #[test]
    fn the_reference_is_protons_own_default_prefix() {
        let root = tmp();
        let win = root.join("files/share/default_pfx/drive_c/windows");
        std::fs::create_dir_all(win.join("system32")).unwrap();
        std::fs::write(root.join("proton"), "#!/usr/bin/env python3\n").unwrap();

        assert_eq!(reference_windows(&root.join("proton")), Some(win));
    }

    /// A runner with no `default_pfx` is not a Proton we can judge, and guessing
    /// would be worse than staying quiet.
    #[test]
    fn a_runner_without_a_default_prefix_is_not_checked() {
        let root = tmp();
        std::fs::write(root.join("proton"), "#!/bin/sh\n").unwrap();

        assert_eq!(reference_windows(&root.join("proton")), None);
    }
}
