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
        group: Some("Late Loaders".into()),
    }];
    let m = merge_user_rules(&b.view(), &rules, &[]).expect("merge");
    assert_eq!(m.kept_user_group, vec!["A.esp".to_string()]);
    assert_eq!(m.rules_added, 0, "nothing of the user's was overwritten");
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn asking_for_the_group_that_is_already_set_is_not_a_disagreement() {
    // Re-running is the documented way to finish an interrupted collection, and
    // this function runs at the end of every run - so the second pass always
    // meets its own output. Calling that "the user decided otherwise" would put
    // a line in every re-run's report and bury the real ones.
    let b = bench("samegroup");
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
    assert!(m.kept_user_group.is_empty(), "{:?}", m.kept_user_group);
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn a_collection_group_whose_after_names_nothing_is_not_written() {
    // libloot builds the group graph before it sorts ANYTHING, so one undefined
    // name in an `after` list makes every later sort of this instance fail -
    // including the ones the user runs long after the collection is forgotten.
    let b = bench("danglingafter");
    let groups = [GroupDef {
        name: "Collection Late".into(),
        after: vec!["default".into(), "A Group Nobody Defines".into()],
    }];
    let m = merge_user_rules(&b.view(), &[], &groups).expect("merge");
    assert_eq!(m.groups_added, 1);
    assert_eq!(m.dangling_after.len(), 1, "{:?}", m.dangling_after);
    assert!(m.dangling_after[0].contains("A Group Nobody Defines"));
    let written = fs::read_to_string(&b.userlist).unwrap();
    assert!(
        !written.contains("A Group Nobody Defines"),
        "it must not reach the file: {written}"
    );
    // And the file libloot has to read back is one libloot accepts.
    let m2 = merge_user_rules(&b.view(), &[], &groups).expect("the userlist reloads");
    assert_eq!(m2.groups_added, 0, "the second run adds nothing");
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn one_group_named_twice_does_not_produce_a_userlist_libloot_refuses() {
    let b = bench("dupgroup");
    let groups = [
        GroupDef {
            name: "Collection Late".into(),
            after: vec!["default".into()],
        },
        GroupDef {
            name: "Collection Late".into(),
            after: vec!["default".into()],
        },
    ];
    let m = merge_user_rules(&b.view(), &[], &groups).expect("merge");
    assert_eq!(m.groups_added, 1, "one group, not two");
    // The proof is that libloot can load what was written.
    merge_user_rules(&b.view(), &[], &[]).expect("the userlist reloads");
    let _ = fs::remove_dir_all(&b.root);
}

#[test]
fn a_collection_extending_an_existing_group_is_applied_not_dropped() {
    // In LOOT a userlist group sharing a name with an existing one does not
    // replace it, it adds to its `after` list. Dropping the collection's entry
    // silently discards the author's placement.
    let b = bench("extend");
    fs::write(
        &b.userlist,
        "groups:\n  - name: 'Collection Late'\n    after:\n      - default\n",
    )
    .unwrap();
    let groups = [GroupDef {
        name: "Collection Late".into(),
        after: vec!["Early Loaders".into()],
    }];
    // "Early Loaders" is a real masterlist group in this bench's masterlist.
    let m = merge_user_rules(&b.view(), &[], &groups).expect("merge");
    assert_eq!(m.groups_extended, vec!["Collection Late".to_string()]);
    let written = fs::read_to_string(&b.userlist).unwrap();
    assert!(written.contains("Early Loaders"), "{written}");
    assert!(written.contains("default"), "the old entry survives: {written}");
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
