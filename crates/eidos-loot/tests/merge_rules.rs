//! Merging a collection's plugin rules into a real userlist, through libloot.
//!
//! Against the real library rather than a mock, because the whole risk here is
//! libloot's opinion of the file: a rule naming a group nothing defines makes it
//! refuse to sort AT ALL, and the symptom is an empty load order with no
//! explanation. That is not something a mock can be wrong about convincingly.

use std::fs;
use std::path::PathBuf;

use eidos_loot::{merge_user_rules, GameView, GroupDef, UserRule};

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "eidos-loot-merge-{}-{name}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

struct Bench {
    root: PathBuf,
    masterlist: PathBuf,
    prelude: PathBuf,
    userlist: PathBuf,
    game: PathBuf,
    local: PathBuf,
}

fn bench(name: &str) -> Bench {
    let root = tmp(name);
    let (game, local) = (root.join("game"), root.join("local"));
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::create_dir_all(&local).unwrap();
    let masterlist = root.join("masterlist.yaml");
    // One group, so "a group that exists" and "a group that does not" are both
    // testable without inventing what LOOT ships.
    fs::write(
        &masterlist,
        "groups:\n  - name: 'Early Loaders'\n    after: ['default']\n",
    )
    .unwrap();
    let prelude = root.join("prelude.yaml");
    fs::write(&prelude, "common: {}\n").unwrap();
    Bench {
        userlist: root.join("userlist.yaml"),
        root,
        masterlist,
        prelude,
        game,
        local,
    }
}

impl Bench {
    fn view(&self) -> GameView<'_> {
        GameView {
            game_id: "skyrimse",
            game_path: &self.game,
            local_path: &self.local,
            plugins: &[],
            mod_dirs: &[],
            masterlist: &self.masterlist,
            prelude: &self.prelude,
            userlist: Some(&self.userlist),
        }
    }
}

#[test]
fn a_collections_rules_reach_the_userlist_and_libloot_reads_them_back() {
    let b = bench("basic");
    let rules = [UserRule {
        plugin: "A.esp".into(),
        after: vec!["B.esp".into()],
        group: Some("Early Loaders".into()),
    }];
    let m = merge_user_rules(&b.view(), &rules, &[]).expect("merge");
    assert_eq!(m.rules_added, 1);
    assert!(m.dangling_groups.is_empty(), "{:?}", m.dangling_groups);
    assert!(b.userlist.is_file(), "the userlist was written");

    // Read back THROUGH libloot, not by grepping the text: the question is
    // whether the library agrees the file says what we meant.
    let m2 = merge_user_rules(&b.view(), &rules, &[]).expect("second merge");
    assert_eq!(
        m2.rules_added, 0,
        "the same rule twice adds nothing the second time"
    );
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn a_group_that_nothing_defines_is_dropped_rather_than_written() {
    // The reason this matters: LOOT refuses to sort at all when a plugin names
    // a group that does not exist, and the user sees no load order and no
    // explanation. Dropping the assignment loses one rule; writing it loses the
    // whole sort.
    let b = bench("dangling");
    let rules = [UserRule {
        plugin: "A.esp".into(),
        after: vec![],
        group: Some("A Group Nobody Defines".into()),
    }];
    let m = merge_user_rules(&b.view(), &rules, &[]).expect("merge");
    assert_eq!(m.rules_added, 0);
    assert_eq!(m.dangling_groups.len(), 1);
    assert!(m.dangling_groups[0].contains("A Group Nobody Defines"));
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn a_group_the_collection_defines_makes_its_own_assignment_legal() {
    let b = bench("defines");
    let groups = [GroupDef {
        name: "Collection Late".into(),
        after: vec!["default".into()],
    }];
    let rules = [UserRule {
        plugin: "A.esp".into(),
        after: vec![],
        group: Some("Collection Late".into()),
    }];
    let m = merge_user_rules(&b.view(), &rules, &groups).expect("merge");
    assert_eq!(m.groups_added, 1);
    assert_eq!(m.rules_added, 1);
    assert!(m.dangling_groups.is_empty(), "{:?}", m.dangling_groups);
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn the_users_own_group_choice_survives_the_collection() {
    // A collection is a suggestion about a mod list the user owns.
    let b = bench("userwins");
    fs::write(
        &b.userlist,
        "plugins:\n  - name: 'A.esp'\n    group: 'Early Loaders'\n",
    )
    .unwrap();
    let rules = [UserRule {
        plugin: "A.esp".into(),
        after: vec![],
        group: Some("Early Loaders".into()),
    }];
    let m = merge_user_rules(&b.view(), &rules, &[]).expect("merge");
    assert_eq!(m.kept_user_group, vec!["A.esp".to_string()]);
    assert_eq!(m.rules_added, 0, "nothing of the user's was overwritten");
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn an_after_rule_is_added_to_what_is_there_rather_than_replacing_it() {
    let b = bench("union");
    fs::write(
        &b.userlist,
        "plugins:\n  - name: 'A.esp'\n    after:\n      - 'Mine.esp'\n",
    )
    .unwrap();
    let rules = [UserRule {
        plugin: "A.esp".into(),
        after: vec!["Theirs.esp".into()],
        group: None,
    }];
    let m = merge_user_rules(&b.view(), &rules, &[]).expect("merge");
    assert_eq!(m.rules_added, 1);
    let text = fs::read_to_string(&b.userlist).unwrap();
    assert!(text.contains("Mine.esp"), "the user's rule survives:\n{text}");
    assert!(text.contains("Theirs.esp"), "and the collection's is added:\n{text}");
    let _ = fs::remove_dir_all(&b.root);
}
