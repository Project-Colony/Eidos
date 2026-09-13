//! Driving a collection install: fetching its archive, installing its members,
//! and applying the three things that are about the collection as a whole.
//!
//! Separate from the pure modules beside it, and heavier: this is where the
//! network, the archiver and the installer live. It exists so the terminal and
//! the window run the SAME code - the alternative was a second copy of it in the
//! GUI, which is how this workspace ended up with three `find_7z` implementations
//! that had quietly drifted apart.

use std::path::{Path, PathBuf};

use crate::installer_answers::{self, InstallerAnswers};
use eidos_instance::Instance;
use std::sync::atomic::AtomicBool;

use crate::install::{Hooks, Installed, Obtained};
use crate::manifest::{Collection, Mod, SourceType};
use crate::report::{Note, Report};
use crate::state::InstallState;

/// Download the collection archive if it is not already here, and hand back its
/// `collection.json`.
pub fn fetch_manifest(
    nexus: &eidos_nexus::Nexus,
    rev: &eidos_nexus::collections::CollectionRevision,
    dir: &Path,
) -> Result<String, String> {
    if let Some(manifest) = cached_manifest(dir)? {
        return Ok(manifest);
    }
    let parent = dir.parent().ok_or("Collection cache has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let url = nexus.collection_archive_url(&rev.download_link)?;
    let archive = dir.with_extension("download.7z");
    (|| -> Result<String, String> {
        nexus.download(&url, &archive)?;
        let extracted =
            eidos_install::extract_to_temp(&archive, parent).map_err(|e| e.to_string())?;
        let manifest_path = extracted.path().join("collection.json");
        let text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Collection archive has no readable collection.json: {e}"))?;
        crate::read(&text)?;
        let hash = eidos_nexus::md5_file(&manifest_path).map_err(|e| e.to_string())?;
        let marker = cache_marker(dir);
        if marker.exists() {
            std::fs::remove_file(&marker).map_err(|e| e.to_string())?;
        }
        if dir.exists() || dir.is_symlink() {
            // Keep an interrupted/legacy cache instead of deleting unknown contents.
            let stale = parent.join(eidos_nexus::unique_download_name(
                parent,
                &format!("{}.incomplete", dir.file_name().unwrap().to_string_lossy()),
            ));
            std::fs::rename(dir, stale).map_err(|e| e.to_string())?;
        }
        std::fs::rename(extracted.path(), dir).map_err(|e| e.to_string())?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .map_err(|e| e.to_string())?;
        use std::io::Write;
        output
            .write_all(hash.as_bytes())
            .and_then(|_| output.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(text)
    })()
}

fn cache_marker(dir: &Path) -> PathBuf {
    dir.with_extension("payload-complete")
}

fn cached_manifest(dir: &Path) -> Result<Option<String>, String> {
    let marker = cache_marker(dir);
    if !std::fs::symlink_metadata(&marker).is_ok_and(|m| m.file_type().is_file())
        || !std::fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_dir())
    {
        return Ok(None);
    }
    let manifest = dir.join("collection.json");
    if !std::fs::symlink_metadata(&manifest).is_ok_and(|m| m.file_type().is_file()) {
        return Ok(None);
    }
    let expected = std::fs::read_to_string(marker).map_err(|e| e.to_string())?;
    if eidos_nexus::md5_file(&manifest).map_err(|e| e.to_string())? != expected {
        return Ok(None);
    }
    let text = std::fs::read_to_string(manifest).map_err(|e| e.to_string())?;
    crate::read(&text)?;
    Ok(Some(text))
}

/// Everything the engine needs from the outside world.
pub struct RealHooks<'a> {
    pub nexus: &'a eidos_nexus::Nexus,
    pub inst: &'a Instance,
    pub game: &'a eidos_games::DetectedGame,
    pub game_id: String,
    /// Where per-member progress goes. The terminal prints it; the window
    /// stores it for its dialog.
    pub say: &'a mut dyn FnMut(String),
    /// The collection's own Nexus domain, for members that name none.
    pub collection_domain: String,
    /// Collection slug and revision, distinct even when two collections share a file.
    pub owner: String,
    /// Members installed under a name that was free, and what it was.
    pub renamed: Vec<(String, String)>,
    pub payload_root: PathBuf,
    pub allow_runtime_mismatch: bool,
}

impl RealHooks<'_> {
    fn owner_marker(&self, m: &Mod) -> String {
        serde_json::to_string(&[
            self.owner.as_str(),
            &crate::state::key_for(m, &self.collection_domain),
        ])
        .expect("strings serialize")
    }

    fn owns(&self, folder: &Path, m: &Mod) -> bool {
        owns_folder(folder, &self.owner_marker(m))
    }
    /// Record the successful install without changing existing profile decisions.
    fn register(
        &mut self,
        name: &str,
        member: &str,
        renamed_to: &Option<String>,
    ) -> Result<(), String> {
        self.inst
            .register_installed_mod(name)
            .map_err(|e| e.to_string())?;
        if let Some(folder) = renamed_to {
            self.renamed.push((member.to_string(), folder.clone()));
        }
        Ok(())
    }

    /// The archive already in `downloads/` for this member, if it is whole.
    fn already_here(&self, m: &Mod) -> Option<PathBuf> {
        let want = m.source.file_id?;
        let mod_id = m.source.mod_id?;
        let domain = if m.domain_name.is_empty() {
            &self.collection_domain
        } else {
            &m.domain_name
        };
        let short = eidos_games::catalog()
            .iter()
            .find(|g| g.nexus_game.eq_ignore_ascii_case(domain))
            .map(|g| g.short_name)
            .unwrap_or(domain);
        let dl = self.inst.downloads_dir();
        std::fs::read_dir(&dl)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "meta"))
            .find_map(|p| {
                let meta = eidos_instance::ModMeta::read(&p);
                let archive = p.with_extension("");
                (meta.file_id()? == want
                    && meta.mod_id()? == mod_id
                    && meta.game_name().is_some_and(|game| {
                        game.eq_ignore_ascii_case(short) || game.eq_ignore_ascii_case(domain)
                    })
                    && archive.is_file()
                    && !PathBuf::from(format!("{}.unfinished", archive.display())).exists())
                .then_some(archive)
            })
    }
}

fn owns_folder(folder: &Path, marker: &str) -> bool {
    std::fs::symlink_metadata(folder).is_ok_and(|meta| meta.file_type().is_dir())
        && eidos_instance::ModMeta::read(&folder.join("meta.ini")).collection_owner()
            == Some(marker)
}

fn reserve_folder(
    inst: &Instance,
    wanted: &str,
    marker: &str,
    previous: Option<&str>,
) -> Result<String, String> {
    let mods = inst.mods_dir();
    if let Some(previous) = previous {
        if eidos_install::fix_directory_name(previous).as_deref() != Some(previous) {
            return Err("The recorded collection folder is not a safe mod name".into());
        }
        let path = mods.join(previous);
        if path.exists() || path.is_symlink() {
            return if owns_folder(&path, marker) {
                Ok(previous.to_string())
            } else {
                Err(format!(
                    "The reserved folder '{previous}' is no longer owned by this collection; it was left untouched"
                ))
            };
        }
    }
    let mut name = previous.unwrap_or(wanted).to_string();
    loop {
        // Linux permits names differing only by case; the game's merged view does not.
        let collision = std::fs::read_dir(&mods)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name));
        if collision {
            name = free_name(&mods, wanted);
            continue;
        }
        let path = mods.join(&name);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                name = free_name(&mods, wanted);
                continue;
            }
            Err(e) => return Err(e.to_string()),
        }
        let mut meta = eidos_instance::ModMeta::default();
        meta.set("eidosCollectionOwner", marker);
        meta.write(&path.join("meta.ini"))
            .map_err(|e| e.to_string())?;
        std::fs::File::open(path.join("meta.ini"))
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::File::open(&path)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::File::open(&mods)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        return Ok(name);
    }
}

impl Hooks for RealHooks<'_> {
    fn validate_recipe(&mut self, c: &Collection) -> Result<(), String> {
        for m in &c.mods {
            crate::recipe::map_exclusions(m, &self.game.install_path, &self.game.data_path)?;
        }
        Ok(())
    }
    fn runtime_check(&mut self, c: &Collection) -> crate::recipe::RuntimeCheck {
        crate::recipe::runtime_check(c, self.game)
    }
    fn allow_runtime_mismatch(&self) -> bool {
        self.allow_runtime_mismatch
    }

    fn verify_installed(&mut self, m: &Mod, folder: &str) -> Result<bool, String> {
        if eidos_install::fix_directory_name(folder).as_deref() != Some(folder) {
            return Err("The saved collection folder is not a safe mod name".into());
        }
        let path = self.inst.mods_dir().join(folder);
        if !path.exists() && !path.is_symlink() {
            return Ok(false);
        }
        if !self.owns(&path, m) {
            return Err(format!(
                "The recorded folder '{folder}' cannot be verified as this collection's output; it was left untouched"
            ));
        }
        let mapped =
            crate::recipe::map_exclusions(m, &self.game.install_path, &self.game.data_path)?;
        crate::recipe::verify_receipt(&mapped, &self.payload_root, &path, &self.owner_marker(m))
    }

    fn recover_installed(&mut self, m: &Mod, folder: &str) -> Result<Option<Installed>, String> {
        if !self.verify_installed(m, folder)? {
            return Ok(None);
        }
        if self
            .inst
            .mods_dir()
            .join(folder)
            .join(".eidos-omod.mohidden/install.json")
            .exists()
        {
            match eidos_install::scripted::retry_omod_effects(self.inst, folder, &AtomicBool::new(false)) {
                Ok(true) => {},
                Ok(false) => return Ok(Some(Installed::NeedsUser("Approved OMOD profile effects remain pending; resume after resolving the reported issue".into()))),
                Err(e) => return Ok(Some(Installed::NeedsUser(format!("Approved OMOD profile effects could not be retried: {e}")))),
            }
        }
        let renamed = (folder != safe(&m.name)).then(|| folder.to_string());
        self.register(folder, &m.name, &renamed)?;
        Ok(Some(
            if m.source.kind == SourceType::Bundle || !m.hashes.is_empty() {
                Installed::Ok(folder.into())
            } else {
                // Placement diagnostics are returned after the finishing callback. Their
                // lost status checkpoint cannot be reconstructed from content alone.
                Installed::Approximate(folder.into(), vec!["Recovered a verified completed recipe after an interrupted status save; installer replay diagnostics were not saved".into()])
            },
        ))
    }

    fn reserve(&mut self, m: &Mod, previous: Option<&str>) -> Result<String, String> {
        let _lock = self
            .inst
            .try_lock("reserving collection member")
            .map_err(|e| e.to_string())?;
        reserve_folder(self.inst, &safe(&m.name), &self.owner_marker(m), previous)
    }

    fn obtain(&mut self, m: &Mod) -> Obtained {
        let mapped =
            match crate::recipe::map_exclusions(m, &self.game.install_path, &self.game.data_path) {
                Ok(m) => m,
                Err(e) => return Obtained::Failed(e),
            };
        let m = &mapped;
        if m.source.kind == SourceType::Bundle {
            return match crate::recipe::bundle_source(m, &self.payload_root) {
                Ok(p) => Obtained::Ready(p),
                Err(e) => Obtained::Failed(e),
            };
        }
        if let Some(p) = self.already_here(m) {
            return checked_archive(m, p);
        }
        match m.source.kind {
            SourceType::Nexus => {
                let (Some(mod_id), Some(file_id)) = (m.source.mod_id, m.source.file_id) else {
                    return Obtained::Unavailable(
                        "the collection names it as a Nexus file but gives no ids".into(),
                    );
                };
                let domain = if m.domain_name.is_empty() {
                    &self.collection_domain
                } else {
                    &m.domain_name
                };
                // The gate first, as every download path here does: a mod whose
                // page Eidos may not describe is one whose files it does not
                // fetch either. It costs one API call per member and there is no
                // cheaper way to obtain a gate honestly.
                let remote = match self.nexus.mod_info(domain, mod_id) {
                    Ok(remote) => remote,
                    Err(e) => return Obtained::Failed(e),
                };
                let gate = remote.gate;
                let nxm = eidos_nexus::NxmUrl {
                    game: domain.to_string(),
                    mod_id,
                    file_id,
                    key: None,
                    expires: None,
                    user_id: None,
                };
                match self.nexus.download_link(&gate, &nxm) {
                    Ok(url) => {
                        let downloads = self.inst.downloads_dir();
                        let dest = downloads.join(eidos_nexus::unique_download_name(
                            &downloads,
                            &format!("{}-{mod_id}-{file_id}.archive", safe(&m.name)),
                        ));
                        match self.nexus.download(&url, &dest) {
                            Ok(_) => {
                                // The `.meta` sidecar. Without it this archive is
                                // invisible to `already_here`, so an interrupted
                                // collection re-downloads everything it had - and
                                // the rest of Eidos shows the row as Untracked.
                                // Written AFTER the download so a free account,
                                // which never gets this far, never pays for the
                                // extra request.
                                if let Ok(file) =
                                    self.nexus.file_info(&gate, domain, mod_id, file_id)
                                {
                                    let _ = eidos_nexus::write_download_meta(
                                        &dest, domain, &nxm, &url, &file, &remote,
                                    );
                                }
                                checked_archive(m, dest)
                            }
                            Err(e) => Obtained::Failed(e),
                        }
                    }
                    // The free-account wall, and the honest wording for it: this
                    // is a Nexus rule, not something a retry will get past.
                    Err(e) if e.contains("403") || e.to_lowercase().contains("premium") => {
                        Obtained::NeedsUser(format!(
                            "fetch it yourself from https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files&file_id={file_id} \
                             (a free Nexus account cannot mint download links)"
                        ))
                    }
                    Err(e) => Obtained::Failed(e),
                }
            }
            SourceType::Direct if !m.source.url.is_empty() => {
                // A matching name alone does not identify a completed archive or partial.
                let downloads = self.inst.downloads_dir();
                let dest = downloads.join(eidos_nexus::unique_download_name(
                    &downloads,
                    &format!("{}.archive", safe(&m.name)),
                ));
                match self.nexus.download(&m.source.url, &dest) {
                    Ok(_) => checked_archive(m, dest),
                    Err(e) => Obtained::Failed(e),
                }
            }
            SourceType::Browse => Obtained::NeedsUser(if m.source.url.is_empty() {
                "the collection says to fetch this one from its page".into()
            } else {
                format!("fetch it yourself from {}", m.source.url)
            }),
            _ => Obtained::Unavailable("no usable source".into()),
        }
    }

    fn install(&mut self, m: &Mod, archive: &Path, folder: &str) -> Installed {
        let _lock = match self.inst.try_lock("installing collection member") {
            Ok(lock) => lock,
            Err(e) => return Installed::Failed(e.to_string()),
        };
        let cancel = AtomicBool::new(false);
        let mods_dir = self.inst.mods_dir();
        if eidos_install::fix_directory_name(folder).as_deref() != Some(folder)
            || !self.owns(&mods_dir.join(folder), m)
        {
            return Installed::Failed("Refusing to replace an unowned collection folder".into());
        }
        let fallback = self.game.prefix().and_then(|prefix| {
            self.game
                .plugin_spec()
                .map(|spec| eidos_plugins::plugins_txt_dir(&prefix, &spec))
        });
        let ctx = eidos_install::fomod_context_for_instance(
            self.inst,
            &self.game.data_path,
            &self.game_id,
            fallback.as_deref(),
        );
        let name = folder.to_string();
        let renamed_to = (name != safe(&m.name)).then(|| name.clone());

        let mapped =
            match crate::recipe::map_exclusions(m, &self.game.install_path, &self.game.data_path) {
                Ok(m) => m,
                Err(e) => return Installed::Failed(e),
            };
        let marker = self.owner_marker(m);
        let policy = eidos_install::OverwritePolicy::ReplaceOwned(marker.clone());
        let finish = |stage: &Path| {
            crate::recipe::finish(&mapped, &self.payload_root, stage, &marker)
                .map_err(eidos_install::InstallError::BadSelection)
        };
        let pending = match installer_answers::read(&mods_dir.join(folder), &marker) {
            Ok(p) => p,
            Err(e) => return Installed::Failed(e),
        };
        let pause = |answers: &InstallerAnswers, why: &str| {
            match installer_answers::write_for_archive(&mods_dir.join(folder), &marker, archive, answers) {
                Ok(()) => Installed::NeedsUser(format!("{why}. Exact answers are saved in {}; answer that receipt and resume this collection", installer_answers::path(&mods_dir.join(folder)).display())),
                Err(e) => Installed::Failed(e),
            }
        };
        let mut unmatched = Vec::new();
        let result = if mapped.source.kind == SourceType::Bundle {
            if pending.is_some() {
                return Installed::Failed(
                    "Recorded installer answers cannot be replaced by a bundle recipe".into(),
                );
            }
            eidos_install::install_destination(
                archive,
                &mods_dir,
                &name,
                &self.game_id,
                policy,
                |stage, _| {
                    crate::recipe::populate(&mapped, archive, stage)
                        .map_err(eidos_install::InstallError::BadSelection)?;
                    finish(stage)?;
                    Ok((String::new(), false, Vec::new()))
                },
            )
        } else {
            if let Err(e) = crate::recipe::validate_archive(&mapped, archive) {
                return Installed::Failed(e);
            }
            let native_omod = match eidos_install::try_open_omod(archive, &mods_dir, |_| {}) {
                Ok(s) => s,
                Err(e) => return Installed::Failed(e.to_string()),
            };
            let addons = eidos_addons::load_addons();
            let custom_context = if native_omod.is_none()
                && (eidos_install::custom::may_handle(&addons, &self.game_id, archive)
                    || matches!(pending, Some(InstallerAnswers::Custom { .. })))
            {
                let mut controls: Vec<_> = self.game.plugin_state_dir().into_iter().collect();
                if let (Some(prefix), Some(spec)) = (self.game.prefix(), self.game.plugin_spec()) {
                    controls.push(eidos_plugins::documents_my_games_dir(&prefix, &spec));
                }
                match eidos_install::custom::context_for_instance(
                    self.inst,
                    &self.game_id,
                    &self.game.install_path,
                    &self.game.data_path,
                    self.game.prefix().as_deref(),
                    &controls,
                    &cancel,
                ) {
                    Ok(c) => c,
                    Err(e) => return Installed::Failed(e.to_string()),
                }
            } else {
                eidos_addons::Context::default()
            };
            let expected_custom = match &pending {
                Some(InstallerAnswers::Custom { receipt, .. }) => Some(receipt),
                _ => None,
            };
            // A pinned OMOD replay must never invoke an unrelated newly installed helper.
            let opened = if let Some(session) = native_omod {
                if expected_custom.is_some() {
                    return Installed::Failed(
                        "Recorded custom installer cannot change into an OMOD".into(),
                    );
                }
                Ok(eidos_install::Opened::Omod(Box::new(session)))
            } else if matches!(pending, Some(InstallerAnswers::Omod { .. })) {
                Err(eidos_install::InstallError::BadSelection(
                    "Recorded OMOD archive no longer matches its handler".into(),
                ))
            } else if !mapped.hashes.is_empty() {
                (|| {
                    let tree = eidos_install::extract_to_temp(archive, &mods_dir)?;
                    match eidos_install::custom::try_custom_installers_with_receipt(
                        tree,
                        archive,
                        &self.game_id,
                        &addons,
                        &custom_context,
                        &cancel,
                        expected_custom,
                    )? {
                        eidos_install::custom::CustomStart::Selected(s) => {
                            Ok(eidos_install::Opened::Custom(s))
                        }
                        // Hash recipes already express native archive selection, including renames.
                        eidos_install::custom::CustomStart::Fallback(tree) => {
                            Ok(eidos_install::Opened::Simple(tree))
                        }
                    }
                })()
            } else {
                eidos_install::open_archive_with_installers(
                    archive,
                    &mods_dir,
                    &name,
                    &self.game_id,
                    &addons,
                    &custom_context,
                    &cancel,
                    expected_custom,
                    |_| {},
                )
            };
            let interactive_finish = |stage: &Path| {
                crate::recipe::validate_installer_selection(&mapped, stage)
                    .map_err(eidos_install::InstallError::BadSelection)?;
                finish(stage)
            };
            match opened {
                Ok(eidos_install::Opened::Custom(session)) => {
                    let receipt = session.receipt();
                    match &session.state {
                        eidos_install::custom::CustomEvaluation::Prompt(prompt) => {
                            return pause(
                                &InstallerAnswers::Custom {
                                    receipt,
                                    prompt: Some(prompt.clone()),
                                },
                                "Custom installer requires a choice",
                            )
                        }
                        eidos_install::custom::CustomEvaluation::Manual(why) => {
                            return pause(
                                &InstallerAnswers::Custom {
                                    receipt,
                                    prompt: None,
                                },
                                &format!("Custom installer requires manual intervention: {why}"),
                            )
                        }
                        eidos_install::custom::CustomEvaluation::Cancelled => {
                            return pause(
                                &InstallerAnswers::Custom {
                                    receipt,
                                    prompt: None,
                                },
                                "Custom installer was cancelled",
                            )
                        }
                        eidos_install::custom::CustomEvaluation::Ready(plan) => {
                            unmatched.extend(plan.warnings.clone())
                        }
                    }
                    if let Err(e) = installer_answers::write_for_archive(
                        &mods_dir.join(folder),
                        &marker,
                        archive,
                        &InstallerAnswers::Custom {
                            receipt,
                            prompt: None,
                        },
                    ) {
                        return Installed::Failed(e);
                    }
                    eidos_install::custom::finish_custom_with_finish(
                        &session,
                        &mods_dir,
                        &name,
                        policy,
                        &eidos_addons::load_addons(),
                        &custom_context,
                        &cancel,
                        interactive_finish,
                    )
                }
                Ok(eidos_install::Opened::Omod(session)) if session.script.is_some() => {
                    use eidos_install::scripted::*;
                    let mut versions = std::collections::BTreeMap::new();
                    if let Ok(pe) = eidos_gamefeatures::preflight::inspect_pe(
                        &self.game.install_path.join("Oblivion.exe"),
                    ) {
                        if let Some(v) = pe.file_version {
                            versions.insert("Oblivion".into(), v.to_string());
                        }
                    }
                    let game = ScriptedGame {
                        game_id: self.game_id.clone(),
                        install_path: self.game.install_path.clone(),
                        data_path: self.game.data_path.clone(),
                        prefix: self.game.prefix(),
                        observed_versions: versions,
                    };
                    let context = match ScriptedContext::capture(self.inst, &game, &cancel) {
                        Ok(c) => c,
                        Err(e) => return Installed::Failed(e.to_string()),
                    };
                    let (receipt, approval) = match &pending {
                        Some(InstallerAnswers::Omod {
                            receipt,
                            apply_profile_effects,
                            allow_incomplete,
                            ..
                        }) => (
                            receipt.clone(),
                            ScriptedApproval {
                                apply_profile_effects: *apply_profile_effects,
                                allow_incomplete: *allow_incomplete,
                            },
                        ),
                        None => match new_omod_receipt(&session, &context, &cancel) {
                            Ok(r) => (r, ScriptedApproval::default()),
                            Err(e) => return Installed::Failed(e.to_string()),
                        },
                        _ => {
                            return Installed::Failed(
                                "Recorded custom installer cannot change into an OMOD script"
                                    .into(),
                            )
                        }
                    };
                    let evaluation =
                        match evaluate_scripted_omod(&session, &context, &receipt, &cancel) {
                            Ok(e) => e,
                            Err(e) => return Installed::Failed(e.to_string()),
                        };
                    let record = |receipt, prompt| InstallerAnswers::Omod {
                        receipt,
                        prompt,
                        apply_profile_effects: approval.apply_profile_effects,
                        allow_incomplete: approval.allow_incomplete,
                    };
                    match evaluation {
                        ScriptedEvaluation::NeedPrompt(prompt) => {
                            return pause(
                                &record(receipt, Some(prompt)),
                                "OMOD installer requires an answer",
                            )
                        }
                        ScriptedEvaluation::Review(review) => {
                            if (!review.unsupported.is_empty()
                                || !approval.apply_profile_effects
                                    && !review.profile_effects.is_empty())
                                && !approval.allow_incomplete
                            {
                                return pause(&record(review.receipt.clone(), None), &format!("OMOD review needs explicit profile-effect approval or incomplete-install acceptance. Profile effects: {:?}; unsupported: {:?}", review.profile_effects, review.unsupported));
                            }
                            if let Err(e) = installer_answers::write_for_archive(
                                &mods_dir.join(folder),
                                &marker,
                                archive,
                                &record(review.receipt.clone(), None),
                            ) {
                                return Installed::Failed(e);
                            }
                            match install_scripted_omod_with_finish(
                                &session,
                                &context,
                                &review,
                                &name,
                                policy,
                                approval,
                                &cancel,
                                interactive_finish,
                            ) {
                                Ok(report) => {
                                    unmatched.extend(report.warnings);
                                    if report.pending_effects {
                                        return Installed::NeedsUser(format!("OMOD files were published; approved profile effects remain pending at {}. Resume to retry them", report.receipt_path.display()));
                                    }
                                    Ok(report.install)
                                }
                                Err(e) => Err(e),
                            }
                        }
                    }
                }
                Ok(eidos_install::Opened::Omod(session)) => {
                    if pending.is_some() {
                        return Installed::Failed(
                            "Recorded scripted installer disappeared; native defaults were refused"
                                .into(),
                        );
                    }
                    if !mapped.hashes.is_empty() {
                        eidos_install::install_destination(
                            archive,
                            &mods_dir,
                            &name,
                            &self.game_id,
                            policy,
                            |stage, _| {
                                crate::recipe::populate(&mapped, &session.payload_root(), stage)
                                    .map_err(eidos_install::InstallError::BadSelection)?;
                                finish(stage)?;
                                Ok((String::new(), false, Vec::new()))
                            },
                        )
                    } else {
                        eidos_install::install_omod_with_finish(
                            &session,
                            &mods_dir,
                            &name,
                            &self.game_id,
                            policy,
                            finish,
                        )
                    }
                }
                Ok(eidos_install::Opened::Fomod(session)) => {
                    let r = eidos_fomod::replay(&session.config, &ctx, &recorded_answers(&mapped));
                    unmatched = r.unmatched;
                    eidos_install::finish_fomod_with_finish(
                        *session,
                        &r.selection,
                        &mods_dir,
                        &self.game_id,
                        &ctx,
                        policy,
                        finish,
                    )
                }
                Ok(
                    eidos_install::Opened::Simple(tree)
                    | eidos_install::Opened::Manual(tree)
                    | eidos_install::Opened::Bain { tree, .. },
                ) => {
                    if !mapped.hashes.is_empty() {
                        eidos_install::install_destination(
                            archive,
                            &mods_dir,
                            &name,
                            &self.game_id,
                            policy,
                            |stage, _| {
                                crate::recipe::populate(&mapped, tree.path(), stage)
                                    .map_err(eidos_install::InstallError::BadSelection)?;
                                finish(stage)?;
                                Ok((String::new(), false, Vec::new()))
                            },
                        )
                    } else {
                        eidos_install::install_extracted_with_finish(
                            &tree,
                            archive,
                            &mods_dir,
                            &name,
                            &self.game_id,
                            policy,
                            &ctx,
                            finish,
                        )
                    }
                }
                Err(e) => Err(e),
            }
        };
        match result {
            Ok(rep) => {
                if let Err(e) = self.register(&rep.name, &m.name, &renamed_to) {
                    return Installed::Failed(e);
                }
                unmatched.extend(
                    rep.missing
                        .iter()
                        .map(|p| format!("Missing archive source: {p}")),
                );
                if unmatched.is_empty() {
                    Installed::Ok(rep.name)
                } else {
                    Installed::Approximate(rep.name, unmatched)
                }
            }
            Err(e)
                if e.to_string()
                    .contains("Collection installer selection conflicts") =>
            {
                Installed::NeedsUser(format!(
                    "{e}; reservation and exact installer receipt retained at {}",
                    installer_answers::path(&mods_dir.join(folder)).display()
                ))
            }
            Err(e) => Installed::Failed(e.to_string()),
        }
    }

    fn progress(&mut self, done: usize, total: usize, member: &str) {
        (self.say)(format!("[{done}/{total}] {member}"));
    }
}

fn checked_archive(m: &Mod, archive: PathBuf) -> Obtained {
    match crate::recipe::validate_archive(m, &archive) {
        Ok(()) => Obtained::Ready(archive),
        Err(error) => Obtained::Failed(error),
    }
}

/// A folder name under `mods/` that nothing is using yet.
fn free_name(mods_dir: &Path, wanted: &str) -> String {
    let names: std::collections::HashSet<_> = std::fs::read_dir(mods_dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_ascii_lowercase())
        .collect();
    for n in 2u64.. {
        let candidate = format!("{wanted} ({n})");
        if !names.contains(&candidate.to_ascii_lowercase()) {
            return candidate;
        }
    }
    unreachable!("the filesystem cannot contain every u64 suffix")
}

pub fn safe(name: &str) -> String {
    eidos_install::fix_directory_name(name).unwrap_or_else(|| "Mod".to_string())
}

/// The collection's recorded FOMOD answers, in the shape the replay wants.
pub fn recorded_answers(m: &Mod) -> Vec<eidos_fomod::RecordedStep> {
    let Some(ch) = m.choices.as_ref() else {
        return Vec::new();
    };
    // Only FOMOD answers are replayable here. Anything else is left to the
    // installer's defaults, and the engine's `unmatched` list says so.
    if !ch.kind.is_empty() && !ch.kind.eq_ignore_ascii_case("fomod") {
        return Vec::new();
    }
    ch.options
        .as_ref()
        .map(|steps| {
            steps
                .iter()
                .map(|s| eidos_fomod::RecordedStep {
                    name: s.name.clone(),
                    groups: s
                        .groups
                        .iter()
                        .map(|g| eidos_fomod::RecordedGroup {
                            name: g.name.clone(),
                            selected: g
                                .choices
                                .iter()
                                .map(|o| eidos_fomod::RecordedOption {
                                    name: o.name.clone(),
                                    idx: o.idx,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Put the collection's members in the order its rules ask for.
pub fn apply_ordering(inst: &Instance, c: &Collection, state: &InstallState, report: &mut Report) {
    let (edges, lost) = crate::rules::edges(c);
    for u in lost {
        report.rules_lost.push(Note {
            subject: u.rule,
            detail: u.why.to_string(),
        });
    }
    if edges.is_empty() {
        return;
    }
    let ordered = crate::rules::order(c.mods.len(), &edges);
    for i in &ordered.cycles {
        if let Some(m) = c.mods.get(*i) {
            report.rule_cycles.push(m.name.clone());
        }
    }
    let folders = crate::install::installed_folders(c, state);
    let list = inst.modlist();
    let names: Vec<String> = list.iter().map(|m| m.name.clone()).collect();
    let merged = crate::rules::merge(&names, &folders, &ordered.sequence);
    if merged == names {
        return;
    }
    // Rebuilt from the existing entries so nothing but the order changes.
    let reordered: Vec<eidos_instance::ModEntry> = merged
        .iter()
        .filter_map(|n| list.iter().find(|m| &m.name == n).cloned())
        .collect();
    // Only ever a permutation. Counting the entries is not enough: a name that
    // landed in two slots keeps the count and loses a mod.
    let mut before: Vec<&String> = names.iter().collect();
    let mut after: Vec<&String> = merged.iter().collect();
    before.sort();
    after.sort();
    if reordered.len() == list.len() && before == after {
        if let Err(error) = inst.save_modlist(&reordered) {
            report.failed.push(Note {
                subject: "mod order".into(),
                detail: format!("Could not save the collection's mod order: {error}"),
            });
        }
    } else {
        report.failed.push(Note {
            subject: "mod order".into(),
            detail:
                "The collection order was not a valid permutation; the previous order was retained"
                    .into(),
        });
    }
}

/// Turn on the plugins the collection expects, and turn off the ones it does
/// not.
///
/// `collection.plugins` is a list of ACTIVATION, not a load order - Vortex
/// writes a position into it and its own reader ignores that too; ordering is
/// expressed as LOOT rules, which [`apply_plugin_rules`] merges. Under an
/// opt-out plugin model most entries are already what they ask for; the one that
/// matters is `"enabled": false`, an ESP the author deliberately unchecked,
/// which would otherwise load.
pub fn apply_plugin_states(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    c: &Collection,
    report: &mut Report,
) {
    if c.plugins.is_empty() {
        return;
    }
    let Some(spec) = game.plugin_spec() else {
        return;
    };
    let Some(prefix) = game.prefix() else {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: format!(
                "no Wine prefix yet, so the {} plugin(s) the collection names were not \
                 switched on",
                c.plugins.len()
            ),
        });
        return;
    };
    let local_dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    let _ = inst.ensure_profiles();
    let prof = inst.active();
    if prof.seed_plugin_state(&local_dir, &spec).is_err() {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: "the profile's plugin state could not be read, so the collection's \
                     activations were not applied"
                .into(),
        });
        return;
    }
    let state_dir = prof.plugins_state_dir();

    // Use the launch view, including whiteouts and the active profile's state.
    let Some(mut list) = inst.plugin_list(&game.data_path, game.def.id, Some(&local_dir)) else {
        return;
    };

    let before: Vec<bool> = list.plugins.iter().map(|p| p.enabled).collect();
    let mut missing: Vec<String> = Vec::new();
    for p in &c.plugins {
        if p.name.trim().is_empty() {
            continue;
        }
        if !list.set_enabled(&p.name, p.enabled) {
            missing.push(p.name.clone());
        }
    }
    // Under an opt-out model most of a collection's list is already what it
    // asks for, and rewriting the load order to say nothing new is a write into
    // the user's profile for no reason.
    if list.plugins.iter().map(|p| p.enabled).eq(before) {
        for name in missing {
            report.loot_notes.push(Note {
                subject: name,
                detail: "the collection expects this plugin and the instance does not have it"
                    .into(),
            });
        }
        return;
    }
    list.refresh(&spec);
    if let Err(e) = list.write_load_order(&state_dir, &spec) {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: format!("the collection's plugin activations could not be written: {e}"),
        });
        return;
    }
    // Shadow for tools that read the prefix; never fatal.
    let _ = list.write_load_order(&local_dir, &spec);
    for name in missing {
        report.loot_notes.push(Note {
            subject: name,
            detail: "the collection expects this plugin and the instance does not have it".into(),
        });
    }
}

/// Merge the collection's LOOT rules into the instance's userlist.
pub fn apply_plugin_rules(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    c: &Collection,
    report: &mut Report,
) {
    if c.plugins.is_empty() && c.plugin_rules.plugins.is_empty() {
        return;
    }
    let rules: Vec<eidos_loot::UserRule> = c
        .plugin_rules
        .plugins
        .iter()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?.to_string();
            Some(eidos_loot::UserRule {
                plugin: name,
                after: p
                    .get("after")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                group: p.get("group").and_then(|g| g.as_str()).map(str::to_string),
            })
        })
        .collect();
    let groups: Vec<eidos_loot::GroupDef> = c
        .plugin_rules
        .groups
        .iter()
        .filter_map(|g| {
            Some(eidos_loot::GroupDef {
                name: g.get("name")?.as_str()?.to_string(),
                after: g
                    .get("after")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect();
    if rules.is_empty() && groups.is_empty() {
        return;
    }
    let Some(prefix) = game.prefix() else {
        report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: "no Wine prefix yet, so the collection's LOOT rules were not merged".into(),
        });
        return;
    };
    let cache = inst.root.join("loot");
    let Some((_, repo)) = eidos_loot::loot_support(game.def.id) else {
        return;
    };
    let Ok((masterlist, prelude)) = eidos_loot::ensure_masterlist(repo, &cache, false) else {
        report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: "the LOOT masterlist could not be fetched, so the rules were not merged".into(),
        });
        return;
    };
    let userlist = cache.join("userlist.yaml");
    let Some(spec) = game.plugin_spec() else {
        return;
    };
    let local = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    let view = eidos_loot::GameView {
        game_id: game.def.id,
        game_path: &game.install_path,
        local_path: &local,
        plugins: &[],
        mod_dirs: &[],
        masterlist: &masterlist,
        prelude: &prelude,
        userlist: Some(&userlist),
    };
    match eidos_loot::merge_user_rules(&view, &rules, &groups) {
        Ok(m) => {
            for p in m.kept_user_group {
                report.loot_notes.push(Note {
                    subject: p,
                    detail: "your own group assignment was kept".into(),
                });
            }
            for d in m.dangling_groups {
                report.loot_notes.push(Note {
                    subject: d,
                    detail: "no such LOOT group exists, so the assignment was dropped".into(),
                });
            }
            for d in m.dangling_after {
                report.loot_notes.push(Note {
                    subject: d,
                    detail: "the collection's group would load after a group nothing defines, \
                             which stops LOOT sorting entirely, so that part was dropped"
                        .into(),
                });
            }
            if m.rules_added > 0 {
                println!(
                    "  merged {} plugin rule(s) into the load order",
                    m.rules_added
                );
            }
        }
        Err(e) => report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: format!("the collection's rules could not be merged: {e}"),
        }),
    }
}

/// Copy the collection's INI fragments in as a mod of their own.
pub fn apply_ini_tweaks(
    inst: &Instance,
    dir: &Path,
    c: &Collection,
    state: &mut InstallState,
    save: &mut dyn FnMut(&InstallState) -> Result<(), String>,
    report: &mut Report,
) {
    let result = (|| -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        let src = entries.filter_map(Result::ok).map(|e| e.path()).find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ini tweaks"))
        });
        let Some(src) = src else {
            return Ok(());
        };
        let root = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
        let files: Vec<_> = std::fs::read_dir(&src)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for file in &files {
            let path = file.path();
            if !std::fs::canonicalize(&path)
                .map_err(|e| e.to_string())?
                .starts_with(&root)
            {
                return Err("An INI source escapes the extracted collection directory".into());
            }
        }
        if !files.iter().any(|f| f.path().is_file()) {
            return Ok(());
        }
        let key = "aux:ini-tweaks";
        let owner = format!("{}:{}:{}", state.game_domain, state.slug, state.revision);
        let marker = serde_json::to_string(&[owner.as_str(), key]).expect("strings serialize");
        let wanted = safe(&format!("{} - INI Tweaks", c.info.name));
        let name = reserve_folder(
            inst,
            &wanted,
            &marker,
            state.folders.get(key).map(String::as_str),
        )?;
        state.folders.insert(key.into(), name.clone());
        if let Err(error) = save(state) {
            report.aborted = true;
            return Err(format!(
                "Could not save the INI output reservation; copying stopped: {error}"
            ));
        }
        let dest = inst.mods_dir().join(&name).join("Ini Tweaks");
        // A replaced subdirectory must not redirect writes out of the owned mod.
        if std::fs::symlink_metadata(&dest).is_ok_and(|m| !m.file_type().is_dir()) {
            return Err("The INI output directory is not an owned directory".into());
        }
        std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        let mut n = 0;
        for file in files {
            if !file.path().is_file() {
                continue;
            }
            let target = dest.join(file.file_name());
            let tmp = dest.join(format!(".eidos-copy-{}-{n}", std::process::id()));
            let copy = (|| -> std::io::Result<()> {
                let mut output = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&tmp)?;
                std::io::copy(&mut std::fs::File::open(file.path())?, &mut output)?;
                output.sync_all()?;
                std::fs::rename(&tmp, &target)
            })();
            if let Err(error) = copy {
                // Only remove the temporary file if this operation created it.
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    let _ = std::fs::remove_file(&tmp);
                }
                return Err(format!(
                    "Could not install {}: {error}",
                    file.path().display()
                ));
            }
            n += 1;
        }
        inst.register_installed_mod(&name)
            .map_err(|e| e.to_string())?;
        report.deferred.push(Note { subject: "INI tweaks".into(), detail: format!("{n} fragment(s) installed as '{name}'; select the wanted fragments in its INI Tweaks tab") });
        Ok(())
    })();
    if let Err(error) = result {
        report.failed.push(Note {
            subject: "INI tweaks".into(),
            detail: error,
        });
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn collection_json_alone_never_completes_a_payload_cache() {
        let root = std::env::temp_dir().join(format!("eidos-payload-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("collection-1");
        std::fs::create_dir_all(&dir).unwrap();
        let text = r#"{"mods":[{"name":"Member"}]}"#;
        std::fs::write(dir.join("collection.json"), text).unwrap();
        assert!(cached_manifest(&dir).unwrap().is_none());
        std::fs::write(cache_marker(&dir), "partial marker").unwrap();
        assert!(cached_manifest(&dir).unwrap().is_none());
        let hash = eidos_nexus::md5_file(&dir.join("collection.json")).unwrap();
        std::fs::write(cache_marker(&dir), hash).unwrap();
        assert_eq!(cached_manifest(&dir).unwrap(), Some(text.into()));
        std::fs::write(dir.join("collection.json"), "changed recipe").unwrap();
        assert!(cached_manifest(&dir).unwrap().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
