use super::*;

pub(super) const RECEIPT_DIR: &str = ".eidos-omod.mohidden";
const RECEIPT_FILE: &str = "install.json";
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PendingWrite {
    target: String,
    before: Option<String>,
    after: String,
    image: usize,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredInstall {
    version: u32,
    receipt: OmodReplayReceipt,
    plan: ObmmPlan,
    #[serde(default)]
    generated_files: Vec<String>,
    apply_profile_effects: bool,
    allow_incomplete: bool,
    warnings: Vec<String>,
    writes: Vec<PendingWrite>,
    complete: bool,
}
fn durable(path: &Path, bytes: &[u8]) -> Result<(), InstallError> {
    eidos_instance::write_atomic(path, bytes)?;
    fs::File::open(path)?.sync_all()?;
    if let Some(p) = path.parent() {
        fs::File::open(p)?.sync_all()?;
    }
    Ok(())
}
fn current(path: &Path, cancel: &AtomicBool) -> Result<Option<Vec<u8>>, InstallError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(_) => context::Source::capture(path.to_path_buf())?
            .read(MAX_EFFECT_FILE, cancel)
            .map(Some),
    }
}
fn effect_target(profile: &Path, name: &str) -> Result<PathBuf, InstallError> {
    if !matches!(
        name,
        "Oblivion.ini" | "plugins/plugins.txt" | "plugins/loadorder.txt"
    ) {
        return Err(bad("receipt contains an unsupported profile target"));
    }
    Ok(checked_destination(profile, &profile.join(name))?)
}
pub(super) fn prepare(
    context: &ScriptedContext,
    review: &ScriptedReview,
    stage: &Path,
    approval: ScriptedApproval,
    warnings: Vec<String>,
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    let dir = stage.join(RECEIPT_DIR);
    fs::create_dir(&dir)?;
    let instance = eidos_instance::Instance::portable(context.origin.instance_root.clone());
    let profile = instance.profile(&context.origin.profile);
    let mut outputs = BTreeMap::new();
    if approval.apply_profile_effects {
        let mut ini = context.ini_text.clone();
        let mut ini_changed = false;
        let mut list = context.plugin_list.clone();
        let spec = eidos_plugins::GameSpec::for_id("oblivion").unwrap();
        // Discover only this owned stage, then merge into the already resolved
        // effective list. No lower hidden provider is reintroduced.
        let added = eidos_plugins::PluginList::discover(
            &[("OMOD staging".into(), stage.to_path_buf())],
            &spec,
        );
        for p in added.plugins {
            if let Some(old) = list
                .plugins
                .iter_mut()
                .find(|old| old.name.eq_ignore_ascii_case(&p.name))
            {
                let enabled = old.enabled;
                *old = p;
                old.enabled = enabled;
            } else {
                list.plugins.push(p);
            }
        }
        let mut plugins_changed = false;
        for effect in &review.profile_effects {
            check_cancel(cancel)?;
            let a = &effect.arguments;
            match effect.command.as_str() {
                "EditINI" => {
                    let section = eidos_ini::section_header(&a[0]).unwrap_or(&a[0]);
                    if section.is_empty() || section.contains(['[', ']', '\r', '\n', '\0']) {
                        return Err(bad("invalid profile INI section"));
                    }
                    ini = eidos_ini::set_key(&ini, section, &a[1], &a[2]);
                    ini_changed = true;
                }
                "RegisterBSA" | "UnregisterBSA" => {
                    let previous =
                        eidos_ini::get_key(&ini, "Archive", "sArchiveList").unwrap_or("");
                    let mut entries: Vec<String> = previous
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                    entries.retain(|s| !s.eq_ignore_ascii_case(&a[0]));
                    if effect.command == "RegisterBSA" {
                        entries.push(a[0].clone());
                    }
                    ini = eidos_ini::set_key(&ini, "Archive", "sArchiveList", &entries.join(", "));
                    ini_changed = true;
                }
                "ActivatePlugin" | "UncheckESP" => {
                    if !list
                        .plugins
                        .iter()
                        .any(|p| p.name.eq_ignore_ascii_case(&a[0]))
                    {
                        return Err(bad(format!(
                            "profile proposal references missing plugin {}",
                            a[0]
                        )));
                    }
                    list.set_enabled(&a[0], effect.command == "ActivatePlugin");
                    plugins_changed = true;
                }
                _ => return Err(bad("unclassified profile proposal")),
            }
        }
        if ini_changed {
            let p = dir.join("ini-staging");
            eidos_instance::write_text(&p, &ini, context.ini_cp1252)?;
            outputs.insert("Oblivion.ini".to_string(), fs::read(&p)?);
            fs::remove_file(p)?;
        }
        if plugins_changed {
            list.refresh(&spec);
            let p = dir.join("plugin-staging");
            list.write_load_order(&p, &spec)?;
            for name in ["plugins.txt", "loadorder.txt"] {
                if p.join(name).is_file() {
                    outputs.insert(format!("plugins/{name}"), fs::read(p.join(name))?);
                }
            }
            if p.exists() {
                fs::remove_dir_all(p)?;
            }
        }
    }
    let mut writes = Vec::new();
    for (i, (name, bytes)) in outputs.into_iter().enumerate() {
        check_cancel(cancel)?;
        let dest = effect_target(&profile.dir(), &name)?;
        let before = current(&dest, cancel)?;
        if before.as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        if let Some(b) = &before {
            durable(&dir.join(format!("before-{i}")), b)?;
        }
        durable(&dir.join(format!("after-{i}")), &bytes)?;
        writes.push(PendingWrite {
            target: name,
            before: before.as_ref().map(|b| digest(b)),
            after: digest(&bytes),
            image: i,
        });
    }
    let record = StoredInstall {
        version: 1,
        receipt: review.receipt.clone(),
        plan: review.plan.clone(),
        generated_files: review.generated_files.clone(),
        apply_profile_effects: approval.apply_profile_effects,
        allow_incomplete: approval.allow_incomplete,
        warnings,
        writes,
        complete: false,
    };
    let bytes = serde_json::to_vec_pretty(&record).map_err(json_error)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(bad("OMOD receipt exceeds 16 MiB"));
    }
    durable(&dir.join(RECEIPT_FILE), &bytes)?;
    Ok(())
}
pub(super) fn receipt_path(mod_dir: &Path) -> PathBuf {
    mod_dir.join(RECEIPT_DIR).join(RECEIPT_FILE)
}
fn read_record(
    mod_root: &Path,
    cancel: &AtomicBool,
) -> Result<Option<(PathBuf, StoredInstall)>, InstallError> {
    let root = checked_destination(Path::new("/"), mod_root)?;
    let file = checked_destination(&root, &receipt_path(&root))?;
    match fs::symlink_metadata(&file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    let bytes = context::Source::capture(file.clone())?.read(16 * 1024 * 1024, cancel)?;
    let record: StoredInstall = serde_json::from_slice(&bytes).map_err(json_error)?;
    if record.version != 1 || record.receipt.version != 1 {
        return Err(bad("unsupported OMOD effect receipt version"));
    }
    Ok(Some((file, record)))
}
pub(super) fn read_receipt(mod_root: &Path) -> Result<Option<OmodReplayReceipt>, InstallError> {
    Ok(read_record(mod_root, &AtomicBool::new(false))?.map(|(_, record)| record.receipt))
}
pub(super) fn retry(
    instance: &eidos_instance::Instance,
    mod_name: &str,
    cancel: &AtomicBool,
) -> Result<bool, InstallError> {
    check_cancel(cancel)?;
    if crate::fix_directory_name(mod_name).as_deref() != Some(mod_name) {
        return Err(bad("invalid OMOD mod name"));
    }
    if !instance.root.is_absolute() || instance.root.parent().is_none() || !instance.root.is_dir() {
        return Err(bad(
            "OMOD retry requires an existing absolute instance directory",
        ));
    }
    checked_destination(Path::new("/"), &instance.root)?;
    let _lock = instance.try_lock("finishing approved OMOD effects")?;
    let mod_dir = checked_destination(&instance.mods_dir(), &instance.mods_dir().join(mod_name))?;
    let (file, mut stored) = read_record(&mod_dir, cancel)?
        .ok_or_else(|| bad("installed mod has no OMOD effect receipt"))?;
    if stored.receipt.origin.instance_root != instance.root
        || stored.receipt.origin.profile != instance.active_profile()
        || stored.receipt.origin.game.game_id != "oblivion"
    {
        return Err(bad(
            "OMOD effects belong to a different installation or active profile",
        ));
    }
    if crate::fix_directory_name(&stored.receipt.origin.profile).as_deref()
        != Some(stored.receipt.origin.profile.as_str())
    {
        return Err(bad("invalid original OMOD profile name"));
    }
    if stored.complete {
        return Ok(true);
    }
    if !stored.apply_profile_effects && !stored.writes.is_empty() {
        return Err(bad("receipt contains unapproved profile writes"));
    }
    let profile = instance.profile(&stored.receipt.origin.profile);
    let dir = file.parent().unwrap();
    // Check the entire retry first. An edited destination never causes a new
    // partial application to an otherwise untouched group of profile files.
    let mut writes = Vec::new();
    let mut targets = std::collections::BTreeSet::new();
    for pending in &stored.writes {
        if !targets.insert(&pending.target) {
            return Err(bad("duplicate pending profile target"));
        }
        if let Some(expected) = &pending.before {
            let backup = checked_destination(dir, &dir.join(format!("before-{}", pending.image)))?;
            let bytes = context::Source::capture(backup)?.read(MAX_EFFECT_FILE, cancel)?;
            if &digest(&bytes) != expected {
                return Err(bad("stored profile backup checksum changed"));
            }
        }
        check_cancel(cancel)?;
        let target = effect_target(&profile.dir(), &pending.target)?;
        let image = checked_destination(dir, &dir.join(format!("after-{}", pending.image)))?;
        let bytes = context::Source::capture(image)?.read(MAX_EFFECT_FILE, cancel)?;
        if digest(&bytes) != pending.after {
            return Err(bad("stored effect image checksum changed"));
        }
        let current = current(&target, cancel)?.map(|b| digest(&b));
        if current.as_deref() == Some(&pending.after) {
            continue;
        }
        if current != pending.before {
            return Err(bad(format!(
                "profile file changed since OMOD approval: {}; pending effect preserved",
                pending.target
            )));
        }
        writes.push((target, bytes));
    }
    instance.register_installed_mod(mod_name)?;
    for (target, bytes) in writes {
        check_cancel(cancel)?;
        let target = checked_destination(&profile.dir(), &target)?;
        fs::create_dir_all(target.parent().unwrap())?;
        durable(&target, &bytes)?;
    }
    stored.complete = true;
    durable(
        &file,
        &serde_json::to_vec_pretty(&stored).map_err(json_error)?,
    )?;
    Ok(true)
}
