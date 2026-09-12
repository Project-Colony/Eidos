use std::{fs, path::{Path, PathBuf}, process::Command};
use eidos_collections::{driver::RealHooks, install::{Hooks, Installed}, manifest::{Mod, Source}, state::{InstallState, Status, key_for}};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!("eidos-collection-ownership-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

fn archive(root: &Path) -> PathBuf {
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("scripts")).unwrap();
    fs::write(payload.join("scripts/new.pex"), b"collection payload").unwrap();
    let archive = root.join("member.zip");
    let status = Command::new(eidos_sevenzip::find_7z().expect("7-Zip is required for installer tests"))
        .current_dir(&payload).args(["a", "-tzip"]).arg(&archive).arg("scripts")
        .stdout(std::process::Stdio::null()).status().unwrap();
    assert!(status.success());
    archive
}

#[test]
fn a_previously_unavailable_member_does_not_own_an_existing_personal_mod() {
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let personal = inst.mods_dir().join("Popular Mod");
    fs::create_dir_all(personal.join("scripts")).unwrap();
    fs::write(personal.join("scripts/personal.pex"), b"personal data").unwrap();
    let def = eidos_games::catalog().iter().find(|g| g.id == "skyrimse").unwrap();
    let game = eidos_games::DetectedGame { def, install_path: temp.0.join("game"), data_path: temp.0.join("game/Data"), compatdata: None, steam_name: "test".into() };
    fs::create_dir_all(&game.data_path).unwrap();
    let member = Mod { name: "Popular Mod".into(), source: Source { file_id: Some(1), mod_id: Some(1), ..Default::default() }, ..Default::default() };
    let mut state = InstallState::default();
    state.set(&key_for(&member, "skyrimspecialedition"), Status::Unavailable("manual download needed".into()));
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-no-network");
    let mut say = |_: String| {};
    let mut hooks = RealHooks { nexus: &nexus, inst: &inst, game: &game, game_id: "skyrimse".into(), say: &mut say, collection_domain: "skyrimspecialedition".into(), owner: "test:1".into(), renamed: vec![], };
    let folder = hooks.reserve(&member, state.folders.get(&key_for(&member, "skyrimspecialedition")).map(String::as_str)).unwrap();
    let installed = hooks.install(&member, &archive(&temp.0), &folder);
    assert!(matches!(installed, Installed::Ok(_)), "{installed:?}");
    assert_eq!(fs::read(personal.join("scripts/personal.pex")).unwrap(), b"personal data");
    assert!(!personal.join("scripts/new.pex").exists());
    assert!(inst.mods_dir().join("Popular Mod (2)/scripts/new.pex").is_file());
    // A retry must reuse its actual renamed destination, not the original personal mod.
    assert_eq!(hooks.reserve(&member, Some(&folder)).unwrap(), folder);
    assert!(hooks.reserve(&member, Some("../outside")).is_err());
    assert!(hooks.reserve(&member, Some("Popular Mod")).is_err());
    assert!(hooks.verify_installed(&member, &folder).unwrap());
    assert!(hooks.verify_installed(&member, "Popular Mod").is_err());
    assert!(!hooks.verify_installed(&member, "Gone").unwrap());
    eidos_install::install_archive_with_policy(&temp.0.join("member.zip"), &inst.mods_dir(), &folder, "skyrimse", eidos_install::OverwritePolicy::Replace, &Default::default()).unwrap();
    assert!(hooks.verify_installed(&member, &folder).is_err(), "a manual replacement revokes collection authority");
    // Modifying or removing the marker revokes replacement authority.
    fs::write(inst.mods_dir().join(&folder).join("meta.ini"), "[General]\nmodid=1\n").unwrap();
    assert!(hooks.reserve(&member, Some(&folder)).is_err());
}

#[test]
fn collection_ini_output_reserves_its_own_folder_and_stops_on_checkpoint_failure() {
    use eidos_collections::{driver::apply_ini_tweaks, manifest::Collection, report::Report};
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let mut collection = Collection::default();
    collection.info.name = "My Collection".into();
    let personal = inst.mods_dir().join("My Collection - INI Tweaks/Ini Tweaks/settings.ini");
    fs::create_dir_all(personal.parent().unwrap()).unwrap();
    fs::write(&personal, b"personal").unwrap();
    let source = temp.0.join("collection");
    fs::create_dir_all(source.join("INI Tweaks")).unwrap();
    fs::write(source.join("INI Tweaks/settings.ini"), b"collection").unwrap();
    let mut state = InstallState { slug: "one".into(), revision: 1, game_domain: "skyrimspecialedition".into(), ..Default::default() };
    let mut report = Report::default();
    apply_ini_tweaks(&inst, &source, &collection, &mut state, &mut |_| Err("disk full".into()), &mut report);
    assert!(report.aborted);
    assert_eq!(fs::read(&personal).unwrap(), b"personal");
    let folder = state.folders["aux:ini-tweaks"].clone();
    let output = inst.mods_dir().join(&folder).join("Ini Tweaks/settings.ini");
    assert!(!output.exists());
    let mut report = Report::default();
    let state_path = temp.0.join("state.json");
    apply_ini_tweaks(&inst, &source, &collection, &mut state, &mut |s| s.save(&state_path).map_err(|e| e.to_string()), &mut report);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(fs::read(output).unwrap(), b"collection");
    assert_eq!(fs::read(personal).unwrap(), b"personal");
    assert_eq!(state.folders["aux:ini-tweaks"], folder);
}

#[test]
fn a_stale_download_sidecar_does_not_hide_a_later_complete_archive() {
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    fs::create_dir_all(inst.downloads_dir()).unwrap();
    let def = eidos_games::catalog().iter().find(|g| g.id == "skyrimse").unwrap();
    let game = eidos_games::DetectedGame { def, install_path: temp.0.join("game"), data_path: temp.0.join("game/Data"), compatdata: None, steam_name: "test".into() };
    for name in ["first.zip.meta", "second.zip.meta"] {
        fs::write(inst.downloads_dir().join(name), "[General]\ngameName=SkyrimSE\nmodID=1\nfileID=1\n").unwrap();
    }
    let sidecars: Vec<_> = fs::read_dir(inst.downloads_dir()).unwrap().map(|e| e.unwrap().path()).collect();
    let complete = sidecars[1].with_extension("");
    fs::write(&complete, b"archive").unwrap();
    let member = Mod { name: "Member".into(), source: Source { kind: eidos_collections::manifest::SourceType::Browse, file_id: Some(1), mod_id: Some(1), ..Default::default() }, ..Default::default() };
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-no-network");
    let mut say = |_: String| {};
    let mut hooks = RealHooks { nexus: &nexus, inst: &inst, game: &game, game_id: "skyrimse".into(), say: &mut say, collection_domain: "skyrimspecialedition".into(), owner: "test:1".into(), renamed: vec![] };
    assert_eq!(hooks.obtain(&member), eidos_collections::install::Obtained::Ready(complete));
}
