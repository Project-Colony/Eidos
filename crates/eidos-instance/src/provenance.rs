//! Per-path generated output receipts, stored outside the game-visible layers.
use crate::{FileStamp, Instance, ModMeta, OverwriteSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedInput {
    pub name: String,
    pub version: Option<String>,
    pub installed_files: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutable {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedReceipt {
    pub id: String,
    pub tool: String,
    pub command: Vec<String>,
    pub executable: Option<ToolExecutable>,
    #[serde(default)]
    pub tool_preset: Option<(PathBuf, Vec<String>)>,
    pub profile: String,
    pub started_at: u64,
    pub inputs: Vec<GeneratedInput>,
    pub plugins: Vec<(String, bool)>,
}

#[derive(Debug, Clone)]
pub struct GeneratedRun {
    pub receipt: GeneratedReceipt,
    pub before: OverwriteSnapshot,
}

#[derive(Debug, Clone)]
pub struct GeneratedOutput {
    pub receipt: GeneratedReceipt,
    pub paths: Vec<PathBuf>,
}

const RECEIPTS_FILE: &str = ".generated-output.json";

#[derive(Debug, Serialize, Deserialize)]
struct StoredOutput {
    mod_name: Option<String>,
    path: PathBuf,
    stamp: FileStamp,
    receipt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GeneratedStore {
    version: u32,
    receipts: HashMap<String, GeneratedReceipt>,
    files: Vec<StoredOutput>,
    #[serde(default)]
    pending: HashMap<String, GeneratedReceipt>,
}

impl Default for GeneratedStore {
    fn default() -> Self {
        Self {
            version: 1,
            receipts: HashMap::new(),
            files: Vec::new(),
            pending: HashMap::new(),
        }
    }
}

fn file_stamp(path: &Path) -> io::Result<FileStamp> {
    use std::os::unix::fs::MetadataExt;
    let md = fs::symlink_metadata(path)?;
    Ok(FileStamp {
        len: md.len(),
        mtime: md.modified()?,
        ino: md.ino(),
        ctime: md.ctime(),
        ctime_ns: md.ctime_nsec(),
    })
}

fn executable_fingerprint(path: &Path) -> Option<ToolExecutable> {
    let md = fs::metadata(path).ok().filter(|md| md.is_file())?;
    Some(ToolExecutable {
        path: path.to_path_buf(),
        size: md.len(),
        modified: md.modified().ok()?,
    })
}

impl StoredOutput {
    fn real_path(&self, instance: &Instance) -> PathBuf {
        match &self.mod_name {
            Some(name) => instance.mods_dir().join(name).join(&self.path),
            None => instance.overwrite_dir().join(&self.path),
        }
    }
    fn unchanged(&self, instance: &Instance) -> bool {
        file_stamp(&self.real_path(instance)).is_ok_and(|stamp| stamp == self.stamp)
    }
}

impl GeneratedStore {
    fn save(&mut self, instance: &Instance) -> io::Result<()> {
        self.files.retain(|file| file.unchanged(instance));
        self.receipts
            .retain(|id, _| self.files.iter().any(|file| &file.receipt == id));
        crate::write_atomic(
            &instance.root.join(RECEIPTS_FILE),
            &serde_json::to_vec(self)?,
        )
    }

    /// Update only successful moves. Unknown output invalidates any previous
    /// attribution at its destination; a failed move retains its Overwrite receipt.
    pub(crate) fn moved(
        &mut self,
        instance: &Instance,
        before: &OverwriteSnapshot,
        moved: &[(String, PathBuf)],
    ) -> io::Result<()> {
        if self.files.is_empty() {
            return Ok(());
        }
        for (owner, path) in moved {
            let receipt = self
                .files
                .iter()
                .find(|file| {
                    file.mod_name.is_none()
                        && file.path == *path
                        && before.files.get(path) == Some(&file.stamp)
                })
                .map(|file| file.receipt.clone());
            self.files.retain(|file| {
                !(file.path == *path
                    && (file.mod_name.is_none() || file.mod_name.as_deref() == Some(owner)))
            });
            if let Some(receipt) = receipt {
                self.files.push(StoredOutput {
                    mod_name: Some(owner.clone()),
                    path: path.clone(),
                    receipt,
                    stamp: file_stamp(&instance.mods_dir().join(owner).join(path))?,
                });
            }
        }
        self.save(instance)
    }
}

impl Instance {
    pub(crate) fn generated_store(&self) -> io::Result<GeneratedStore> {
        let bytes = match fs::read(self.root.join(RECEIPTS_FILE)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(GeneratedStore::default())
            }
            Err(error) => return Err(error),
        };
        let store: GeneratedStore = serde_json::from_slice(&bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid generated-output receipts: {error}"),
            )
        })?;
        let mut paths = std::collections::HashSet::new();
        if store.version != 1
            || store.files.iter().any(|file| {
                !paths.insert((file.mod_name.clone(), file.path.clone()))
                    || file.path.as_os_str().is_empty()
                    || !file
                        .path
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                    || file
                        .mod_name
                        .as_ref()
                        .is_some_and(|name| !crate::tools::is_mod_folder_name(name))
                    || !store.receipts.contains_key(&file.receipt)
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid generated-output receipt paths or version",
            ));
        }
        Ok(store)
    }

    pub(crate) fn generated_store_for_move(&self) -> io::Result<GeneratedStore> {
        let mut store = self.generated_store()?;
        if !store.files.is_empty() {
            // Refuse ordinary persistence failures before moving user files.
            store.save(self)?;
        }
        Ok(store)
    }

    fn generated_tool_preset(&self, title: &str) -> Option<(PathBuf, Vec<String>)> {
        self.tools()
            .into_iter()
            .find(|tool| tool.title.eq_ignore_ascii_case(title))
            .map(|tool| (tool.exe, tool.args))
    }

    fn generated_inputs(&self) -> Vec<GeneratedInput> {
        self.modlist()
            .into_iter()
            .filter(|entry| entry.is_active())
            .map(|entry| {
                let meta = ModMeta::read(&entry.path.join("meta.ini"));
                GeneratedInput {
                    name: entry.name,
                    version: meta.version(),
                    installed_files: meta.installed_files(),
                }
            })
            .collect()
    }

    /// Capture inputs BEFORE a tool starts. The command is recorded verbatim;
    /// an executable fingerprint is size/mtime, never a guessed tool version.
    pub fn begin_tool_run(
        &self,
        tool: &str,
        executable: Option<&Path>,
        command: &[String],
        plugins: &[(String, bool)],
    ) -> io::Result<GeneratedRun> {
        let _lock = self.try_lock("recording tool inputs")?;
        let mut store = self.generated_store()?;
        let (_, trust) = self.modlist_checked();
        if let Some(reason) = trust.reason() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                reason.to_string(),
            ));
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let run = GeneratedRun {
            receipt: GeneratedReceipt {
                id: format!("{}-{}", now.as_nanos(), std::process::id()),
                tool: tool.into(),
                command: command.into(),
                executable: executable
                    .or_else(|| command.first().map(Path::new))
                    .and_then(executable_fingerprint),
                tool_preset: self.generated_tool_preset(tool),
                profile: self.active_profile(),
                started_at: now.as_secs(),
                inputs: self.generated_inputs(),
                plugins: plugins.into(),
            },
            before: self.overwrite_snapshot(),
        };
        store
            .pending
            .insert(run.receipt.id.clone(), run.receipt.clone());
        store.save(self)?;
        Ok(run)
    }

    /// Interrupted runs are retained as unverified context. No files are ever
    /// attributed to them automatically after a restart.
    pub fn pending_tool_runs(&self) -> io::Result<Vec<GeneratedReceipt>> {
        let mut pending: Vec<_> = self.generated_store()?.pending.into_values().collect();
        pending.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(pending)
    }

    /// Persist receipts even after a nonzero tool exit, before any output capture.
    /// Paths unchanged during this run retain their own earlier receipts.
    pub fn finish_tool_run(&self, run: GeneratedRun) -> io::Result<usize> {
        let _lock = self.try_lock("recording generated output")?;
        let mut store = self.generated_store()?;
        if store.pending.get(&run.receipt.id) != Some(&run.receipt) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tool run has no matching pending receipt",
            ));
        }
        store.pending.remove(&run.receipt.id);
        let after = self.overwrite_snapshot();
        let mut count = 0;
        for (path, stamp) in &after.files {
            if run.before.files.get(path) == Some(stamp) {
                continue;
            }
            store
                .files
                .retain(|file| file.mod_name.is_some() || file.path != *path);
            store.files.push(StoredOutput {
                mod_name: None,
                path: path.clone(),
                stamp: *stamp,
                receipt: run.receipt.id.clone(),
            });
            count += 1;
        }
        if count > 0 {
            store.receipts.insert(run.receipt.id.clone(), run.receipt);
        }
        store.save(self)?;
        Ok(count)
    }

    /// Current attributed paths for every owner, grouped by run. None is Overwrite.
    /// External rewrites and removed files are omitted rather than misattributed.
    pub fn generated_outputs(&self) -> io::Result<Vec<(Option<String>, GeneratedOutput)>> {
        let store = self.generated_store()?;
        let mut groups =
            std::collections::BTreeMap::<(Option<String>, String), Vec<PathBuf>>::new();
        for file in &store.files {
            if file.unchanged(self) {
                groups
                    .entry((file.mod_name.clone(), file.receipt.clone()))
                    .or_default()
                    .push(file.path.clone());
            }
        }
        Ok(groups
            .into_iter()
            .map(|((owner, id), mut paths)| {
                paths.sort();
                (
                    owner,
                    GeneratedOutput {
                        receipt: store.receipts[&id].clone(),
                        paths,
                    },
                )
            })
            .collect())
    }

    /// Current attributed paths for one owner; None selects Overwrite.
    pub fn generated_output(&self, mod_name: Option<&str>) -> io::Result<Vec<GeneratedOutput>> {
        if mod_name.is_some_and(|name| !crate::tools::is_mod_folder_name(name)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid mod name",
            ));
        }
        Ok(self
            .generated_outputs()?
            .into_iter()
            .filter(|(owner, _)| owner.as_deref() == mod_name)
            .map(|(_, output)| output)
            .collect())
    }

    /// A conservative comparison of the recorded inputs, not inferred dependencies.
    pub fn generated_input_drift(
        &self,
        receipt: &GeneratedReceipt,
        plugins: &[(String, bool)],
    ) -> bool {
        let Ok(store) = self.generated_store() else {
            return true;
        };
        let produced: Vec<_> = store
            .files
            .iter()
            .filter(|file| file.receipt == receipt.id && file.unchanged(self))
            .collect();
        // Creating/enabling this run's own output is not a change to its inputs.
        // Existing input mods/plugins still participate, including their order.
        let inputs: Vec<_> = self
            .generated_inputs()
            .into_iter()
            .filter(|input| {
                receipt
                    .inputs
                    .iter()
                    .any(|old| old.name.eq_ignore_ascii_case(&input.name))
                    || !produced.iter().any(|file| {
                        file.mod_name
                            .as_ref()
                            .is_some_and(|name| name.eq_ignore_ascii_case(&input.name))
                    })
            })
            .collect();
        let plugins: Vec<_> = plugins
            .iter()
            .filter(|(name, _)| {
                receipt
                    .plugins
                    .iter()
                    .any(|(old, _)| old.eq_ignore_ascii_case(name))
                    || !produced.iter().any(|file| {
                        file.path.parent() == Some(Path::new(""))
                            && file
                                .path
                                .file_name()
                                .is_some_and(|file_name| file_name.eq_ignore_ascii_case(name))
                    })
            })
            .cloned()
            .collect();
        self.active_profile() != receipt.profile
            || self.generated_tool_preset(&receipt.tool) != receipt.tool_preset
            || inputs != receipt.inputs
            || plugins != receipt.plugins
            || receipt
                .executable
                .as_ref()
                .is_some_and(|file| executable_fingerprint(&file.path).as_ref() != Some(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Instance);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "eidos-generated-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let instance = Instance::portable(root);
            instance.create().unwrap();
            Self(instance)
        }
        fn put(&self, path: &str, bytes: &[u8]) {
            let path = self.0.overwrite_dir().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    #[test]
    fn receipts_follow_only_changed_files_across_runs_restarts_and_moves() {
        let f = Fixture::new();
        f.put("unrelated.txt", b"older output");
        let run =
            f.0.begin_tool_run("Generator A", None, &["generator-a.exe".into()], &[])
                .unwrap();
        f.put("a.esp", b"first");
        assert_eq!(f.0.finish_tool_run(run).unwrap(), 1);
        let run =
            f.0.begin_tool_run("Generator B", None, &["generator-b.exe".into()], &[])
                .unwrap();
        f.put("b.esp", b"second");
        assert_eq!(f.0.finish_tool_run(run).unwrap(), 1);
        let reopened = Instance::portable(f.0.root.clone());
        let receipts = reopened.generated_output(None).unwrap();
        assert_eq!(receipts.len(), 2);
        assert!(receipts
            .iter()
            .all(|r| !r.paths.contains(&PathBuf::from("unrelated.txt"))));
        reopened.overwrite_into_mod("Generated").unwrap();
        assert!(reopened.generated_output(None).unwrap().is_empty());
        let captured = reopened.generated_output(Some("Generated")).unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured.iter().flat_map(|r| &r.paths).count(), 2);
        assert!(!f.0.overwrite_dir().join(".generated-output.json").exists());
        assert!(!f
            .0
            .mods_dir()
            .join("Generated/.generated-output.json")
            .exists());
    }

    #[test]
    fn inputs_include_exact_sources_order_profile_and_plugin_activation() {
        let f = Fixture::new();
        let a = f.0.create_empty_mod("A").unwrap();
        let b = f.0.create_empty_mod("B").unwrap();
        f.0.save_modlist(&[a.clone(), b.clone()]).unwrap();
        let mut meta = ModMeta::read(&a.path.join("meta.ini"));
        meta.set("version", "1.0");
        meta.set_installed_files(&[(42, 100), (42, 101)]);
        meta.write(&a.path.join("meta.ini")).unwrap();
        let plugins = vec![("A.esp".to_string(), true), ("B.esp".to_string(), false)];
        let run =
            f.0.begin_tool_run(
                "Generator",
                None,
                &["generator.exe".into(), "--patch".into()],
                &plugins,
            )
            .unwrap();
        assert_eq!(
            run.receipt.inputs[0].installed_files,
            [(42, 100), (42, 101)]
        );
        assert!(!f.0.generated_input_drift(&run.receipt, &plugins));
        f.0.save_modlist(&[b.clone(), a.clone()]).unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &plugins));
        f.0.save_modlist(&[a.clone(), b.clone()]).unwrap();
        meta.set("version", "2.0");
        meta.write(&a.path.join("meta.ini")).unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &plugins));
        meta.set("version", "1.0");
        meta.set_installed_files(&[(42, 102)]);
        meta.write(&a.path.join("meta.ini")).unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &plugins));
        meta.set_installed_files(&[(42, 100), (42, 101)]);
        meta.write(&a.path.join("meta.ini")).unwrap();
        assert!(!f.0.generated_input_drift(&run.receipt, &plugins));
        let mut changed = plugins.clone();
        changed[1].1 = true;
        assert!(f.0.generated_input_drift(&run.receipt, &changed));
        changed = plugins.clone();
        changed.reverse();
        assert!(f.0.generated_input_drift(&run.receipt, &changed));
        f.0.ensure_manifest("skyrimse", crate::InstanceKind::Portable)
            .unwrap();
        f.0.profile("Other").save_modlist(&[a, b]).unwrap();
        f.0.set_active_profile("Other").unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &plugins));
    }
    #[test]
    fn a_pending_run_survives_restart_without_claiming_output() {
        let f = Fixture::new();
        let _run =
            f.0.begin_tool_run("Generator", None, &["generator.exe".into()], &[])
                .unwrap();
        assert!(
            f.0.root.join(RECEIPTS_FILE).is_file(),
            "capture inputs durably before launch"
        );
        f.put("unverified.esp", b"unknown unfinished run");
        let reopened = Instance::portable(f.0.root.clone());
        assert!(reopened.generated_output(None).unwrap().is_empty());
        assert_eq!(reopened.pending_tool_runs().unwrap().len(), 1);
        let later = reopened.begin_tool_run("Later", None, &[], &[]).unwrap();
        reopened.finish_tool_run(later).unwrap();
        assert_eq!(
            reopened.pending_tool_runs().unwrap().len(),
            1,
            "finishing a later run must not clear the interrupted receipt"
        );
        assert!(reopened.generated_output(None).unwrap().is_empty());
    }

    #[test]
    fn newly_captured_output_is_not_itself_input_drift() {
        let f = Fixture::new();
        let input = f.0.create_empty_mod("Input").unwrap();
        f.0.save_modlist(&[input]).unwrap();
        let plugins = vec![("Input.esp".to_string(), true)];
        let run =
            f.0.begin_tool_run("Generator", None, &["generator.exe".into()], &plugins)
                .unwrap();
        let before = run.before.clone();
        f.put("Generated.esp", b"patch");
        f.0.finish_tool_run(run).unwrap();
        f.0.capture_overwrite_into_mod("Generated", &before)
            .unwrap();
        let outputs = f.0.generated_output(Some("Generated")).unwrap();
        let plugins = vec![
            ("Input.esp".to_string(), true),
            ("Generated.esp".to_string(), true),
        ];
        assert!(!f.0.generated_input_drift(&outputs[0].receipt, &plugins));
    }
    #[test]
    fn partial_capture_and_send_to_existing_mod_keep_each_paths_receipt() {
        let f = Fixture::new();
        let existing = f.0.create_empty_mod("Existing").unwrap();
        let metadata = fs::read(existing.path.join("meta.ini")).unwrap();
        let outside = f.0.root.join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, existing.path.join("blocked")).unwrap();
        let run = f.0.begin_tool_run("Generator", None, &[], &[]).unwrap();
        let before = run.before.clone();
        f.put("good.esp", b"good");
        f.put("blocked/file.esp", b"blocked");
        f.0.finish_tool_run(run).unwrap();
        assert!(f.0.capture_overwrite_into_mod("Existing", &before).is_err());
        assert_eq!(
            f.0.generated_output(Some("Existing")).unwrap()[0].paths,
            [PathBuf::from("good.esp")]
        );
        assert_eq!(
            f.0.generated_output(None).unwrap()[0].paths,
            [PathBuf::from("blocked/file.esp")]
        );
        assert!(!outside.join("file.esp").exists());
        fs::remove_file(existing.path.join("blocked")).unwrap();
        let owners = HashMap::from([("blocked/file.esp".into(), "Existing".into())]);
        assert_eq!(f.0.sync_overwrite_to_mods(&owners).unwrap().0, 1);
        assert!(f.0.generated_output(None).unwrap().is_empty());
        assert_eq!(
            f.0.generated_output(Some("Existing")).unwrap()[0]
                .paths
                .len(),
            2
        );
        assert_eq!(fs::read(existing.path.join("meta.ini")).unwrap(), metadata);
        fs::write(existing.path.join("good.esp"), b"external rewrite").unwrap();
        assert_eq!(
            f.0.generated_output(Some("Existing")).unwrap()[0].paths,
            [PathBuf::from("blocked/file.esp")]
        );
    }

    #[test]
    fn unknown_overwrite_cannot_inherit_a_destinations_old_receipt() {
        let f = Fixture::new();
        let run = f.0.begin_tool_run("Generator", None, &[], &[]).unwrap();
        f.put("patch.esp", b"generated");
        f.0.finish_tool_run(run).unwrap();
        f.0.overwrite_into_mod("Output").unwrap();
        f.put("patch.esp", b"untracked replacement");
        f.0.sync_overwrite_to_mods(&HashMap::from([("patch.esp".into(), "Output".into())]))
            .unwrap();
        assert!(f.0.generated_output(Some("Output")).unwrap().is_empty());
    }

    #[test]
    fn corrupt_receipts_fail_before_moving_output() {
        let f = Fixture::new();
        f.put("patch.esp", b"precious");
        fs::write(f.0.root.join(RECEIPTS_FILE), b"not json").unwrap();
        assert!(f.0.overwrite_into_mod("Output").is_err());
        assert_eq!(
            fs::read(f.0.overwrite_dir().join("patch.esp")).unwrap(),
            b"precious"
        );
        assert!(!f.0.mods_dir().join("Output").exists());
        assert!(f.0.begin_tool_run("Generator", None, &[], &[]).is_err());
    }

    #[test]
    fn a_failed_receipt_write_retains_pending_context_and_unattributed_output() {
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::new();
        let executable = f.0.root.join("generator.exe");
        fs::write(&executable, b"tool version one").unwrap();
        let run =
            f.0.begin_tool_run(
                "Generator",
                Some(&executable),
                &["virtual/generator.exe".into(), "--patch".into()],
                &[],
            )
            .unwrap();
        assert_eq!(run.receipt.executable.as_ref().unwrap().path, executable);
        assert_eq!(run.receipt.command[1], "--patch");
        f.put("partial.esp", b"partial output");
        fs::set_permissions(&f.0.root, fs::Permissions::from_mode(0o555)).unwrap();
        let result = f.0.finish_tool_run(run.clone());
        fs::set_permissions(&f.0.root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert_eq!(f.0.pending_tool_runs().unwrap().len(), 1);
        assert!(f.0.generated_output(None).unwrap().is_empty());
        f.0.finish_tool_run(run.clone()).unwrap();
        assert!(f.0.pending_tool_runs().unwrap().is_empty());
        assert!(!f.0.generated_input_drift(&run.receipt, &[]));
        fs::write(&executable, b"tool version two is different").unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &[]));
    }
    #[test]
    fn refused_receipt_transfer_leaves_output_in_overwrite() {
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::new();
        let run = f.0.begin_tool_run("Generator", None, &[], &[]).unwrap();
        f.put("patch.esp", b"generated");
        f.0.finish_tool_run(run).unwrap();
        fs::set_permissions(&f.0.root, fs::Permissions::from_mode(0o555)).unwrap();
        let result = f.0.overwrite_into_mod("Output");
        fs::set_permissions(&f.0.root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert!(f.0.overwrite_dir().join("patch.esp").is_file());
        assert!(!f.0.mods_dir().join("Output").exists());
    }

    #[test]
    fn changed_tool_preset_is_reported_as_drift() {
        let f = Fixture::new();
        let mut tool = crate::Tool {
            title: "Generator".into(),
            exe: "generator.exe".into(),
            args: vec!["--old".into()],
            ..Default::default()
        };
        f.0.save_tools(&[tool.clone()]).unwrap();
        let run =
            f.0.begin_tool_run(
                &tool.title,
                None,
                &["generator.exe".into(), "--old".into()],
                &[],
            )
            .unwrap();
        assert!(!f.0.generated_input_drift(&run.receipt, &[]));
        tool.args = vec!["--new".into()];
        f.0.save_tools(&[tool]).unwrap();
        assert!(f.0.generated_input_drift(&run.receipt, &[]));
    }
}
