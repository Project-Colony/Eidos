//! `eidos nxm`: the nxm:// handler - registration and incoming links.

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::Instance;

use crate::*;

/// The instance a browser-initiated download should land in. The browser
/// carries no instance context at all - this process is spawned by
/// xdg-open - so the answer comes from the registry: the LAST instance the
/// user actually used, when it belongs to one of the candidate games, then
/// each candidate's known instances (portables first, per the registry's
/// preference order), then the first candidate's global path as the
/// create-on-demand fallback. Before the registry existed this hardwired
/// `Instance::global`, which sent every download to the XDG folder no matter
/// which portable instance the user was playing.
fn pick_instance<'a>(candidates: &[&'a DetectedGame]) -> (&'a DetectedGame, Instance) {
    let reg = eidos_instance::Registry::load();
    if let Some(last) = &reg.last {
        let inst = last.instance();
        if inst.exists() {
            let gid = match last {
                eidos_instance::InstanceRef::Global(id) => Some(id.clone()),
                eidos_instance::InstanceRef::Portable(_) => inst.read_manifest().map(|m| m.game_id),
            };
            if let Some(game) = gid.and_then(|gid| candidates.iter().find(|g| g.def.id == gid)) {
                return (game, inst);
            }
        }
    }
    for g in candidates {
        if let Some(inst) = reg.candidates_for(g.def.id).into_iter().find(|i| i.exists()) {
            return (g, inst);
        }
    }
    let first = candidates[0];
    (first, Instance::global(first.def.id))
}

/// `eidos nxm <url>` - download a "Mod Manager Download" link into the game's
/// downloads dir (with its MO2-format .meta). `--register` installs the
/// x-scheme-handler so the site's button opens Eidos.
pub(crate) fn cmd_nxm(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("--resume") => {
            let Some(archive) = args.get(1).map(std::path::PathBuf::from) else {
                eidos_log::info!("usage: eidos nxm --resume <downloads/archive.7z>");
                exit(2);
            };
            resume(&archive);
        }
        Some("--register") => {
            let exe = std::env::current_exe().unwrap_or_else(|_| "eidos".into());
            let apps = home().join(".local/share/applications");
            let _ = std::fs::create_dir_all(&apps);
            let desktop = apps.join("eidos-nxm.desktop");
            let body = format!(
                "[Desktop Entry]\nType=Application\nName=Eidos (Nexus nxm handler)\n\
                 Exec={} nxm %u\nMimeType=x-scheme-handler/nxm;\nNoDisplay=true\nTerminal=false\n",
                exe.display()
            );
            if let Err(e) = std::fs::write(&desktop, body) {
                eidos_log::warn!("could not write {}: {e}", desktop.display());
                exit(1);
            }
            let _ = std::process::Command::new("xdg-mime")
                .args(["default", "eidos-nxm.desktop", "x-scheme-handler/nxm"])
                .status();
            println!(
                "Registered {} for nxm:// links.\nThe site's \"Mod Manager Download\" button now downloads through Eidos.",
                desktop.display()
            );
        }
        Some(url) => {
            let nxm = match eidos_nexus::NxmLink::parse(url) {
                Ok(eidos_nexus::NxmLink::Mod(n)) => n,
                Ok(eidos_nexus::NxmLink::Collection(c)) => {
                    // A collection is not something this process installs, so it
                    // is handed to the window - which is where the mod list it
                    // has to be compared against lives.
                    //
                    // The INSTANCE is resolved here, by the same domain match a
                    // mod link gets. Launching the window bare opened whatever
                    // instance was last used, so a Skyrim collection clicked
                    // while Fallout 4 was the last instance came up compared
                    // against Fallout 4's mods - every "installed" and every
                    // "missing" wrong, and wrong in a way that looks like an
                    // answer.
                    let games = detect(&home());
                    let candidates: Vec<&DetectedGame> = games
                        .iter()
                        .filter(|g| g.def.nexus_game.eq_ignore_ascii_case(&c.game))
                        .collect();
                    if candidates.is_empty() {
                        eidos_log::info!(
                            "That collection is for '{}', which is not a game Eidos found here.",
                            c.game
                        );
                        exit(1);
                    }
                    let (game, inst) = pick_instance(&candidates);
                    // Created before the pin is handed over: the window ignores
                    // `EIDOS_INSTANCE` when the folder has no manifest, and a
                    // first-ever collection link for a game whose instance does
                    // not exist yet would then fall back to whatever was last
                    // open - the very mismatch this branch exists to prevent.
                    inst.create().ok();
                    println!(
                        "Collection {} (revision {}) for {} - opening it in Eidos.",
                        c.slug,
                        c.revision.map_or_else(|| "latest".to_string(), |r| r.to_string()),
                        game.def.name
                    );
                    show_collection_in_gui(url, &inst);
                    return;
                }
                Err(e) => {
                    eidos_log::info!("bad nxm link: {e}");
                    exit(1);
                }
            };
            // Which detected game does this link belong to? (VR editions share
            // their parent's Nexus.)
            let games = detect(&home());
            let candidates: Vec<&DetectedGame> = games
                .iter()
                .filter(|g| g.def.nexus_game.eq_ignore_ascii_case(&nxm.game))
                .collect();
            if candidates.is_empty() {
                eidos_log::info!("No detected game matches the Nexus domain '{}'.", nxm.game);
                exit(1);
            }
            let (game, inst) = pick_instance(&candidates);
            inst.create().ok();
            // How the follow-up `eidos install` should NAME this instance: the
            // id when it is the global one, the folder when it is portable.
            let inst_arg = if inst.root == Instance::global(game.def.id).root {
                game.def.id.to_string()
            } else {
                inst.root.display().to_string()
            };

            let nexus = nexus_client();
            // The MOD is looked up first, before its file. That ordering is not
            // stylistic: the lookup is what resolves the mod's content rating,
            // and the token it returns is what the two calls below require. A
            // mod Eidos may not describe is one it does not fetch files for.
            let remote_mod = match nexus.mod_info(&nxm.game, nxm.mod_id) {
                Ok(m) => m,
                Err(e) => {
                    eidos_log::warn!("mod lookup failed: {e}");
                    exit(1);
                }
            };
            if let Some(why) = remote_mod.hidden() {
                eidos_log::info!("{}", why.message());
                exit(1);
            }
            let file = match nexus.file_info(&remote_mod.gate, &nxm.game, nxm.mod_id, nxm.file_id) {
                Ok(f) => f,
                Err(e) => {
                    eidos_log::warn!("file lookup failed: {e}");
                    exit(1);
                }
            };
            let link = match nexus.download_link(&remote_mod.gate, &nxm) {
                Ok(l) => l,
                Err(e) => {
                    eidos_log::warn!("could not resolve the download: {e}");
                    exit(1);
                }
            };
            // The API's `file_name` FIRST - it is the authoritative answer to
            // "what is this archive called", and the CDN URI is not. Nexus serves
            // supporter downloads from a path whose last segment is a bare UUID
            // (`supporter-files.nexus-cdn.com/66/2a/35/662a3503-...`), which parses
            // perfectly well as a name, so preferring the URI meant every premium
            // download landed as an unreadable UUID with no extension.
            let name = eidos_nexus::sanitize_file_name(&file.file_name)
                .or_else(|| {
                    // Only trust the URI when it actually looks like a file. An
                    // archive always has an extension; a segment without one is a
                    // CDN object id, not a name.
                    eidos_nexus::file_name_from_uri(&link).filter(|n| n.contains('.'))
                })
                .unwrap_or_else(|| format!("{}-{}.archive", nxm.mod_id, nxm.file_id));
            let downloads = inst.downloads_dir();
            // An IN-FLIGHT download of the same name is not a free name either.
            // The guard below only ever looked at the finished archive, so a
            // second `eidos nxm` for a file already arriving opened the same
            // partial and the same sidecar: two byte streams appended into one
            // file, and the first process's pid erased from under it.
            if eidos_nexus::live_download_pid(&downloads.join(&name)).is_some() {
                println!("Already downloading: {}", downloads.join(&name).display());
                return;
            }
            // Don't silently clobber an existing download. If the very same file is
            // already here (its .meta carries this fileID), stop; otherwise give the
            // new download a unique `<i>_<name>` (MO2's getDownloadFileName).
            let existing = downloads.join(&name);
            if existing.is_file() {
                let meta = std::fs::read_to_string(format!("{}.meta", existing.display())).unwrap_or_default();
                if meta.contains(&format!("fileID={}", nxm.file_id)) {
                    println!("Already downloaded: {}", existing.display());
                    println!("Install it:  eidos install \"{inst_arg}\" \"{}\"", existing.display());
                    return;
                }
            }
            let name = eidos_nexus::unique_download_name(&downloads, &name);
            let dest = downloads.join(&name);

            // The sidecar goes down BEFORE the first byte, not after the last.
            //
            // The transfer runs in this process, launched from the browser; the
            // window is somewhere else entirely. The `.meta` is the only thing
            // that tells it a download exists at all - written afterwards, a mod
            // was invisible for the whole minute it took to arrive and then
            // appeared finished, which is precisely backwards from what a
            // download manager is for. Written first, the row shows up at 0%
            // and the `.unfinished` partial beside it carries the progress.
            //
            // If the download then fails, the sidecar and the partial are both
            // left in place: that pair IS a paused download, and `download`
            // already resumes it with a Range request.
            let _ = eidos_nexus::write_download_meta(
                &dest,
                game.def.short_name,
                &nxm,
                &link,
                &file,
                &remote_mod,
            );
            println!("Downloading {} ({}) ...", file.name, name);
            run_transfer(&nexus, &link, &dest, &inst_arg);
        }
        None => {
            eidos_log::info!(
                "usage:\n\
                 \x20 eidos nxm <nxm://...>   download a Mod Manager Download link\n\
                 \x20 eidos nxm --resume <f>  continue a paused or interrupted download\n\
                 \x20 eidos nxm --register    make the browser send nxm:// links to Eidos"
            );
            exit(2);
        }
    }
}


/// Run the transfer, announcing this process in the sidecar for the lifetime of
/// the download so the window can pause or cancel it.
///
/// The pid is cleared on every exit path, successful or not - a stale one would
/// make a finished download look like it still had a process behind it. It is
/// also only ever BELIEVED after a liveness check (`live_download_pid`), because
/// a machine that lost power leaves one behind that no cleanup can reach.
fn run_transfer(nexus: &eidos_nexus::Nexus, link: &str, dest: &std::path::Path, inst_arg: &str) {
    let _ = eidos_nexus::set_download_meta_key(
        dest,
        eidos_nexus::DOWNLOAD_PID_KEY,
        &std::process::id().to_string(),
    );
    let outcome = nexus.download(link, dest);
    let _ = eidos_nexus::set_download_meta_key(dest, eidos_nexus::DOWNLOAD_PID_KEY, "");
    match outcome {
        Ok(bytes) => {
            let _ = eidos_nexus::set_download_meta_key(dest, "paused", "false");
            println!("Downloaded {} ({:.1} MiB)", dest.display(), bytes as f64 / (1024.0 * 1024.0));
            println!("Install it:  eidos install \"{inst_arg}\" \"{}\"", dest.display());
        }
        Err(e) => {
            eidos_log::warn!("download failed: {e}");
            exit(1);
        }
    }
}

/// `eidos nxm --resume <archive>` - continue a paused or interrupted download.
///
/// The stored CDN link cannot simply be reused: it carries an `expires=`
/// timestamp and a signature, so a download paused for an hour has a dead URL. A
/// fresh one is resolved from the mod and file ids the sidecar recorded, and the
/// partial then resumes with a Range request from exactly where it stopped.
///
/// A free account has no way through this: its download links are minted
/// per-click by the site, so resuming needs a new "Mod Manager Download" press.
/// `download_link` already says so, and that message is what surfaces here.
fn resume(archive: &std::path::Path) {
    let meta = eidos_nexus::meta_path_for(archive);
    if !meta.is_file() {
        eidos_log::info!("no .meta beside {} - nothing to resume from", archive.display());
        exit(1);
    }
    if let Some(pid) = eidos_nexus::live_download_pid(archive) {
        eidos_log::info!("already downloading (pid {pid})");
        exit(1);
    }
    let key = |k: &str| eidos_nexus::download_meta_key(archive, k);
    let (Some(game), Some(mod_id), Some(file_id)) = (key("gameName"), key("modID"), key("fileID"))
    else {
        eidos_log::info!("that .meta does not record which Nexus file it came from");
        exit(1);
    };
    let (Ok(mod_id), Ok(file_id)) = (mod_id.parse::<u64>(), file_id.parse::<u64>()) else {
        eidos_log::warn!("that .meta has an unreadable modID/fileID");
        exit(1);
    };
    // The sidecar stores the game by SHORT name; the API wants the domain.
    let domain = eidos_games::catalog()
        .iter()
        .find(|d| d.short_name.eq_ignore_ascii_case(&game))
        .map(|d| d.nexus_game.to_string())
        .unwrap_or(game);
    let nexus = match eidos_nexus::Nexus::connect() {
        Ok(n) => n,
        Err(e) => {
            eidos_log::info!("{e}");
            exit(1);
        }
    };
    let remote_mod = match nexus.mod_info(&domain, mod_id) {
        Ok(m) => m,
        Err(e) => {
            eidos_log::warn!("could not look the mod up: {e}");
            exit(1);
        }
    };
    // No key and no expiry: a premium account resolves a mirror from the ids
    // alone, which is exactly the case that can be resumed unattended.
    let nxm = eidos_nexus::NxmUrl {
        game: domain,
        mod_id,
        file_id,
        key: None,
        expires: None,
        user_id: None,
    };
    let link = match nexus.download_link(&remote_mod.gate, &nxm) {
        Ok(l) => l,
        Err(e) => {
            eidos_log::warn!("could not resolve a fresh download link: {e}");
            exit(1);
        }
    };
    let _ = eidos_nexus::set_download_meta_key(archive, "paused", "false");
    println!("Resuming {} ...", archive.display());
    run_transfer(&nexus, &link, archive, "<instance>");
}


/// Hand a collection link to the window, pinned to `inst`.
///
/// The GUI is where a collection is read: it holds the mod list this has to be
/// joined against, and the answer is a list to look at rather than a thing to
/// do. Launched detached, like every other GUI hand-off here - the browser is
/// waiting on this process, and it should not wait for a window.
fn show_collection_in_gui(link: &str, inst: &Instance) {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("eidos-gui")))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("eidos-gui"));
    match std::process::Command::new(&exe)
        .arg("--collection")
        .arg(link)
        // Pinned to the instance resolved above, through the same variable the
        // CLI uses - so the window opens the collection's own game rather than
        // whichever instance happened to be last.
        .env("EIDOS_INSTANCE", &inst.root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => {}
        Err(e) => {
            eidos_log::warn!("could not open the Eidos window ({e}); the link was {link}");
        }
    }
}
