//! `eidos pack` and `eidos unpack`: one instance, one file.
//!
//! `eidos pack <instance> <file.eidos>`   write the whole instance into one file
//! `eidos unpack <file.eidos> [folder]`   put it back, here or on another machine

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Instant;

use eidos_instance::Instance;
use eidos_transfer::{human_bytes, Options, Transfer, EXTENSION, LEVELS};

use crate::*;

/// A progress line that behaves in a terminal and in a log file.
///
/// A pack of a real instance runs for a quarter of an hour, so silence is not an
/// option; but the same command redirected into a file must not fill it with
/// four thousand repaints. In a terminal it is one line that rewrites itself, and
/// everywhere else one line per ten percent.
///
/// 7-Zip's percentages are NOT monotonic - a second pass restarts the count - so
/// they are clamped here to a running maximum. A bar that goes backwards reads as
/// a bug in the thing being measured.
fn progress_line() -> impl FnMut(u8) {
    let tty = std::io::stdout().is_terminal();
    let mut high = 0u8;
    let mut announced = 0i16 - 1;
    move |p| {
        high = high.max(p.min(100));
        if tty {
            let filled = (high as usize * 40) / 100;
            print!(
                "\r  [{}{}] {high:>3}%",
                "#".repeat(filled),
                " ".repeat(40 - filled)
            );
            let _ = std::io::stdout().flush();
        } else if (high as i16) / 10 > announced {
            announced = (high as i16) / 10;
            println!("  {high}%");
        }
    }
}

/// Close the progress line, if one was drawn.
fn end_progress() {
    if std::io::stdout().is_terminal() {
        println!();
    }
}

/// `1h04m` / `17m04s` / `9s` - a duration as somebody waiting for it would say it.
fn human_secs(secs: u64) -> String {
    match (secs / 3600, (secs % 3600) / 60, secs % 60) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m{s:02}s"),
        (h, m, _) => format!("{h}h{m:02}m"),
    }
}

/// What the flags on `eidos pack` parsed to.
#[derive(Debug)]
struct PackArgs {
    positional: Vec<String>,
    opt: Options,
    dry_run: bool,
}

/// Hand-rolled like the rest of this front end: an unknown flag is a usage
/// error rather than a positional, because `eidos pack skyrimse --forse out.eidos`
/// silently writing a file called `--forse` is not a good afternoon.
fn parse_pack_args(args: &[String]) -> Result<PackArgs, String> {
    let mut out = PackArgs {
        positional: Vec::new(),
        opt: Options::default(),
        dry_run: false,
    };
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let level = |v: &str| -> Result<u8, String> {
            let n: u8 = v
                .parse()
                .map_err(|_| format!("--level wants a number, not '{v}'"))?;
            LEVELS.contains(&n).then_some(n).ok_or_else(|| {
                format!(
                    "--level {n} is not one 7-Zip accepts; use one of {}",
                    LEVELS
                        .iter()
                        .map(u8::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
        };
        match a {
            "--no-downloads" => out.opt.downloads = false,
            "--force" => out.opt.force = true,
            "--dry-run" => out.dry_run = true,
            "--level" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("--level wants a number after it".to_string());
                };
                out.opt.level = level(v)?;
            }
            _ if a.starts_with("--level=") => out.opt.level = level(&a["--level=".len()..])?,
            // Any leading dash, not only two. `-force` is a plausible typo and
            // `-` is not a filename anybody means, so treating either as the
            // destination would write the backup to a file called `-force`.
            _ if a.starts_with('-') => return Err(format!("unknown option '{a}'")),
            _ => out.positional.push(a.to_string()),
        }
        i += 1;
    }
    // Two, and only two. A third is a mistyped flag or a path with an unquoted
    // space in it, and silently packing to the SECOND of three names the user
    // typed is the kind of quiet wrong answer this whole feature must not give.
    if out.positional.len() > 2 {
        return Err(format!(
            "too many arguments (expected an instance and a destination, got {})",
            out.positional.len()
        ));
    }
    Ok(out)
}

/// `<game-id>-<date>.eidos`, for when the destination given is a folder.
fn default_name(game_id: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // `2026-09-07 15:04` -> `2026-09-07-1504`: the same stamp the rest of Eidos
    // shows, in a shape a filesystem and a shell are both happy with.
    let stamp = eidos_instance::format_stamp(now).replace(' ', "-").replace(':', "");
    format!("{game_id}-{stamp}.{EXTENSION}")
}

/// Where the backup actually goes: a folder means "in here, named for me", and a
/// name with no extension at all gets ours.
fn resolve_destination(given: &str, game_id: &str) -> PathBuf {
    let p = expand(given);
    // A trailing slash means a FOLDER, whether or not it exists yet. Without
    // this, `eidos pack skyrimse ~/backups/` on a folder not yet created falls
    // through to the extension test, and `~/backups` gains no extension, so the
    // backup is written as `~/backups/.eidos` - a hidden file, inside a folder
    // the user thought they were naming.
    if p.is_dir() || given.ends_with('/') {
        return p.join(default_name(game_id));
    }
    match p.extension() {
        Some(_) => p,
        None => PathBuf::from(format!("{}.{EXTENSION}", p.display())),
    }
}

fn pack_usage() -> ! {
    eidos_log::info!(
        "usage: eidos pack <instance> <file.eidos> [options]\n\
         \x20 --dry-run        list what would be packed and stop\n\
         \x20 --no-downloads   leave downloads/ out (the archives mods came from)\n\
         \x20 --level <n>      7-Zip compression, one of 0 1 3 5 7 9 (default 1)\n\
         \x20 --force          replace an existing file\n\
         \n\
         The destination may be a folder, in which case the backup is named after\n\
         the game and today's date."
    );
    exit(2);
}

/// `eidos pack <instance> <file.eidos>` - the whole instance in one file.
pub(crate) fn cmd_pack(args: &[String]) {
    let parsed = match parse_pack_args(args) {
        Ok(p) => p,
        Err(e) => {
            eidos_log::info!("eidos pack: {e}");
            pack_usage();
        }
    };
    let (Some(id), Some(given)) = (parsed.positional.first(), parsed.positional.get(1)) else {
        pack_usage();
    };
    let target = resolve(id);
    let inst = target.inst;
    if !inst.exists() {
        eidos_log::info!(
            "eidos pack: '{}' is not an instance folder - there is nothing to pack.",
            inst.root.display()
        );
        exit(1);
    }
    let dest = resolve_destination(given, &target.game_id);

    // BEFORE the lock, deliberately: finding 7-Zip costs up to three fork+exec,
    // and a fork briefly shares every open descriptor - including the instance
    // lock - so a probe made while holding it can be refused by its own child.
    let transfer = match Transfer::new(parsed.opt) {
        Ok(t) => t,
        Err(e) => {
            eidos_log::warn!("eidos pack: {e}");
            exit(1);
        }
    };

    // Packing only reads, but a mod installed or a game saved halfway through
    // lands in the archive as half a mod. The lock is what makes the backup a
    // snapshot rather than a smear.
    let _lock = match inst.try_lock("eidos pack") {
        Ok(l) => l,
        Err(e) => {
            eidos_log::warn!("Cannot pack now: {e}.");
            exit(1);
        }
    };

    println!(
        "Instance: {} ({}, {})",
        inst.root.display(),
        target.game_id,
        match inst.read_manifest().map(|m| m.kind) {
            Some(eidos_instance::InstanceKind::Global) => "central",
            _ => "portable",
        }
    );
    print!("  reading the instance... ");
    let _ = std::io::stdout().flush();
    let scan = Instant::now();
    let plan = eidos_transfer::plan(&inst, &parsed.opt);
    println!("{}", human_secs(scan.elapsed().as_secs()));
    print_plan(&plan, &dest, parsed.opt.level);

    if parsed.dry_run {
        // The same checks the real run makes, so the preview is a preview and
        // not an optimistic sketch of one.
        if let Err(e) = transfer.check_pack(&inst, &plan, &dest) {
            eidos_log::warn!("eidos pack would refuse: {e}");
            exit(1);
        }
        println!("\n(dry run - nothing written; drop --dry-run to make the backup)");
        return;
    }

    let started = Instant::now();
    let mut on_progress = progress_line();
    match transfer.pack(&inst, &plan, &dest, &mut on_progress) {
        Ok(r) => {
            end_progress();
            let pct = r
                .percent_of_source()
                .map(|p| format!(", {p:.0}% of {}", human_bytes(r.source_bytes)))
                .unwrap_or_default();
            println!(
                "Packed {} entries into {} ({}{pct}) in {}.",
                r.entries,
                r.path.display(),
                human_bytes(r.bytes),
                human_secs(started.elapsed().as_secs())
            );
            println!("On the other machine: eidos unpack {} <folder>", r.path.display());
            // An incomplete backup is not a success, and this is a command
            // people put in front of `&&`. The archive is real and worth
            // keeping - it is named above - but the exit code has to say that
            // something did not make it in, or `eidos pack ... && rm -rf <old>`
            // proceeds on a short archive. 7-Zip's own convention, and this
            // program's: 1 means the operation did not fully succeed.
            if !r.warnings.is_empty() {
                println!();
                for w in &r.warnings {
                    println!("WARNING: {w}");
                }
                println!(
                    "The backup was written but it is NOT complete. Fix what is above and \
                     pack again before you rely on it."
                );
                exit(1);
            }
        }
        Err(e) => {
            end_progress();
            eidos_log::warn!("eidos pack: {e}");
            exit(1);
        }
    }
}

/// The part of the output `--dry-run` and a real run share, so the preview is
/// the same text as the thing it previews.
fn print_plan(plan: &eidos_transfer::Plan, dest: &Path, level: u8) {
    println!(
        "  {} file(s), {} folder(s) ({} empty), {}",
        plan.files,
        plan.dirs,
        plan.empty_dirs,
        human_bytes(plan.bytes)
    );
    if plan.downloads_bytes > 0 {
        println!(
            "  of which downloads/: {} (--no-downloads leaves them out)",
            human_bytes(plan.downloads_bytes)
        );
    }
    println!("  -> {} (LZMA2 -mx{level}, non-solid)", dest.display());
    if !plan.left.is_empty() {
        println!("Left out:");
        for l in plan.left.iter().take(20) {
            println!("  {:<28} {}", l.path, l.why);
        }
        if plan.left.len() > 20 {
            println!("  ... and {} more", plan.left.len() - 20);
        }
    }
    if !plan.scrub.is_empty() {
        println!(
            "{} download record(s) go in with their signed download URL removed \
             (it has expired, and it names the account that downloaded the file).",
            plan.scrub.len()
        );
    }
    if !plan.tools_outside.is_empty() {
        println!(
            "{} tool(s) live outside the instance. They are named in the backup so the \
             other machine knows what to install, but they are not in it:",
            plan.tools_outside.len()
        );
        for t in &plan.tools_outside {
            println!("  {:<28} {}", t.title, t.exe);
        }
    }
    if let Some(free) = eidos_transfer::free_bytes(&eidos_transfer::existing_ancestor(dest)) {
        println!("Free where it will be written: {}", human_bytes(free));
    }
}

fn unpack_usage() -> ! {
    eidos_log::info!(
        "usage: eidos unpack <file.eidos> [folder] [--force]\n\
         \x20 --info    show what the backup holds and stop\n\
         \x20 --force   unpack into a folder that is not empty\n\
         \n\
         With no folder, a backup of a central instance goes back to the central\n\
         location for its game; a portable one needs a folder."
    );
    exit(2);
}

/// `eidos unpack <file.eidos> [folder]` - put a backup back.
pub(crate) fn cmd_unpack(args: &[String]) {
    let info = args.iter().any(|a| a == "--info");
    let force = args.iter().any(|a| a == "--force");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if let Some(bad) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--info" && *a != "--force")
    {
        eidos_log::info!("eidos unpack: unknown option '{bad}'");
        unpack_usage();
    }
    let Some(archive) = positional.first() else {
        unpack_usage();
    };
    if positional.len() > 2 {
        eidos_log::info!(
            "eidos unpack: too many arguments (expected a backup file and a folder, got {}).",
            positional.len()
        );
        unpack_usage();
    }
    let archive = expand(archive);

    let opt = Options {
        force,
        ..Options::default()
    };
    let transfer = match Transfer::new(opt) {
        Ok(t) => t,
        Err(e) => {
            eidos_log::warn!("eidos unpack: {e}");
            exit(1);
        }
    };
    let manifest = match transfer.peek(&archive) {
        Ok(m) => m,
        Err(e) => {
            eidos_log::warn!("eidos unpack: {e}");
            exit(1);
        }
    };

    println!("{}", archive.display());
    println!(
        "  {} instance, made {} by Eidos {}",
        manifest.game_id, manifest.created, manifest.eidos_version
    );
    println!(
        "  {} file(s), {} folder(s), {} unpacked",
        manifest.files,
        manifest.directories,
        human_bytes(manifest.bytes)
    );
    println!("  was at: {}", manifest.source_root);
    println!(
        "  profile(s): {} (active: {})",
        manifest.profiles.join(", "),
        manifest.active_profile
    );
    if !manifest.downloads {
        println!("  downloads/ was left out of this backup");
    }
    if !manifest.tools_outside.is_empty() {
        println!("  tools it expects to find on this machine:");
        for t in &manifest.tools_outside {
            let here = if Path::new(&t.exe).exists() {
                "found"
            } else {
                "MISSING"
            };
            println!("    {:<28} {} ({here})", t.title, t.exe);
        }
    }
    if info {
        if !manifest.left_out.is_empty() {
            println!("  left out of the backup:");
            for (path, why) in manifest.left_out.iter().take(20) {
                println!("    {path:<28} {why}");
            }
            if manifest.left_out_more > 0 {
                println!("    ... and {} more", manifest.left_out_more);
            }
        }
        return;
    }

    // No folder given: a central instance knows where it belongs. A portable one
    // does not, and guessing would write somebody's 70 GB somewhere they did not
    // choose.
    let dest = match positional.get(1) {
        Some(folder) => expand(folder),
        None if !manifest.portable && !manifest.game_id.is_empty() => {
            let d = Instance::global(&manifest.game_id).root;
            println!("No folder given; this is a backup of the central instance, so:");
            println!("  -> {}", d.display());
            d
        }
        None => {
            eidos_log::info!(
                "eidos unpack: this is a backup of a PORTABLE instance, so it needs a \
                 folder to go into."
            );
            unpack_usage();
        }
    };

    // If the folder is already there, somebody may have it open - and with
    // --force this would write over a live instance. Nothing can hold a folder
    // that does not exist yet, and taking the lock would CREATE it, so ask only
    // when there is something to ask about.
    let _lock = if dest.is_dir() {
        match Instance::portable(dest.clone()).try_lock("eidos unpack") {
            Ok(l) => Some(l),
            Err(e) => {
                eidos_log::warn!("Cannot unpack there: {e}.");
                exit(1);
            }
        }
    } else {
        None
    };

    let started = Instant::now();
    let mut on_progress = progress_line();
    match transfer.unpack(&archive, &dest, &mut on_progress) {
        Ok(r) => {
            end_progress();
            let inst = Instance::portable(r.root.clone());
            println!(
                "Unpacked {} entries into {} in {}.",
                r.entries,
                r.root.display(),
                human_secs(started.elapsed().as_secs())
            );
            let mods = inst.modlist().len();
            if mods > 0 {
                println!("  {mods} mod(s), profile '{}'.", inst.active_profile());
            }
            if r.relocated.values > 0 {
                println!(
                    "  {} path(s) in {} file(s) now point here instead of {}.",
                    r.relocated.values,
                    r.relocated.files.len(),
                    r.manifest.source_root
                );
            }
            for w in &r.warnings {
                eidos_log::warn!("  {w}");
            }
            if !r.missing_tools.is_empty() {
                println!("  Tools this instance uses that are not on this machine:");
                for t in &r.missing_tools {
                    println!("    {:<28} was at {}", t.title, t.exe);
                }
                println!(
                    "  Install them, then fix their paths in the Tools tab (or {}/tools.ini).",
                    r.root.display()
                );
            }
            remember_use(&inst, &r.manifest.game_id);
            println!("\nNext:");
            println!(
                "  1. Install the game through Steam and launch it once, if you have not."
            );
            println!(
                "  2. eidos prereqs {} --install    (rebuild the Proton prefix this backup \
                 deliberately left behind)",
                r.root.display()
            );
            println!("  3. eidos play {}", r.root.display());
        }
        Err(e) => {
            end_progress();
            eidos_log::warn!("eidos unpack: {e}");
            exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_flags_value_is_never_mistaken_for_the_destination() {
        // `--level 5` puts a bare `5` on the command line. A positional filter
        // that only skipped `--` arguments would pack the instance into a file
        // called `5`.
        let p = parse_pack_args(&v(&["skyrimse", "--level", "5", "out.eidos"])).unwrap();
        assert_eq!(p.positional, v(&["skyrimse", "out.eidos"]));
        assert_eq!(p.opt.level, 5);
        let p = parse_pack_args(&v(&["--level=9", "skyrimse", "out.eidos"])).unwrap();
        assert_eq!(p.positional, v(&["skyrimse", "out.eidos"]));
        assert_eq!(p.opt.level, 9);
    }

    #[test]
    fn a_mistyped_flag_is_a_usage_error_not_a_filename() {
        assert!(parse_pack_args(&v(&["skyrimse", "--forse", "out.eidos"])).is_err());
        assert!(parse_pack_args(&v(&["skyrimse", "--level", "4"])).is_err());
        assert!(parse_pack_args(&v(&["skyrimse", "--level", "x"])).is_err());
        assert!(parse_pack_args(&v(&["skyrimse", "--level"])).is_err());
    }

    #[test]
    fn a_single_dash_is_an_option_not_a_filename() {
        // `-force` would otherwise become the destination, and the backup would
        // be written to a file called `-force`.
        assert!(parse_pack_args(&v(&["skyrimse", "-force", "out.eidos"])).is_err());
        assert!(parse_pack_args(&v(&["skyrimse", "-", "out.eidos"])).is_err());
    }

    #[test]
    fn a_third_positional_is_refused_rather_than_ignored() {
        // An unquoted space in a path is the usual cause, and quietly packing to
        // the second of three names is a wrong answer given in silence.
        let e = parse_pack_args(&v(&["skyrimse", "my", "backup.eidos"])).unwrap_err();
        assert!(e.contains("too many"), "{e}");
    }

    #[test]
    fn a_trailing_slash_names_a_folder_even_before_it_exists() {
        // Otherwise the backup lands as a HIDDEN `.eidos` file inside it.
        let chosen = resolve_destination("/tmp/eidos-no-such-dir-xyz/", "skyrimse");
        assert_eq!(chosen.parent().unwrap(), Path::new("/tmp/eidos-no-such-dir-xyz"));
        assert!(chosen
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("skyrimse-"));
    }

    #[test]
    fn the_flags_that_change_what_is_packed() {
        let p = parse_pack_args(&v(&["skyrimse", "out.eidos"])).unwrap();
        assert!(p.opt.downloads && !p.opt.force && !p.dry_run);
        assert_eq!(p.opt.level, eidos_transfer::DEFAULT_LEVEL);
        let p = parse_pack_args(&v(&["skyrimse", "o", "--no-downloads", "--force", "--dry-run"]))
            .unwrap();
        assert!(!p.opt.downloads && p.opt.force && p.dry_run);
    }

    #[test]
    fn a_name_without_an_extension_gets_ours_and_a_folder_gets_a_dated_name() {
        let dir = std::env::temp_dir();
        let chosen = resolve_destination(&dir.display().to_string(), "skyrimse");
        let name = chosen.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("skyrimse-"), "{name}");
        assert!(name.ends_with(".eidos"), "{name}");
        assert_eq!(chosen.parent().unwrap(), dir);
        // No extension at all: ours. A different one: theirs.
        assert_eq!(
            resolve_destination("/tmp/eidos-transfer-cli-no-such/backup", "skyrimse"),
            PathBuf::from("/tmp/eidos-transfer-cli-no-such/backup.eidos")
        );
        assert_eq!(
            resolve_destination("/tmp/eidos-transfer-cli-no-such/backup.7z", "skyrimse"),
            PathBuf::from("/tmp/eidos-transfer-cli-no-such/backup.7z")
        );
    }

    #[test]
    fn a_wait_is_reported_the_way_somebody_waiting_would_say_it() {
        assert_eq!(human_secs(9), "9s");
        assert_eq!(human_secs(64), "1m04s");
        assert_eq!(human_secs(1024), "17m04s");
        assert_eq!(human_secs(3864), "1h04m");
    }
}
