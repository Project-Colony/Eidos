use super::*;
use crate::test_support::*;

#[test]
fn source_selection_rechecks_the_known_instances_current_manifest() {
    let root = temp_portable("skyrimse");
    let mut app = app_for_game("skyrimse");
    let mut second = app.games[0].clone();
    second.install_path = "/second/skyrim".into();
    app.games.push(second);
    let inst = Instance::portable(root.clone());
    app.known = vec![KnownInstance {
        label: "portable".into(),
        inst: inst.clone(),
        game_index: 0,
        portable: true,
    }];
    inst.ensure_installation(
        "skyrimse",
        InstanceKind::Portable,
        &app.games[1].selection_id(),
    )
    .unwrap();
    let _ = update_inner(&mut app, Message::OpenKnown(0));
    assert_eq!(app.selected, Some(1));
    eidos_instance::Manifest::new("fallout4", InstanceKind::Portable)
        .write(&inst.manifest_path())
        .unwrap();
    assert_eq!(instance_game_index(&app.games, &inst, "skyrimse"), None);
    app.created = None;
    app.screen = Screen::Welcome;
    let _ = update_inner(&mut app, Message::OpenKnown(0));
    assert!(app.created.is_none());
    assert_eq!(inst.read_manifest().unwrap().game_id, "fallout4");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn extension_picker_rejects_stale_requests_targets_and_views_before_starting_a_helper() {
    let (mut app, root) = data_app(&[], &[]);
    let path = root.join("example.txt");
    fs::write(&path, "example").unwrap();
    let original = update::collection_target(&app);
    let epoch = app.archive_epoch.get();
    for mismatch in 0..3 {
        app.extension_picker = 7;
        let mut target = original.clone();
        if mismatch == 1 {
            target.as_mut().unwrap().profile = "another profile".into();
        }
        let _ = update_inner(
            &mut app,
            Message::ExtensionFilePicked {
                request: if mismatch == 0 { 6 } else { 7 },
                target,
                epoch: if mismatch == 2 {
                    epoch.wrapping_sub(1)
                } else {
                    epoch
                },
                operation: eidos_addons::protocol::Operation::Preview,
                path: Some(path.clone()),
            },
        );
        assert!(
            app.preview_pending.is_none(),
            "started a stale helper: mismatch {mismatch}"
        );
    }
    let _ = update_inner(
        &mut app,
        Message::ExtensionFilePicked {
            request: 7,
            target: original,
            epoch,
            operation: eidos_addons::protocol::Operation::Preview,
            path: Some(path),
        },
    );
    assert!(app.preview_pending.is_some());
    let _ = update_inner(&mut app, Message::ClosePreview);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn open_known_switches_to_the_chosen_portable_instance() {
    // The reported gap: a portable instance existed on disk but nothing
    // could ever OPEN it again. The welcome list entry must actually open.
    let root = temp_portable("skyrimse");
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Welcome;
    app.known = vec![KnownInstance {
        label: "Skyrim SE - portable".into(),
        inst: Instance::portable(root.clone()),
        game_index: 0,
        portable: true,
    }];
    let _ = update_inner(&mut app, Message::OpenKnown(0));
    assert_eq!(
        app.created.as_ref().map(|i| i.root.clone()),
        Some(root.clone())
    );
    assert!(
        matches!(app.screen, Screen::Main),
        "opening must land on the main screen"
    );
    assert_eq!(app.selected, Some(0));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_known_on_a_missing_root_says_so_instead_of_wedging() {
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Welcome;
    app.known = vec![KnownInstance {
        label: "gone".into(),
        inst: Instance::portable(PathBuf::from("/nonexistent/eidos-test-root")),
        game_index: 0,
        portable: true,
    }];
    let _ = update_inner(&mut app, Message::OpenKnown(0));
    assert!(app.created.is_none(), "a dead root must not fake an open");
    assert!(matches!(app.screen, Screen::Welcome));
    assert!(
        app.status
            .as_deref()
            .unwrap_or("")
            .contains("not reachable"),
        "the skip must be said, not silent: {:?}",
        app.status
    );
}

#[test]
fn finish_refuses_to_relabel_a_foreign_portable_folder() {
    // ensure_manifest keeps an existing manifest, so before this check a
    // fallout4 folder adopted under a skyrimse wizard kept its old game id
    // while everything else treated it as Skyrim - a silent mislabel.
    let root = temp_portable("fallout4");
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Summary;
    app.kind = InstanceKind::Portable;
    app.name = "Mine".into();
    app.portable_path = root.display().to_string();
    let _ = update_inner(&mut app, Message::Finish);
    assert!(app.created.is_none(), "adoption must refuse, not relabel");
    assert!(
        app.error.as_deref().unwrap_or("").contains("fallout4"),
        "the refusal must name the folder's real game: {:?}",
        app.error
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn finish_refuses_a_root_inside_the_game_install() {
    // The MO2-veteran reflex: put the manager in the game folder. Steam
    // owns that tree and Eidos mounts over it - the wizard must say no.
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Summary;
    app.kind = InstanceKind::Portable;
    app.name = "Mine".into();
    // app_for_game's install_path is /nowhere.
    app.portable_path = "/nowhere/Eidos".into();
    let _ = update_inner(&mut app, Message::Finish);
    assert!(
        app.created.is_none(),
        "an instance inside the install must not be created"
    );
    assert!(
        app.error.as_deref().unwrap_or("").contains("own folder"),
        "the refusal must explain itself: {:?}",
        app.error
    );
    assert!(
        !Path::new("/nowhere/Eidos").exists(),
        "nothing may be created on refusal"
    );
}

#[test]
fn finish_adopts_a_matching_portable_folder() {
    let root = temp_portable("skyrimse");
    fs::create_dir_all(root.join("mods/Existing Mod")).unwrap();
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Summary;
    app.kind = InstanceKind::Portable;
    app.name = "Mine".into();
    app.portable_path = root.display().to_string();
    let _ = update_inner(&mut app, Message::Finish);
    assert_eq!(
        app.created.as_ref().map(|i| i.root.clone()),
        Some(root.clone())
    );
    assert!(matches!(app.screen, Screen::Main));
    assert!(
        app.mods.iter().any(|m| m.name == "Existing Mod"),
        "adoption must see the folder's own mods"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn switching_instance_drops_the_previous_games_merged_view() {
    // The Data tab is memoised per directory against `view_generation`, so a
    // switch that does not bump it answers every already-listed directory out
    // of the OLD instance. Going from Skyrim to a game with no mods at all
    // still drew Skyrim's merged tree, `[skyrimse]` provenance and all.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "eidos-switch-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));

    let mut app = app_for_game("stellarblade");
    app.kind = InstanceKind::Portable;
    app.portable_path = root.to_string_lossy().into_owned();

    // Stand in for what browsing the previous instance's Data tab leaves behind.
    app.data_listing.borrow_mut().insert(
        String::new(),
        (
            app.view_generation.get(),
            std::rc::Rc::new(vec![DataRow {
                name: "SKSE".into(),
                source: "[skyrimse]".into(),
                is_dir: true,
                real: PathBuf::from("/old/SKSE"),
                size: None,
                mtime: None,
                conflicted: false,
            }]),
        ),
    );
    app.listing_cache.borrow_mut().insert(
        PathBuf::from("/old"),
        (
            app.view_generation.get(),
            std::rc::Rc::new(vec!["stale".to_string()]),
        ),
    );
    app.files_cache
        .borrow_mut()
        .insert("OldMod".into(), (vec!["a.esp".into()], false));
    app.data_expanded.insert("meshes".to_string());

    let _ = update(&mut app, Message::Finish);
    assert!(app.created.is_some(), "the instance was created");

    assert!(
        app.data_listing.borrow().is_empty(),
        "the merged listing survived the switch"
    );
    assert!(
        app.listing_cache.borrow().is_empty(),
        "a directory listing survived the switch"
    );
    assert!(
        app.files_cache.borrow().is_empty(),
        "a mod's file list survived the switch"
    );
    assert!(
        app.data_expanded.is_empty(),
        "expanded paths from the old game survived"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn releasing_outside_the_list_still_drops_where_you_aimed() {
    // The bug this replaces: cancelling on pointer EXIT meant dragging up
    // past the header - the only way to reach an earlier row - dropped the
    // mod and cleared the selection every time. Leaving and letting go are
    // indistinguishable to `on_exit`, so the release is caught globally.
    let mut app = nav_app(&["a", "b", "c", "d"]);
    let _ = update(&mut app, Message::DragStart(3));
    let _ = update(&mut app, Message::DragOverGap(1));
    assert!(
        app.drag_state.is_some_and(|d| d.aimed),
        "the gap was aimed at"
    );

    // Released with the pointer anywhere at all.
    let _ = update(&mut app, Message::PointerReleased);
    assert!(app.drag_state.is_none(), "the drag ended");
    assert_eq!(
        names(&app.mods),
        vec!["a", "d", "b", "c"],
        "it moved where it aimed"
    );
}

#[test]
fn a_long_drag_still_drops_after_the_list_has_scrolled_under_it() {
    // The bug: dragging a block far enough that the auto-scroll took over
    // put the pointer on a scroll band, which is not a drop strip. A second
    // release handler on the list then cancelled the drag before the global
    // one could commit it, so a long drag never landed. One handler decides
    // now, and it only ever looks at where the drag was AIMED.
    let mut app = nav_app(&["a", "b", "c", "d", "e"]);
    app.selected_mods = [3usize, 4].into_iter().collect();
    let _ = update(&mut app, Message::DragStart(3));
    let _ = update(&mut app, Message::DragOverGap(0));

    // The list scrolls under the pointer; the aim does not change.
    let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Up)));
    let _ = update(&mut app, Message::DragScrollTick);
    assert!(
        app.drag_state.is_some_and(|d| d.aimed),
        "the scroll disarmed the drag"
    );

    let _ = update(&mut app, Message::PointerReleased);
    assert_eq!(
        names(&app.mods),
        vec!["d", "e", "a", "b", "c"],
        "the block did not land"
    );
    assert!(
        app.drag_scroll.is_none(),
        "the scroll timer outlived the drag"
    );
}

#[test]
fn a_release_that_aimed_at_nothing_is_a_click_not_a_move() {
    // A plain click arms a drag too. Releasing it must not reorder anything.
    let mut app = nav_app(&["a", "b", "c"]);
    let _ = update(&mut app, Message::DragStart(2));
    let _ = update(&mut app, Message::PointerReleased);
    assert!(app.drag_state.is_none());
    assert_eq!(names(&app.mods), vec!["a", "b", "c"], "a click moved a row");
}

#[test]
fn the_auto_scroll_only_runs_while_a_drag_does() {
    let mut app = nav_app(&["a", "b", "c"]);
    // No drag: entering a band cannot start anything. The bands are not even
    // rendered then, but a stale message must not be enough on its own.
    let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Up)));
    assert!(app.drag_scroll.is_none());

    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Down)));
    assert_eq!(app.drag_scroll, Some(ScrollEdge::Down));

    // Ending the drag stops it, so no timer outlives the gesture.
    let _ = update(&mut app, Message::PointerReleased);
    assert!(app.drag_scroll.is_none());
}

#[test]
fn the_auto_scroll_speeds_up_toward_the_edge() {
    // Depth 0 is the inner lip of the band, 1.0 hard against the edge of the
    // list. The point of the range: one speed can creep to the row just off
    // screen OR cross a 250-mod list, never both.
    let mut app = nav_app(&["a", "b", "c"]);
    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragScrollEdge(Some(ScrollEdge::Down)));
    // Entering starts mid-range rather than at full speed, since `on_move`
    // has not fired yet and a lurch is worse than a slow start.
    assert!((app.drag_scroll_depth - 0.5).abs() < f32::EPSILON);

    let _ = update(&mut app, Message::DragScrollDepth(0.0));
    assert_eq!(app.drag_scroll_depth, 0.0);
    let _ = update(&mut app, Message::DragScrollDepth(1.0));
    assert_eq!(app.drag_scroll_depth, 1.0);

    // Out-of-range values are clamped, not trusted: the depth comes from a
    // pointer position divided by a height, and both can surprise.
    let _ = update(&mut app, Message::DragScrollDepth(4.2));
    assert_eq!(app.drag_scroll_depth, 1.0);
    let _ = update(&mut app, Message::DragScrollDepth(-1.0));
    assert_eq!(app.drag_scroll_depth, 0.0);
}

#[test]
fn the_slow_end_is_still_slower_than_the_fast_end() {
    // Guards the constants against being edited into each other's order,
    // which would make the band feel arbitrary rather than aimable.
    assert!(DRAG_SCROLL_SLOW_PX > 0.0, "the shallow end must still move");
    assert!(
        DRAG_SCROLL_FAST_PX > DRAG_SCROLL_SLOW_PX * 2.0,
        "the range is too narrow to be worth having"
    );
}

#[test]
fn a_tick_without_a_drag_does_nothing_and_disarms() {
    // The shape that broke this before: the scroll must never act on state
    // it kept about the list. A tick with no drag behind it does nothing at
    // all rather than moving the view somewhere it remembered.
    let mut app = nav_app(&["a", "b", "c"]);
    app.drag_scroll = Some(ScrollEdge::Up);
    let _ = update(&mut app, Message::DragScrollTick);
    assert!(
        app.drag_scroll.is_none(),
        "a tick with no drag left the edge armed"
    );
}

#[test]
fn a_press_alone_is_not_yet_a_drag() {
    // What the auto-scroll bands key off. `DragStart` fires on PRESS, so
    // keying them off "a drag exists" put them under the pointer on every
    // click - and a `mouse_area` laid out beneath a stationary cursor
    // publishes `on_enter` at once, which is what launched the list.
    let mut app = nav_app(&["a", "b", "c"]);
    let _ = update(&mut app, Message::DragStart(1));
    assert!(app.drag_state.is_some(), "the press armed a drag");
    assert!(
        !app.drag_state.is_some_and(|d| d.aimed),
        "but nothing is aimed at yet"
    );

    let _ = update(&mut app, Message::DragOverGap(0));
    assert!(
        app.drag_state.is_some_and(|d| d.aimed),
        "crossing a gap is a real drag"
    );
}

#[test]
fn typing_in_a_field_takes_the_navigation_keys_away_from_the_list() {
    // iced's on_key_press is a global subscription and cannot see which
    // widget holds the caret, so a space typed into the filter box would
    // otherwise toggle a mod.
    let mut app = nav_app(&["a", "b", "c"]);
    app.selected_mod = Some(0);
    let _ = update(&mut app, Message::SearchChanged("te".to_string()));
    assert!(app.typing);

    // Pressing a row hands it straight back.
    let _ = update(&mut app, Message::SelectMod(1));
    assert!(!app.typing);
    // As does Escape, which is the way out when the pointer is not involved.
    let _ = update(&mut app, Message::SearchChanged("te".to_string()));
    let _ = update(&mut app, Message::ClearSelection);
    assert!(!app.typing);
}

#[test]
fn every_main_screen_field_hands_the_keyboard_over() {
    // A field that forgets to do this is invisible until someone types a
    // space into it and a mod turns off.
    for msg in [
        Message::SearchChanged("x".into()),
        Message::RenameChanged("x".into()),
        Message::NotesChanged("x".into()),
        Message::OverwriteToModName("x".into()),
        Message::SendToPriorityChanged("x".into()),
        Message::ProfileRenameChanged("x".into()),
        Message::ProfileCopyChanged("x".into()),
        Message::PickerNameChanged("x".into()),
    ] {
        let mut app = nav_app(&["a"]);
        let label = format!("{msg:?}");
        let _ = update(&mut app, msg);
        assert!(app.typing, "{label} did not claim the keyboard");
    }
}

#[test]
fn the_keyboard_leaves_the_games_own_content_alone() {
    // Unmanaged rows are not in modlist.txt, so a flipped flag is lost on
    // the next save - which reads as the key having done nothing.
    let mut app = nav_app(&["dlc", "mod"]);
    app.mods[0].unmanaged = true;
    app.selected_mod = Some(0);
    let before = app.mods[0].enabled;
    let _ = update(&mut app, Message::ToggleMod(0));
    assert_eq!(
        app.mods[0].enabled, before,
        "unmanaged content is not togglable"
    );

    // And Delete refuses it outright.
    let _ = key_nav(&mut app, Nav::Remove);
    assert_eq!(app.confirm_remove, None);
}

#[test]
fn delete_twice_actually_removes_and_escape_calls_it_off() {
    // The first version armed a guard the keyboard could not confirm, while
    // telling the user to press Delete again. The promise has to be true.
    let mut app = nav_app(&["a", "b"]);
    app.selected_mod = Some(1);
    let _ = key_nav(&mut app, Nav::Remove);
    assert_eq!(app.confirm_remove, Some(1));
    assert!(app
        .status
        .as_deref()
        .unwrap_or_default()
        .contains("Delete again"));

    // Escape is the advertised way out.
    let _ = update(&mut app, Message::ClearSelection);
    assert_eq!(app.confirm_remove, None);
}

#[test]
fn a_click_on_a_selected_row_does_not_reorder_the_list() {
    // A press arms a drag, so a plain click arrives as a drop. With a
    // multi-row selection there is no "own edge" to recognise it by, and
    // committing would COMPACT a non-contiguous set and save that.
    let mut app = nav_app(&["a", "b", "c", "d", "e"]);
    app.selected_mods = [0, 2, 4].into_iter().collect();
    app.selected_mod = Some(2);
    let _ = update(&mut app, Message::DragStart(2));
    assert!(
        app.drag_state.is_some_and(|d| !d.aimed),
        "a press has aimed at nothing yet"
    );
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(
        names(&app.mods),
        ["a", "b", "c", "d", "e"],
        "a click moved rows"
    );

    // Actually aiming somewhere still works.
    let _ = update(&mut app, Message::DragStart(2));
    let _ = update(&mut app, Message::DragOverGap(0));
    assert!(app.drag_state.is_some_and(|d| d.aimed));
}

/// The indices of a selection, sorted, for a readable assertion.
fn sel(app: &App) -> Vec<usize> {
    let mut v: Vec<usize> = app.selected_mods.iter().copied().collect();
    v.sort_unstable();
    v
}

#[test]
fn a_separator_dragged_alone_leaves_its_mods_behind() {
    // MO2's behaviour, and the whole point of the fix: `dropMimeData` hands
    // exactly the dragged rows to `changeModPriority` (modlist.cpp:1159),
    // gathering no children. The mods left behind are not orphaned - they now
    // belong to whatever header is above them, because membership is nothing
    // but adjacency.
    let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragOverGap(4));
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(
        names(&app.mods),
        ["a", "b", "Tail_separator", "Head_separator", "c"]
    );
}

#[test]
fn a_folded_separator_still_moves_alone_and_comes_back_open() {
    // MO2 force-expands a separator whose priority just changed
    // (ModListView::onModPrioritiesChanged, modlistview.cpp:449). Without it a
    // folded header dropped somewhere new goes on hiding rows that were never
    // inside it, which reads as mods having been deleted.
    let mut app = nav_app(&["Head_separator", "a", "Tail_separator", "b"]);
    app.collapsed.insert("Head".to_string());
    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragOverGap(3));
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(
        names(&app.mods),
        ["a", "Tail_separator", "Head_separator", "b"]
    );
    assert!(
        !app.collapsed.contains("Head"),
        "a header that now hides rows must be open"
    );

    // Landing with nothing under it hides nothing, so the fold is left alone -
    // the user's choice is only overridden where keeping it would mislead.
    let mut app = nav_app(&["a", "Head_separator", "b"]);
    app.collapsed.insert("Head".to_string());
    let _ = update(&mut app, Message::DragStart(1));
    let _ = update(&mut app, Message::DragOverGap(3));
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(names(&app.mods), ["a", "b", "Head_separator"]);
    assert!(
        app.collapsed.contains("Head"),
        "nothing is hidden, so nothing was unfolded"
    );
}

#[test]
fn mods_swallowed_by_a_folded_neighbour_are_named() {
    // Lift a header out from between a folded group and its own mods, and
    // those mods join the folded group: off screen, and with nothing else to
    // say so. The fold is the user's and is left alone; the disappearance is
    // not left to be discovered.
    let mut app = nav_app(&["Armour_separator", "a", "Weapons_separator", "w1", "w2"]);
    app.collapsed.insert("Armour".to_string());
    let _ = update(&mut app, Message::ModSendBottom(2));
    assert_eq!(
        names(&app.mods),
        ["Armour_separator", "a", "w1", "w2", "Weapons_separator"]
    );
    assert!(
        app.status
            .as_deref()
            .is_some_and(|s| s.contains("folded group")),
        "two mods went off screen unremarked: {:?}",
        app.status
    );

    // An ordinary move hides nothing, and says nothing.
    let mut app = nav_app(&["a", "b", "c"]);
    let _ = update(&mut app, Message::ModSendBottom(0));
    assert_eq!(app.status, None);
}

#[test]
fn alt_click_on_a_separator_selects_its_whole_group() {
    // MO2's gesture for taking a section rather than its label
    // (ModListView::mousePressEvent, modlistview.cpp:1444).
    let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
    app.modifiers = iced::keyboard::Modifiers::ALT;
    let _ = update(&mut app, Message::DragStart(0));
    assert_eq!(
        sel(&app),
        vec![0, 1, 2],
        "header plus its group, stopping at the next header"
    );

    let _ = update(&mut app, Message::DragStart(3));
    assert_eq!(
        sel(&app),
        vec![3, 4],
        "the last group runs to the end of the list"
    );

    // Alt on an ordinary row is not this gesture.
    let _ = update(&mut app, Message::DragStart(1));
    assert_eq!(sel(&app), Vec::<usize>::new());
}

#[test]
fn a_group_selected_with_alt_moves_as_one_block() {
    let mut app = nav_app(&["Head_separator", "a", "b", "Tail_separator", "c"]);
    app.modifiers = iced::keyboard::Modifiers::ALT;
    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragOverGap(5));
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(
        names(&app.mods),
        ["Tail_separator", "c", "Head_separator", "a", "b"]
    );
    assert_eq!(
        sel(&app),
        vec![2, 3, 4],
        "a block stays selected so it can be dragged again"
    );
}

#[test]
fn a_mixed_selection_no_longer_leaves_its_header_behind() {
    // `real_selection` filtered the separator out of the batch reorders, which
    // lifted a group's mods above their own header and stranded it.
    let mut app = nav_app(&["a", "Sec_separator", "b", "c"]);
    app.selected_mods = [1, 2].into_iter().collect();
    app.selected_mod = Some(1);
    let _ = update(&mut app, Message::BatchSendTop);
    assert_eq!(names(&app.mods), ["Sec_separator", "b", "a", "c"]);
}

#[test]
fn a_drag_re_anchors_the_selection_it_just_moved() {
    let mut app = nav_app(&["a", "b", "c", "d"]);
    app.selected_mods = [0, 1].into_iter().collect();
    app.selected_mod = Some(0);
    app.sel_anchor = Some(0);
    let _ = update(&mut app, Message::DragStart(0));
    let _ = update(&mut app, Message::DragOverGap(4));
    let _ = update(&mut app, Message::DragDrop);
    assert_eq!(names(&app.mods), ["c", "d", "a", "b"]);
    // Left at 0, the next Shift+click would have built its run from a row
    // nobody chose.
    assert_eq!(app.sel_anchor, Some(2));
    assert_eq!(sel(&app), vec![2, 3]);
}

#[test]
fn send_to_priority_keeps_the_menu_that_hosts_its_editor() {
    // Both "Send to..." items armed an inline editor and closed the card that
    // draws it, so they did nothing visible - for every row, not just
    // separators - and the armed state then hijacked the next right-click.
    let mut app = nav_app(&["a", "b"]);
    let _ = update(&mut app, Message::OpenModMenu(1));
    let _ = update(&mut app, Message::SendToPriorityStart(1));
    assert_eq!(
        app.menu_mod,
        Some(1),
        "the card holding the editor was dismissed"
    );
    assert!(app.send_priority.is_some());

    let _ = update(&mut app, Message::SendToPriorityChanged("0".to_string()));
    let _ = update(&mut app, Message::SendToPriorityCommit);
    assert_eq!(names(&app.mods), ["b", "a"]);
    assert_eq!(app.menu_mod, None, "the commit is what closes the menu");

    // And closing the menu disarms it, so the next right-click opens a menu.
    let _ = update(&mut app, Message::OpenModMenu(0));
    let _ = update(&mut app, Message::SendToSeparatorStart(0));
    let _ = update(&mut app, Message::CloseMenu);
    assert!(app.send_separator.is_none());
}

#[test]
fn opening_a_menu_freezes_where_it_was_summoned() {
    let mut app = nav_app(&["a", "b"]);
    let _ = update(&mut app, Message::PointerAt(iced::Point::new(300.0, 220.0)));
    let _ = update(&mut app, Message::OpenModMenu(1));
    assert_eq!(app.menu_at, Some(iced::Point::new(300.0, 220.0)));

    // The pointer keeps moving; a menu that followed it could not be aimed at.
    let _ = update(&mut app, Message::PointerAt(iced::Point::new(700.0, 600.0)));
    assert_eq!(app.menu_at, Some(iced::Point::new(300.0, 220.0)));

    // Closing releases it, so a stale point cannot place the next one.
    let _ = update(&mut app, Message::CloseMenu);
    assert_eq!(app.menu_at, None);
}

#[test]
fn a_criterion_cycles_off_only_except_and_back() {
    // Three settings, not two: "only conflicted" and "everything except
    // conflicted" are both questions people ask.
    let mut app = nav_app(&["a"]);
    assert_eq!(app.filters.active, Criterion::Off);
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    assert_eq!(app.filters.active, Criterion::Require);
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    assert_eq!(app.filters.active, Criterion::Exclude);
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    assert_eq!(app.filters.active, Criterion::Off, "back to not filtering");
}

#[test]
fn changing_a_filter_drops_the_selection_and_any_drag() {
    // The visible set changes underneath every row index they hold.
    let mut app = nav_app(&["a", "b", "c"]);
    app.selected_mods.extend([0, 2]);
    app.drag_state = Some(DragState {
        from: 0,
        gap: 2,
        aimed: true,
    });
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Conflicted));
    assert!(app.drag_state.is_none());
    // `Conflicted -> only` with no conflict map hides everything, so nothing
    // in the selection may survive it.
    assert!(app.selected_mods.is_empty());
}

/// Filtering does not renumber the rows, so an index kept across it still
/// RESOLVES - it just resolves to a mod that is no longer on screen. The
/// keyboard reads `selected_mod` raw, and `real_selection` falls back to it
/// when the multi-selection is empty, so a focus left on a hidden row aimed
/// Space, Delete and every batch action at a mod the user could not see.
#[test]
fn a_filter_never_leaves_the_keyboard_aimed_at_a_row_it_hid() {
    // Alpha stays visible (disabled); Ivy is enabled, so Active->Exclude hides it.
    let mut app = nav_app(&["Alpha", "Ivy"]);
    app.mods[0].enabled = false;
    app.mods[1].enabled = true;

    // A PLAIN CLICK - the only gesture a mouse user makes. It sets the focus
    // and the anchor and deliberately leaves `selected_mods` empty, which is
    // why clearing only that set was never enough.
    let _ = update_inner(&mut app, Message::SelectMod(1));
    assert_eq!(app.selected_mod, Some(1));
    assert!(
        app.selected_mods.is_empty(),
        "a plain click never populates the set"
    );

    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    assert_eq!(app.filters.active, Criterion::Exclude);
    assert_eq!(
        mod_row_visibility(&app, None),
        vec![true, false],
        "Ivy is off screen"
    );

    assert_eq!(app.selected_mod, None, "the focus went with the row");
    assert_eq!(app.sel_anchor, None, "and so did the shift-select anchor");
    // The proof that matters: the keys that ACT are now inert.
    let _ = update_inner(&mut app, Message::KeyNav(Nav::Toggle));
    assert!(
        app.mods[1].enabled,
        "Space must not toggle a mod that is not drawn"
    );
    let _ = update_inner(&mut app, Message::KeyNav(Nav::Remove));
    assert_ne!(
        app.confirm_remove,
        Some(1),
        "Delete must not arm on a hidden row"
    );
}

#[test]
fn a_filter_never_redirects_a_batch_action_onto_a_hidden_row() {
    let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
    app.mods[0].enabled = false;
    app.mods[1].enabled = false;
    app.mods[2].enabled = true;
    // Ctrl+click three rows: the set is full and the focus is on the last.
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    for i in 0..3 {
        let _ = update_inner(&mut app, Message::SelectMod(i));
    }
    app.modifiers = iced::keyboard::Modifiers::default();
    assert_eq!(app.selected_mods.len(), 3);

    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    // Ivy is hidden. `real_selection` falls back to `selected_mod` when the
    // set is empty, so a stale focus here would aim the batch Remove at the
    // one row nobody can see.
    assert!(
        !real_selection(&app).contains(&2),
        "the hidden row is not a batch target"
    );
}

#[test]
fn folding_a_group_takes_the_focus_with_it() {
    // Folding hides rows exactly like a filter does, through the same path.
    let mut app = nav_app(&["Gear_separator", "under"]);
    // The fold is keyed by DISPLAY name (the "_separator" suffix stripped),
    // which is what the header button sends.
    let key = app.mods[0].display_name().to_string();
    assert_eq!(key, "Gear");
    let _ = update_inner(&mut app, Message::SelectMod(1));
    assert_eq!(app.selected_mod, Some(1));
    let _ = update_inner(&mut app, Message::ToggleCollapse(key));
    assert_eq!(
        mod_row_visibility(&app, None),
        vec![true, false],
        "the group folded"
    );
    assert_eq!(app.selected_mod, None, "the focus did not survive the fold");
}

#[test]
fn select_all_only_takes_what_the_list_is_drawing() {
    let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
    app.mods[0].enabled = false;
    app.mods[1].enabled = false;
    app.mods[2].enabled = true;
    app.focus = Pane::Mods;
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));
    let _ = update_inner(&mut app, Message::CycleFilter(FilterField::Active));

    let _ = update_inner(&mut app, Message::SelectAllInFocus);
    // Not `0..mods.len()`: Ctrl+A used to sweep in every hidden row, and the
    // batch Remove then aimed remove_dir_all at all of them.
    assert_eq!(app.selected_mods.len(), 2);
    assert!(
        !app.selected_mods.contains(&2),
        "the hidden row is not selected"
    );
    assert_eq!(
        app.selected_mod,
        Some(0),
        "the focus lands on a row that is drawn"
    );
}

/// A download row dragged onto an insertion strip.
fn dl_row(name: &str) -> DownloadRow {
    DownloadRow {
        name: name.to_string(),
        path: PathBuf::from("/tmp").join(name),
        size: 1,
        version: String::new(),
        mod_name: None,
        mod_id: None,
        state: DownloadState::Ready,
        downloaded: 1,
        total: 1,
        speed: None,
        hidden: false,
        modified: std::time::SystemTime::UNIX_EPOCH,
    }
}

#[test]
fn dragging_a_download_aims_at_a_gap_and_a_plain_click_does_not() {
    let mut app = nav_app(&["a", "b", "c"]);
    app.downloads = vec![dl_row("Mod.7z")];

    // A press ARMS the drag - it does not commit anything, because the same
    // press is how the row is clicked.
    let _ = update_inner(&mut app, Message::DownloadDragStart(0));
    let d = app.download_drag.as_ref().expect("armed");
    assert!(!d.aimed, "a press alone is not an aim");
    assert_eq!(d.gap, 3, "unaimed, it would land at the end");

    // Releasing without ever crossing a strip is a plain click: no install.
    let _ = update_inner(&mut app, Message::PointerReleased);
    assert!(app.download_drag.is_none());
    assert_eq!(
        app.install_at, None,
        "a click on a download row installs nothing"
    );

    // Now with an aim.
    let _ = update_inner(&mut app, Message::DownloadDragStart(0));
    let _ = update_inner(&mut app, Message::DownloadDragOverGap(1));
    assert!(app.download_drag.as_ref().unwrap().aimed);
    let _ = update_inner(&mut app, Message::DownloadDragDrop);
    assert_eq!(
        app.install_at.as_ref().map(|(gap, _)| *gap),
        Some(1),
        "the drop remembers where it was aimed"
    );
    assert!(app.download_drag.is_none(), "and the drag is over");
}

#[test]
fn a_partial_download_cannot_be_dragged() {
    let mut app = nav_app(&["a"]);
    let mut row = dl_row("Half.7z");
    row.state = DownloadState::Downloading;
    app.downloads = vec![row];
    let _ = update_inner(&mut app, Message::DownloadDragStart(0));
    assert!(
        app.download_drag.is_none(),
        "there is nothing to install out of a partial"
    );
}

#[test]
fn a_gap_means_nothing_under_a_filter_so_the_drop_says_so() {
    let mut app = nav_app(&["a", "b", "c"]);
    app.downloads = vec![dl_row("Mod.7z")];
    app.search = "a".to_string();
    let _ = update_inner(&mut app, Message::DownloadDragStart(0));
    let _ = update_inner(&mut app, Message::DownloadDragOverGap(1));
    let _ = update_inner(&mut app, Message::DownloadDragDrop);
    // The strip between two VISIBLE rows can have any number of hidden rows
    // behind it, so "here" would be a lie. It installs at the end and says
    // so - through the installer, because `ModPicked` sets its own status a
    // moment later and would overwrite anything said here.
    assert_eq!(app.install_at, None);
    assert!(
        app.pending_note
            .as_deref()
            .unwrap_or("")
            .contains("end of the list"),
        "{:?}",
        app.pending_note
    );
}

#[test]
fn saving_morrowind_ini_in_the_window_does_not_block_its_next_plugin_edit() {
    // The INI editor writes the same Morrowind.ini the plugin state is hashed
    // from; the window's own save is not another process's sort.
    let root = temp_portable("morrowind");
    let inst = Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let mut app = app_for_game("morrowind");
    app.games[0].install_path = root.join("game");
    app.games[0].data_path = root.join("game/Data Files");
    let game = app.games[0].clone();
    fs::create_dir_all(&game.data_path).unwrap();
    for name in ["A.esp", "B.esp"] {
        fs::write(game.data_path.join(name), []).unwrap();
    }
    let ini = "[General]\r\n[Game Files]\r\nGameFile0=A.esp\r\nGameFile1=B.esp\r\n";
    fs::write(game.install_path.join("Morrowind.ini"), ini).unwrap();
    app.created = Some(inst.clone());
    app.tab = Tab::Plugins;
    let spec = game.plugin_spec().unwrap();
    app.plugins = compute_plugins(&app);
    let list = app.plugins.clone().unwrap();
    write_plugin_state(&app, &list, &spec).unwrap();

    let _ = update_inner(&mut app, Message::ShowIniEditor);
    {
        let ed = app.ini_editor.as_mut().expect("the editor opened");
        assert_eq!(ed.current, "Morrowind.ini");
        let edited = ed.content.text().replace("[General]", "[General]\r\nSubtitles=1");
        ed.content = iced::widget::text_editor::Content::with_text(&edited);
        ed.dirty = true;
    }
    let _ = update_inner(&mut app, Message::IniEditorSave);
    assert!(!app.ini_editor.as_ref().unwrap().dirty, "status={:?}", app.status);

    let mut next = app.plugins.clone().unwrap();
    assert!(next.set_enabled("B.esp", false));
    write_plugin_state(&app, &next, &spec).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_ini_editor_opens_the_real_morrowind_ini_and_never_reverts_an_outside_write() {
    // A fresh profile used to open an EMPTY buffer, and saving it founded
    // the profile's Morrowind.ini on a stub.
    let root = temp_portable("morrowind");
    let inst = Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let mut app = app_for_game("morrowind");
    app.games[0].install_path = root.join("game");
    app.games[0].data_path = root.join("game/Data Files");
    let game = app.games[0].clone();
    fs::create_dir_all(&game.data_path).unwrap();
    let ini = "[General]\r\n[Game Files]\r\nGameFile0=A.esp\r\n";
    fs::write(game.install_path.join("Morrowind.ini"), ini).unwrap();
    app.created = Some(inst.clone());

    let _ = update_inner(&mut app, Message::ShowIniEditor);
    let ed = app.ini_editor.as_mut().expect("the editor opened");
    assert!(!ed.missing, "status={:?}", app.status);
    assert!(ed.original.contains("GameFile0=A.esp"), "{}", ed.original);

    // `eidos sort` rewrites it while the editor is open.
    let path = inst.active().ini_path("Morrowind.ini");
    fs::write(&path, "[Game Files]\r\nGameFile0=B.esp\r\n").unwrap();
    ed.content = iced::widget::text_editor::Content::with_text("[General]\r\nSubtitles=1\r\n");
    ed.dirty = true;
    let _ = update_inner(&mut app, Message::IniEditorSave);
    assert!(app.ini_editor.as_ref().unwrap().dirty);
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Game Files]\r\nGameFile0=B.esp\r\n");
    fs::remove_dir_all(root).unwrap();
}

/// An instance with two profiles and a save in the active one.
fn saves_app() -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    // A second profile is a directory under profiles/; the copy target only
    // needs it to exist.
    fs::create_dir_all(inst.profile("Second").saves_dir()).unwrap();
    let dir = inst.active().saves_dir();
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Save1.ess"), b"x").unwrap();
    fs::write(dir.join("Save1.skse"), b"co").unwrap();
    fs::write(dir.join("Save2.ess"), b"y").unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    load_saves(&mut app);
    (app, root)
}

#[test]
fn the_profile_chips_are_read_once_and_follow_every_change() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    fs::create_dir_all(inst.profile("Second").dir()).unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;

    let (names, active) = cached_profiles(&app);
    assert!(names.contains(&"Default".to_string()) && names.contains(&"Second".to_string()));
    assert_eq!(active, "Default");
    // Cached: a change on disk alone must NOT be picked up, or the memo is
    // not doing anything.
    fs::create_dir_all(app.created.as_ref().unwrap().profile("Third").dir()).unwrap();
    assert_eq!(cached_profiles(&app).0.len(), 2, "still memoised");

    // But a profile message drops it, whichever branch the handler takes -
    // delete and rename do not bump the view generation.
    let _ = update_inner(&mut app, Message::ProfileDeleteCommit("Second".into()));
    let (names, _) = cached_profiles(&app);
    assert!(
        names.contains(&"Third".to_string()),
        "the memo fell: {names:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_mod_page_that_is_not_a_web_link_is_refused_before_it_is_stored() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    fs::create_dir_all(root.join("mods/M")).unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = vec![ModEntry {
        name: "M".into(),
        enabled: true,
        path: root.join("mods/M"),
        unmanaged: false,
    }];
    app.screen = Screen::Main;
    app.info_mod = Some(0);

    // Refused at the SAVE, not at the click that opens it: a value that
    // cannot be opened must not be storable, or the menu entry becomes a
    // dead end that looks live.
    app.url_edit = "file:///etc/passwd".to_string();
    let _ = update_inner(&mut app, Message::ModUrlSave);
    assert!(app.status.as_deref().unwrap_or("").contains("http"));
    assert_eq!(app.created.as_ref().unwrap().mod_meta("M").url(), None);

    app.url_edit = "https://github.com/me/mod".to_string();
    let _ = update_inner(&mut app, Message::ModUrlSave);
    assert_eq!(
        app.created.as_ref().unwrap().mod_meta("M").url().as_deref(),
        Some("https://github.com/me/mod")
    );
    let _ = fs::remove_dir_all(&root);
}

/// A collection state holding a revision built from the captured payload.
/// `mods` is (folder, mod id, extra `meta.ini` lines), `downloads` is the
/// file ids to leave a WHOLE archive for.
fn collection_app_with(
    mods: &[(&str, u64, &str)],
    downloads: &[u64],
    partial: &[u64],
) -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for (name, mod_id, extra) in mods {
        let dir = root.join("mods").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("meta.ini"),
            format!("[General]\nmodid={mod_id}\n{extra}"),
        )
        .unwrap();
    }
    let dl = inst.downloads_dir();
    fs::create_dir_all(&dl).unwrap();
    for file_id in downloads.iter().chain(partial) {
        fs::write(
            dl.join(format!("a{file_id}.7z.meta")),
            format!(
                "[General]\ngameName=SkyrimSE\nmodID={}\nfileID={file_id}\n",
                captured_revision()
                    .mods
                    .iter()
                    .find(|m| m.file_id == *file_id)
                    .map(|m| m.mod_id)
                    .unwrap_or(1)
            ),
        )
        .unwrap();
    }
    // Whole: the archive is there and nothing is still arriving beside it.
    for file_id in downloads {
        fs::write(dl.join(format!("a{file_id}.7z")), b"x").unwrap();
    }
    // Interrupted: `eidos nxm` wrote the sidecar before the first byte and
    // left it behind when the transfer died.
    for file_id in partial {
        fs::write(dl.join(format!("a{file_id}.7z.unfinished")), b"x").unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.mods = mods
        .iter()
        .map(|(n, _, _)| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: root.join("mods").join(n),
            unmanaged: false,
        })
        .collect();
    app.created = Some(inst);
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);
    (app, root)
}

fn collection_app(mods: &[(&str, u64)], downloads: &[u64]) -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for (name, mod_id) in mods {
        let dir = root.join("mods").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("meta.ini"), format!("[General]\nmodid={mod_id}\n")).unwrap();
    }
    let dl = inst.downloads_dir();
    fs::create_dir_all(&dl).unwrap();
    for file_id in downloads {
        fs::write(dl.join(format!("a{file_id}.7z")), b"x").unwrap();
        fs::write(
            dl.join(format!("a{file_id}.7z.meta")),
            format!(
                "[General]\ngameName=SkyrimSE\nmodID={}\nfileID={file_id}\n",
                captured_revision()
                    .mods
                    .iter()
                    .find(|m| m.file_id == *file_id)
                    .map(|m| m.mod_id)
                    .unwrap_or(1)
            ),
        )
        .unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.mods = mods
        .iter()
        .map(|(n, _)| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: root.join("mods").join(n),
            unmanaged: false,
        })
        .collect();
    app.created = Some(inst);
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);
    (app, root)
}

/// A revision with three members. Built by hand rather than parsed: the
/// PARSE is already tested against a captured real payload in eidos-nexus,
/// and what these tests exercise is the local join, which only needs ids.
fn captured_revision() -> eidos_nexus::collections::CollectionRevision {
    use eidos_nexus::collections::{CollectionMod, CollectionRevision};
    let member = |name: &str, mod_id: u64, file_id: u64| CollectionMod {
        name: name.to_string(),
        mod_id,
        file_id,
        domain: "skyrimspecialedition".to_string(),
        version: "2.1".to_string(),
        file_title: format!("{name}.7z"),
        size_in_bytes: 1024,
        optional: false,
    };
    CollectionRevision {
        slug: "rqhcxy".to_string(),
        revision_number: 1,
        name: "The Great Cities Collection".to_string(),
        summary: String::new(),
        author: "HookerHeels".to_string(),
        game_domain: "skyrimspecialedition".to_string(),
        download_link: "/v2/collections/335/revisions/467/download_link".to_string(),
        mod_count: 3,
        total_size: 3072,
        instructions: String::new(),
        mods: vec![
            member("Karthwasten", 37471, 232153),
            member("Mixwater Mill", 37414, 232146),
            member("Shors Stone", 36462, 234034),
        ],
        hidden: None,
    }
}

#[test]
fn a_collection_member_is_matched_against_what_the_instance_already_has() {
    let rev = captured_revision();
    // First member installed by mod id, second downloaded by file id, the
    // rest missing.
    let (first, second) = (rev.mods[0].mod_id, rev.mods[1].file_id);
    let (mut app, root) = collection_app(&[("Karthwasten", first)], &[second]);
    app.collection = Some(CollectionState {
        install_check: None,
        runtime: None,
        link: String::new(),
        revision: Some(rev),
        states: Vec::new(),
        loading: false,
        error: None,
        confirm_fetch: false,
        asked: std::collections::HashSet::new(),
    });
    recompute_collection_states(&mut app);

    let st = &app.collection.as_ref().unwrap().states;
    assert_eq!(
        st[0],
        MemberState::Unverified,
        "a mod id does not identify the requested file"
    );
    assert_eq!(
        st[1],
        MemberState::Downloaded,
        "matched on the exact file id"
    );
    assert!(st[2..].iter().all(|s| *s == MemberState::Missing));
    let _ = fs::remove_dir_all(&root);
}

/// The three things the join used to get wrong, each of which told the user
/// something specific and false.
#[test]
fn the_join_does_not_claim_more_than_it_knows() {
    let rev = captured_revision();
    let (first, second) = (rev.mods[0].mod_id, rev.mods[1].file_id);
    let want = rev.mods[0].version.clone();
    assert!(!want.is_empty(), "the fixture must pin a version");

    // (1) A mod id from ANOTHER game's page is not this member. Nexus ids
    //     are per game, so a bare id is ambiguous across them.
    let (mut app, root) =
        collection_app_with(&[("Elsewhere", first, "gameName=Skyrim\n")], &[], &[second]);
    let state = |app: &mut App| {
        app.collection = Some(CollectionState {
            install_check: None,
            runtime: None,
            link: String::new(),
            revision: Some(captured_revision()),
            states: Vec::new(),
            loading: false,
            error: None,
            confirm_fetch: false,
            asked: std::collections::HashSet::new(),
        });
        recompute_collection_states(app);
        app.collection.as_ref().unwrap().states.clone()
    };
    let st = state(&mut app);
    assert_eq!(st[0], MemberState::Missing, "another game's mod id");
    // (2) A sidecar beside an UNFINISHED transfer is not a download. It is
    //     written before the first byte and survives a failure.
    assert_eq!(st[1], MemberState::Missing, "the archive never arrived");
    let _ = fs::remove_dir_all(&root);

    // (3) An outdated copy is not the version the collection asks for.
    let (mut app, root) = collection_app_with(
        &[("Outdated", first, "gameName=SkyrimSE\nversion=0.0.1-old\n")],
        &[],
        &[],
    );
    let st = state(&mut app);
    assert_eq!(st[0], MemberState::OtherVersion);
    let _ = fs::remove_dir_all(&root);

    // A matching version alone cannot verify the requested file.
    let (mut app, root) = collection_app_with(
        &[(
            "Right",
            first,
            &format!("gameName=SkyrimSE\nversion={want}\n"),
        )],
        &[],
        &[],
    );
    let st = state(&mut app);
    assert_eq!(st[0], MemberState::Unverified);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn async_results_do_not_mutate_another_instance_or_collection_request() {
    let (mut app, root) = list_app(&["Personal"]);
    let inst = app.created.clone().unwrap();
    let path = inst.meta_path("Personal");
    fs::write(&path, "[General]\nmodid=42\nendorsed=0\n").unwrap();
    let _ = update_inner(
        &mut app,
        Message::ModEndorsed(root.join("other-instance"), "Personal".into(), 42, Ok(true)),
    );
    let _ = update_inner(
        &mut app,
        Message::ModEndorsed(root.clone(), "Personal".into(), 99, Ok(true)),
    );
    assert!(!inst.mod_meta("Personal").endorsed());
    let _ = update_inner(
        &mut app,
        Message::CollectionFetched(
            root.join("other-instance"),
            "old-request".into(),
            Ok(captured_revision()),
        ),
    );
    assert!(app.collection.is_none());
    let _ = update_inner(&mut app, Message::ShowCollection(String::new()));
    app.collection.as_mut().unwrap().loading = true;
    let _ = update_inner(
        &mut app,
        Message::CollectionLinkChanged("current-request".into()),
    );
    let _ = update_inner(
        &mut app,
        Message::CollectionFetched(root.clone(), "old-request".into(), Ok(captured_revision())),
    );
    assert!(app.collection.as_ref().unwrap().revision.is_none());
    assert!(!app.collection.as_ref().unwrap().loading);
    let names = BTreeSet::new();
    let snapshot = SortFingerprint {
        game: "skyrimse".into(),
        profile: "Default".into(),
        names,
        instance: Some(root.clone()),
        epoch: app.archive_epoch.get(),
    };
    let _ = update_inner(
        &mut app,
        Message::PluginsSorted(Ok((
            SortFingerprint {
                instance: Some(root.join("other-instance")),
                ..snapshot.clone()
            },
            vec![],
            Err("unused report".into()),
        ))),
    );
    assert!(app
        .status
        .as_deref()
        .unwrap_or_default()
        .contains("Discarded"));
    plugin_state_changed(&app);
    let _ = update_inner(
        &mut app,
        Message::PluginsSorted(Ok((snapshot, vec![], Err("unused report".into())))),
    );
    assert!(app
        .status
        .as_deref()
        .unwrap_or_default()
        .contains("Discarded"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_actions_preserve_unreadable_content_and_invalidate_ini_views() {
    let (mut app, root) = list_app(&["Personal"]);
    let inst = app.created.clone().unwrap();
    let path = inst.meta_path("Personal");
    fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();
    let _ = update_inner(&mut app, Message::ModTrack(0));
    assert_eq!(fs::read(&path).unwrap(), [0xff, 0xfe, 0xfd]);
    assert!(app
        .status
        .as_deref()
        .unwrap_or_default()
        .contains("Could not save"));
    fs::write(&path, "[General]\nmodid=42\n").unwrap();
    let epoch = app.archive_epoch.get();
    let _ = update_inner(&mut app, Message::ToggleIniTweak(0, "Skyrim.ini".into()));
    assert_eq!(inst.mod_meta("Personal").ini_tweaks(), &["Skyrim.ini"]);
    assert!(app.archive_epoch.get() > epoch);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mod_information_shows_generated_receipts_and_input_drift() {
    let (mut app, root) = list_app(&["Input", "Output"]);
    let inst = app.created.clone().unwrap();
    inst.save_modlist(&app.mods).unwrap();
    let run = inst
        .begin_tool_run("Test Generator", None, &["generator".into()], &[])
        .unwrap();
    fs::write(inst.overwrite_dir().join("generated.txt"), b"output").unwrap();
    inst.finish_tool_run(run).unwrap();
    inst.overwrite_into_mod("Output").unwrap();
    let _ = update(&mut app, Message::ShowModInfo(1));
    assert!(app.info_provenance.contains("Test Generator"));
    assert!(
        app.info_provenance.contains("Recorded inputs unchanged"),
        "{}",
        app.info_provenance
    );
    let mut meta = inst.mod_meta("Input");
    meta.set("version", "changed");
    meta.write(&inst.meta_path("Input")).unwrap();
    let _ = update(&mut app, Message::ShowModInfo(1));
    assert!(app.info_provenance.contains("Inputs changed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_matching_uses_every_exact_merged_source_and_rejects_same_version_files() {
    let mut rev = captured_revision();
    rev.mods[1].mod_id = rev.mods[0].mod_id;
    let id = rev.mods[0].mod_id;
    let file = rev.mods[0].file_id;
    let extra = format!("gameName=SkyrimSE\nversion=2.1\n[installedFiles]\n1\\modid={id}\n1\\fileid={file}\n2\\modid=999\n2\\fileid=123\nsize=2\n");
    let (mut app, root) = collection_app_with(&[("Merged", id, &extra)], &[], &[]);
    app.collection = Some(CollectionState {
        install_check: None,
        runtime: None,
        revision: Some(rev),
        link: String::new(),
        states: Vec::new(),
        loading: false,
        error: None,
        confirm_fetch: false,
        asked: HashSet::new(),
    });
    recompute_collection_states(&mut app);
    assert_eq!(
        app.collection.as_ref().unwrap().states[..2],
        [MemberState::Installed, MemberState::OtherVersion]
    );
    app.collection
        .as_mut()
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .mods[0]
        .mod_id = 999;
    app.collection
        .as_mut()
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .mods[0]
        .file_id = 123;
    recompute_collection_states(&mut app);
    assert_eq!(
        app.collection.as_ref().unwrap().states[0],
        MemberState::Installed
    );
    app.meta_cache.get_mut("Merged").unwrap().install_warning = Some("Missing source".into());
    recompute_collection_states(&mut app);
    assert_eq!(
        app.collection.as_ref().unwrap().states[0],
        MemberState::Unverified
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn one_click_asks_for_a_capped_batch_not_the_whole_collection() {
    use crate::update::{next_fetch_batch, FETCH_BATCH};
    let mut rev = captured_revision();
    // Twenty missing members - a modest collection by Nexus standards, and
    // twenty `eidos nxm` children would be twenty processes at once.
    let one = rev.mods[0].clone();
    rev.mods = (0..20u64)
        .map(|i| eidos_nexus::collections::CollectionMod {
            mod_id: 1000 + i,
            file_id: 2000 + i,
            ..one.clone()
        })
        .collect();
    let states = vec![MemberState::Missing; rev.mods.len()];
    let mut asked = std::collections::HashSet::new();

    let (batch, left) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
    assert_eq!(batch.len(), FETCH_BATCH, "capped");
    assert_eq!(
        left,
        20 - FETCH_BATCH,
        "and it says how many are behind them"
    );

    // Clicking again advances instead of restarting the same few: the first
    // batch is still `Missing` (its downloads are running), so only `asked`
    // can tell them apart.
    asked.extend(batch.iter().map(|(_, f, _)| *f));
    let (second, _) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
    assert_eq!(second.len(), FETCH_BATCH);
    assert!(
        second
            .iter()
            .all(|(_, f, _)| !batch.iter().any(|(_, g, _)| g == f)),
        "no overlap with the first batch"
    );

    // And the tail is short rather than wrapping.
    asked.extend(rev.mods.iter().map(|m| m.file_id).take(18));
    let (tail, left) = next_fetch_batch(&rev, &states, &asked, FETCH_BATCH);
    assert_eq!(tail.len(), 2);
    assert_eq!(left, 0);
}

#[test]
fn fetching_the_missing_members_takes_two_clicks() {
    let (mut app, root) = collection_app(&[], &[]);
    let rev = captured_revision();
    app.collection = Some(CollectionState {
        install_check: None,
        runtime: None,
        link: String::new(),
        revision: Some(rev),
        states: Vec::new(),
        loading: false,
        error: None,
        confirm_fetch: false,
        asked: std::collections::HashSet::new(),
    });
    recompute_collection_states(&mut app);
    assert!(
        app.collection
            .as_ref()
            .unwrap()
            .states
            .iter()
            .all(|s| *s == MemberState::Missing),
        "the fixture instance has none of them"
    );

    // First click arms and spawns nothing.
    let _ = update_inner(&mut app, Message::CollectionFetchMissing);
    assert!(app.collection.as_ref().unwrap().confirm_fetch);
    assert!(
        app.collection.as_ref().unwrap().asked.is_empty(),
        "nothing started yet"
    );

    // And any other action disarms it, so a stray click cannot start a
    // dozen transfers a minute later.
    let _ = update_inner(&mut app, Message::CollectionLinkChanged("x".to_string()));
    assert!(!app.collection.as_ref().unwrap().confirm_fetch);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_member_already_asked_for_is_not_asked_for_twice() {
    let (mut app, root) = collection_app(&[], &[]);
    let rev = captured_revision();
    let all: Vec<u64> = rev.mods.iter().map(|m| m.file_id).collect();
    app.collection = Some(CollectionState {
        install_check: None,
        runtime: None,
        link: String::new(),
        revision: Some(rev),
        states: Vec::new(),
        loading: false,
        error: None,
        confirm_fetch: false,
        // Every member already started. A download in flight leaves its
        // member `missing` until the sidecar lands, so without this set the
        // pane would spawn the same transfers again on the next click.
        asked: all.into_iter().collect(),
    });
    recompute_collection_states(&mut app);

    let _ = update_inner(&mut app, Message::CollectionFetchMissing);
    assert!(
        !app.collection.as_ref().unwrap().confirm_fetch,
        "nothing to confirm"
    );
    let s = app.status.clone().unwrap_or_default();
    assert!(s.contains("already started"), "{s}");
    assert!(s.contains("Look up again"), "and how to retry: {s}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn preview_results_cannot_replace_a_newer_selection_or_changed_file() {
    let (mut app, root) = data_app(&[("old.txt", "old"), ("new.txt", "new")], &[]);
    let old = app.mods[0].path.join("old.txt");
    let new = app.mods[0].path.join("new.txt");
    let _ = update_inner(&mut app, Message::PreviewFile(old.clone()));
    let first = app.preview_pending.as_ref().unwrap().id;
    let _ = update_inner(&mut app, Message::PreviewFile(new.clone()));
    let second = app.preview_pending.as_ref().unwrap().id;
    let _ = update_inner(&mut app, Message::PreviewReady(first, build_preview(&old)));
    assert_eq!(app.preview_pending.as_ref().unwrap().id, second);
    assert_eq!(app.preview.as_ref().unwrap().path(), new);
    let decoded = build_preview(&new);
    fs::write(&new, "changed after decoding").unwrap();
    let _ = update_inner(&mut app, Message::PreviewReady(second, decoded));
    assert!(app.preview.is_none());
    assert!(app.status.as_deref().unwrap().contains("changed"));
    let _ = update_inner(&mut app, Message::PreviewFile(old.clone()));
    let third = app.preview_pending.as_ref().unwrap().id;
    let _ = update_inner(&mut app, Message::ClosePreview);
    let _ = update_inner(&mut app, Message::PreviewReady(third, build_preview(&old)));
    assert!(app.preview.is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cached_dds_selection_rejects_replaced_loose_files_and_archives() {
    let (mut app, root) = data_app(&[], &[]);
    let mut bytes = vec![0; 152];
    bytes[..4].copy_from_slice(b"DDS ");
    for (at, value) in [
        (4, 124u32),
        (8, 0x21007),
        (12, 1),
        (16, 1),
        (28, 1),
        (76, 32),
        (80, 4),
        (84, u32::from_le_bytes(*b"DX10")),
        (108, 0x401008),
        (128, 28),
        (132, 3),
        (140, 1),
    ] {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[148..].copy_from_slice(&[255, 0, 0, 255]);
    for (archived, change_view) in [(false, false), (true, false), (false, true), (true, true)]
    {
        let path = root.join(if archived {
            "textures.bsa"
        } else {
            "texture.dds"
        });
        fs::write(&path, &bytes).unwrap();
        let content = file_preview::from_bytes(
            if archived {
                Path::new("texture.dds")
            } else {
                &path
            },
            bytes.clone(),
            false,
        );
        assert!(matches!(content, Preview::Dds { .. }));
        let decoded = if archived {
            Preview::Archive {
                source: archive_conflicts::MemberSource {
                    path: path.clone(),
                    member: "texture.dds".into(),
                    identity: eidos_conflicts::archive_identity(&path).unwrap(),
                },
                content: Box::new(content),
            }
        } else {
            content
        };
        let _ = update_inner(&mut app, Message::PreviewFile(path.clone()));
        let id = app.preview_pending.as_ref().unwrap().id;
        file_preview::complete(&mut app, id, decoded);
        let cached = app.preview.clone().unwrap();
        // The cached pixels belong to the old source, even if a new control
        // request starts after its replacement has already finished.
        if change_view {
            bump_views(&app);
        } else {
            fs::write(&path, b"replacement source").unwrap();
        }
        let _ = update_inner(
            &mut app,
            Message::PreviewDdsSelection(dds_preview::Selection {
                channel: dds_preview::Channel::Alpha,
                ..Default::default()
            }),
        );
        let id = app.preview_pending.as_ref().unwrap().id;
        file_preview::complete(&mut app, id, cached);
        assert!(
            app.preview.is_none(),
            "accepted stale DDS pixels; archived={archived}"
        );
        assert!(app.status.as_deref().unwrap().contains("changed"));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_divider_resizes_the_panes_and_refuses_to_collapse_one() {
    let mut app = app_for_game("skyrimse");
    app.window = iced::Size::new(1000.0, 800.0);
    let before = app.split;

    // Moving the pointer without grabbing must not move anything: the
    // divider is a handle, not a hover target.
    let _ = update_inner(&mut app, Message::PointerAt(iced::Point::new(300.0, 400.0)));
    assert_eq!(app.split, before, "the pointer alone must not resize");

    let _ = update_inner(&mut app, Message::SplitGrab);
    let _ = update_inner(&mut app, Message::PointerAt(iced::Point::new(300.0, 400.0)));
    assert!((app.split - 0.3).abs() < 0.001, "split={}", app.split);

    // Dragged past the edge, both panes must survive: a pane at zero width
    // takes the divider with it and there is no way to get either back.
    let _ = update_inner(
        &mut app,
        Message::PointerAt(iced::Point::new(-500.0, 400.0)),
    );
    assert_eq!(app.split, 0.15);
    let _ = update_inner(
        &mut app,
        Message::PointerAt(iced::Point::new(5000.0, 400.0)),
    );
    assert_eq!(app.split, 0.85);

    // Releasing stops the drag and hands the value to the preferences, which
    // is what makes it survive a restart.
    let _ = update_inner(&mut app, Message::PointerReleased);
    assert!(!app.split_drag);
    assert_eq!(app.prefs.split, 0.85);

    // And a pointer move after release is inert again.
    let _ = update_inner(&mut app, Message::PointerAt(iced::Point::new(100.0, 400.0)));
    assert_eq!(app.split, 0.85);
}

#[test]
fn a_backup_is_inert_and_can_be_restored_over_the_mod_it_came_from() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let modd = root.join("mods").join("Armour");
    fs::create_dir_all(modd.join("Meshes")).unwrap();
    fs::write(modd.join("Meshes").join("a.nif"), b"original").unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    reload_mods(&mut app);

    let i = app.mods.iter().position(|m| m.name == "Armour").unwrap();
    let _ = update_inner(&mut app, Message::ModBackup(i));
    assert!(root
        .join("mods")
        .join("Armour_backup")
        .join("Meshes")
        .join("a.nif")
        .is_file());

    // Inert: it contributes nothing to the game, whatever modlist.txt says.
    let backup = app
        .mods
        .iter()
        .find(|m| m.name == "Armour_backup")
        .expect("in the list");
    assert!(backup.is_backup());
    assert!(!backup.is_active(), "a backup must never reach the game");

    // A second backup does not replace the first - that would lose the very
    // state somebody took a backup to keep.
    let i = app.mods.iter().position(|m| m.name == "Armour").unwrap();
    let _ = update_inner(&mut app, Message::ModBackup(i));
    assert!(
        root.join("mods").join("Armour_backup2").is_dir(),
        "status={:?}",
        app.status
    );

    // Now break the mod, and restore.
    fs::write(modd.join("Meshes").join("a.nif"), b"broken").unwrap();
    let b = app
        .mods
        .iter()
        .position(|m| m.name == "Armour_backup")
        .unwrap();
    let _ = update_inner(&mut app, Message::ModRestoreBackup(b));
    assert_eq!(
        fs::read(modd.join("Meshes").join("a.nif")).unwrap(),
        b"broken",
        "one click arms"
    );
    let _ = update_inner(
        &mut app,
        Message::ConfirmModRestoreBackup("Armour_backup".to_string()),
    );
    assert_eq!(
        fs::read(modd.join("Meshes").join("a.nif")).unwrap(),
        b"original",
        "status={:?} error={:?}",
        app.status,
        app.error
    );
    // And the scratch directory the restore used is gone.
    assert!(!root.join("mods").join("Armour.eidos-restoring").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn renaming_a_file_replaces_the_name_and_never_overwrites() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let modd = root.join("mods").join("Armour");
    fs::create_dir_all(modd.join("Meshes")).unwrap();
    fs::write(modd.join("Meshes").join("a.nif"), b"a").unwrap();
    fs::write(modd.join("Meshes").join("taken.nif"), b"b").unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = vec![ModEntry {
        name: "Armour".to_string(),
        enabled: true,
        path: modd.clone(),
        unmanaged: false,
    }];
    app.screen = Screen::Main;

    let _ = update_inner(
        &mut app,
        Message::FiletreeRenameStart(0, "Meshes/a.nif".to_string()),
    );
    // The box holds the NAME, not the path - editing directories in a rename
    // box is a move, which this is not.
    assert_eq!(app.tree_rename_text, "a.nif");

    let _ = update_inner(
        &mut app,
        Message::FiletreeRenameChanged("b.nif".to_string()),
    );
    let _ = update_inner(&mut app, Message::FiletreeRenameCommit);
    assert!(
        modd.join("Meshes").join("b.nif").is_file(),
        "status={:?} error={:?}",
        app.status,
        app.error
    );
    assert!(!modd.join("Meshes").join("a.nif").exists());

    // Never over something already there: fs::rename would replace it in
    // silence, and this is a mod's own contents.
    let _ = update_inner(
        &mut app,
        Message::FiletreeRenameStart(0, "Meshes/b.nif".to_string()),
    );
    let _ = update_inner(
        &mut app,
        Message::FiletreeRenameChanged("taken.nif".to_string()),
    );
    let _ = update_inner(&mut app, Message::FiletreeRenameCommit);
    assert!(
        modd.join("Meshes").join("b.nif").is_file(),
        "the rename was refused"
    );
    assert_eq!(
        fs::read(modd.join("Meshes").join("taken.nif")).unwrap(),
        b"b"
    );
    assert!(app.error.is_some());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn deleting_from_the_filetree_takes_two_clicks() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let modd = root.join("mods").join("Armour");
    fs::create_dir_all(modd.join("Meshes")).unwrap();
    fs::write(modd.join("Meshes").join("a.nif"), b"a").unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = vec![ModEntry {
        name: "Armour".to_string(),
        enabled: true,
        path: modd.clone(),
        unmanaged: false,
    }];
    app.screen = Screen::Main;

    let _ = update_inner(
        &mut app,
        Message::FiletreeDelete(0, "Meshes/a.nif".to_string()),
    );
    assert!(
        modd.join("Meshes").join("a.nif").is_file(),
        "one click only arms"
    );
    assert!(app.tree_delete_armed.is_some());

    // And any other action disarms it, like every other confirmation here.
    let _ = update_inner(&mut app, Message::Refresh);
    assert!(app.tree_delete_armed.is_none());
    assert!(modd.join("Meshes").join("a.nif").is_file());

    let _ = update_inner(
        &mut app,
        Message::FiletreeDelete(0, "Meshes/a.nif".to_string()),
    );
    let _ = update_inner(
        &mut app,
        Message::ConfirmFiletreeDelete("Armour".to_string(), "Meshes/a.nif".to_string()),
    );
    assert!(!modd.join("Meshes").join("a.nif").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_edit_in_a_stale_window_keeps_what_another_process_wrote() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for n in ["Aaa", "Zzz"] {
        fs::create_dir_all(root.join("mods").join(n)).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst.clone());
    app.screen = Screen::Main;
    reload_mods(&mut app);
    // `eidos install` in a terminal while the window is open.
    fs::create_dir_all(root.join("mods").join("SkyUI")).unwrap();
    inst.register_installed_mod("SkyUI").unwrap();
    // A tick in the window, which still shows the list from before.
    let at = |app: &App| app.mods.iter().position(|m| m.name == "Aaa").unwrap();
    let i = at(&app);
    app.mods[i].enabled = !app.mods[i].enabled;
    mods_changed(&mut app);
    let skyui = inst.modlist().into_iter().find(|m| m.name == "SkyUI");
    assert!(skyui.is_some_and(|m| m.enabled), "status={:?}", app.status);
    assert!(app.mods.iter().any(|m| m.name == "SkyUI"), "the window reloaded");
    // Reloaded, the window's edits land again - twice, so its own save is
    // not mistaken for another process's.
    for on in [true, false] {
        let i = at(&app);
        app.mods[i].enabled = on;
        mods_changed(&mut app);
        let aaa = inst.modlist().into_iter().find(|m| m.name == "Aaa").unwrap();
        assert_eq!(aaa.enabled, on, "status={:?}", app.status);
    }
    // A removal edits modlist.txt itself before saving; that is the
    // window's own write, not another process's.
    let zzz = app.mods.iter().position(|m| m.name == "Zzz").unwrap();
    let _ = update_inner(&mut app, Message::ModRemove(zzz));
    let _ = update_inner(&mut app, Message::ModRemove(zzz));
    assert_eq!(app.status.as_deref(), Some("Removed 'Zzz'."));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_rename_in_a_stale_window_is_refused_before_the_folder_moves() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for n in ["Aaa", "SkyUI"] {
        fs::create_dir_all(root.join("mods").join(n)).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst.clone());
    app.screen = Screen::Main;
    reload_mods(&mut app);
    for m in app.mods.iter_mut() {
        m.enabled = true;
    }
    mods_changed(&mut app);
    // `eidos install` in a terminal while the window is open.
    fs::create_dir_all(root.join("mods").join("Foo")).unwrap();
    inst.register_installed_mod("Foo").unwrap();

    let i = app.mods.iter().position(|m| m.name == "SkyUI").unwrap();
    let _ = update_inner(&mut app, Message::RenameStart(i));
    let _ = update_inner(&mut app, Message::RenameChanged("SkyUI 5.2".to_string()));
    let _ = update_inner(&mut app, Message::RenameCommit);

    // Refused while the folder still had its old name, and said so: had it
    // moved first, the reload would have listed "SkyUI 5.2" DISABLED.
    assert!(root.join("mods").join("SkyUI").is_dir(), "status={:?}", app.status);
    assert!(!root.join("mods").join("SkyUI 5.2").exists());
    assert!(app.status.as_deref().unwrap_or("").contains("another Eidos process"));
    let listed = inst.modlist();
    assert!(listed.iter().any(|m| m.name == "SkyUI" && m.enabled));
    assert!(listed.iter().any(|m| m.name == "Foo" && m.enabled));
    assert!(app.mods.iter().any(|m| m.name == "Foo"), "the window reloaded");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn batch_edits_in_a_stale_window_report_the_refusal_not_success() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for n in ["Aaa", "Bbb"] {
        fs::create_dir_all(root.join("mods").join(n)).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst.clone());
    app.screen = Screen::Main;
    reload_mods(&mut app);
    let stale = |app: &mut App, name: &str| {
        fs::create_dir_all(root.join("mods").join(name)).unwrap();
        inst.register_installed_mod(name).unwrap();
        app.selected_mods = (0..app.mods.len()).collect();
    };

    // "Enable selected" used to replace the refusal with "Enabled 2 mod(s)."
    stale(&mut app, "Foo");
    let _ = update_inner(&mut app, Message::BatchToggleMods);
    assert!(app.status.as_deref().unwrap_or("").contains("another Eidos process"));

    // And a batch Remove refuses BEFORE deleting, not after.
    stale(&mut app, "Bar");
    let _ = update_inner(&mut app, Message::ConfirmBatchRemove);
    assert!(app.status.as_deref().unwrap_or("").contains("another Eidos process"));
    assert!(root.join("mods").join("Aaa").is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn renaming_a_mod_leaves_it_where_it_was_and_still_enabled() {
    // The defect, exactly as it was reported: rename a mod and it is
    // "teleported all the way to the top, unticked". The rename itself was
    // fine - what failed was `save_mods` immediately after it, refused by
    // the instance lock the rename handler was still holding. modlist.txt
    // then still named the OLD folder, the reload treated the renamed one as
    // a mod nobody had seen, and a new mod goes to the top disabled.
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for n in ["Aaa", "Middle", "Zzz"] {
        fs::create_dir_all(root.join("mods").join(n).join("Meshes")).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    reload_mods(&mut app);
    // Everything on, and remember where the middle one sits.
    for m in app.mods.iter_mut() {
        m.enabled = true;
    }
    mods_changed(&mut app);
    let before = app.mods.iter().position(|m| m.name == "Middle").unwrap();
    let total = app.mods.len();
    // A second profile sharing the pool must follow the rename too.
    let inst = app.created.clone().unwrap();
    inst.profile("Other").create_from(&inst.active()).unwrap();

    let _ = update_inner(&mut app, Message::RenameStart(before));
    let _ = update_inner(&mut app, Message::RenameChanged("Renamed".to_string()));
    let _ = update_inner(&mut app, Message::RenameCommit);

    assert!(
        root.join("mods").join("Renamed").is_dir(),
        "the folder moved: status={:?}",
        app.status
    );
    assert_eq!(app.mods.len(), total, "no row appeared or vanished");
    let after = app
        .mods
        .iter()
        .position(|m| m.name == "Renamed")
        .expect("still listed");
    assert_eq!(after, before, "same position - status={:?}", app.status);
    assert!(
        app.mods[after].enabled,
        "still enabled - status={:?}",
        app.status
    );

    // And it survives a reload, which is what proves modlist.txt was written
    // rather than merely the in-memory list being right.
    reload_mods(&mut app);
    let reloaded = app
        .mods
        .iter()
        .position(|m| m.name == "Renamed")
        .expect("in modlist.txt");
    assert_eq!(reloaded, before);
    assert!(app.mods[reloaded].enabled);
    let (other, trust) = inst.profile("Other").modlist_checked();
    assert!(trust.is_good(), "{trust:?}");
    let at = other.iter().position(|m| m.name == "Renamed").expect("listed in Other");
    assert_eq!(at, before);
    assert!(other[at].enabled);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_confirmation_survives_the_list_moving_under_it() {
    // Two-click confirmations that store an INDEX are a known trap here: a
    // reload between the clicks and the second one acts on a different row.
    // A restore would then overwrite a different mod - and the `is_backup`
    // guard would not catch it, because the other row is a backup too.
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for (name, body) in [("A", "a"), ("B", "b")] {
        let d = root.join("mods").join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("f.txt"), body).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    reload_mods(&mut app);

    // Back both up, so there are two backups to confuse.
    for name in ["A", "B"] {
        let i = app.mods.iter().position(|m| m.name == name).unwrap();
        let _ = update_inner(&mut app, Message::ModBackup(i));
    }
    // Break A, then arm ITS restore.
    fs::write(root.join("mods").join("A").join("f.txt"), b"broken").unwrap();
    let i = app.mods.iter().position(|m| m.name == "A_backup").unwrap();
    let _ = update_inner(&mut app, Message::ModRestoreBackup(i));
    assert_eq!(
        app.confirm_restore.as_deref(),
        Some("A_backup"),
        "armed by name"
    );

    // Now the list moves under it - a refresh, an install, anything.
    app.mods.reverse();

    let _ = update_inner(
        &mut app,
        Message::ConfirmModRestoreBackup("A_backup".to_string()),
    );
    assert_eq!(
        fs::read(root.join("mods").join("A").join("f.txt")).unwrap(),
        b"a",
        "A was restored (status={:?} error={:?})",
        app.status,
        app.error
    );
    assert_eq!(
        fs::read(root.join("mods").join("B").join("f.txt")).unwrap(),
        b"b",
        "and B was not touched"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn nothing_can_act_on_a_row_the_list_is_not_drawing() {
    // The defect this exists for was systemic: sorting and grouping changed
    // what is DRAWN, and the keyboard, shift-extend, select-all and the bulk
    // actions were all still asking "which rows pass the filter" - a
    // different question once the two orders disagree. One answer now.
    let (mut app, root) = list_app(&["Zeta", "SEP_separator", "Alpha", "Mid"]);

    // Load order: exactly what it always was, separators included.
    assert_eq!(drawn_mod_rows(&app), vec![0, 1, 2, 3]);

    // Grouped, with one group folded: the folded rows are drawn by nobody,
    // so nothing may reach them.
    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
    let label = match display_entries(&app).first() {
        Some(ListEntry::Group(l, _)) => l.clone(),
        other => panic!("expected a header, got {other:?}"),
    };
    let _ = update_inner(&mut app, Message::ToggleGroupFold(label));
    assert!(
        drawn_mod_rows(&app).is_empty(),
        "everything is inside the folded group"
    );
    // Select-all, which feeds the batch Remove that DELETES FROM DISK.
    assert!(
        mods_visible_for_bulk(&app).is_empty(),
        "a batch action must not reach a mod nobody can see"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_drag_is_refused_outside_load_order_rather_than_moving_the_wrong_row() {
    // Hiding the insertion strips was not enough: a row's own press still
    // armed a drag, and the gap it aimed at addressed the REAL list while
    // the rows on screen were somewhere else.
    let (mut app, root) = list_app(&["Zeta", "Alpha", "Mid"]);
    let before: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();

    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    assert!(!can_reorder(&app));
    let _ = update_inner(&mut app, Message::SelectMod(0));
    assert!(app.drag_state.is_none(), "no drag is armed while sorted");
    let _ = update_inner(&mut app, Message::DragOverGap(2));
    let _ = update_inner(&mut app, Message::DragDrop);
    assert_eq!(
        app.mods.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
        before,
        "and nothing moved"
    );

    // Back in load order it works again, which is the point of the escape
    // hatch in the View menu.
    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    assert!(can_reorder(&app));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn shift_click_selects_the_run_that_is_on_screen() {
    let (mut app, root) = list_app(&["Zeta", "Alpha", "Mid"]);
    // Sorted by name the drawn order is Alpha(1), Mid(2), Zeta(0).
    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    assert_eq!(drawn_mod_rows(&app), vec![1, 2, 0]);

    let _ = update_inner(&mut app, Message::SelectMod(1));
    let _ = update_inner(&mut app, Message::SelectModExtend(2));
    // Alpha to Mid: two rows, adjacent on screen. Over the raw index range
    // that would have been 1..=2 by luck; the case that broke is below.
    assert_eq!(app.selected_mods.len(), 2);

    // Alpha to Zeta is the WHOLE drawn list. Over raw indices it would have
    // been 0..=1 - Zeta and Alpha - silently missing the row between them.
    let _ = update_inner(&mut app, Message::SelectMod(1));
    let _ = update_inner(&mut app, Message::SelectModExtend(0));
    let mut got: Vec<usize> = app.selected_mods.iter().copied().collect();
    got.sort_unstable();
    assert_eq!(got, vec![0, 1, 2], "every row between them ON SCREEN");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn collapse_all_acts_on_whichever_folds_are_on_screen() {
    let (mut app, root) = list_app(&["A", "SEP_separator", "B"]);
    // Ungrouped: the separators, as it always did.
    let _ = update_inner(&mut app, Message::CollapseAllGroups);
    assert!(!app.collapsed.is_empty());

    // Grouped: the separators are not drawn and their folds are suspended,
    // so folding them would be a menu entry that visibly does nothing.
    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
    let _ = update_inner(&mut app, Message::CollapseAllGroups);
    assert!(
        !app.groups_collapsed.is_empty(),
        "the headers that ARE drawn"
    );
    assert!(drawn_mod_rows(&app).is_empty());
    let _ = update_inner(&mut app, Message::ExpandAllGroups);
    assert!(app.groups_collapsed.is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_separator_fold_cannot_hide_mods_in_a_grouped_list() {
    // The separator that would unfold them is not drawn under a grouping, so
    // a fold left standing hides mods with no way back.
    let (mut app, root) = list_app(&["SEP_separator", "A", "B"]);
    let _ = update_inner(&mut app, Message::ToggleCollapse("SEP".to_string()));
    assert_eq!(drawn_mod_rows(&app).len(), 1, "folded: only the separator");

    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
    assert_eq!(drawn_mod_rows(&app).len(), 2, "the mods are back");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_group_header_counts_only_the_rows_it_actually_heads() {
    let (mut app, root) = list_app(&["Alpha", "Beta"]);
    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
    assert!(matches!(
        display_entries(&app).first(),
        Some(ListEntry::Group(_, 2))
    ));

    // Filter one out: the count follows, and a header left with nothing does
    // not draw at all.
    let _ = update_inner(&mut app, Message::SearchChanged("alpha".to_string()));
    assert!(matches!(
        display_entries(&app).first(),
        Some(ListEntry::Group(_, 1))
    ));
    let _ = update_inner(&mut app, Message::SearchChanged("nothing".to_string()));
    assert!(
        display_entries(&app).is_empty(),
        "no header with nothing under it"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn offline_mode_stops_the_sign_in_that_never_reaches_connect() {
    let mut app = app_for_game("skyrimse");
    app.prefs.offline = true;
    let _ = update_inner(&mut app, Message::NexusSignInStart);
    // Signing in opens a BROWSER before any request is made, so the guard in
    // `Nexus::connect` is not on this path at all.
    assert!(!app.nexus_signing_in, "it must not start");
    assert_eq!(
        app.nexus_error.as_deref(),
        Some(eidos_nexus::OFFLINE_MESSAGE)
    );
}

#[test]
fn an_archive_with_no_sidecar_can_still_be_hidden() {
    // The pile somebody most wants to hide is the one copied in by hand,
    // which has no sidecar at all - and the key-editing helper only edits a
    // file that already exists.
    let (mut app, root) = downloads_instance(&[("ByHand.7z", "")]);
    assert_eq!(app.downloads.len(), 1);

    let _ = update_inner(&mut app, Message::HideDownload("ByHand.7z".to_string()));
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.downloads.is_empty(), "it is hidden");
    assert!(
        root.join("downloads").join("ByHand.7z").is_file(),
        "and still there"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn grouping_puts_every_mod_under_exactly_one_header() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    // Two with a category, one without, plus a separator - which grouping
    // must drop, because it heads rows the grouping has moved.
    for (name, cat) in [
        ("Helm", "5"),
        ("Sword", "5"),
        ("Loose", ""),
        ("SEP_separator", ""),
    ] {
        let dir = root.join("mods").join(name);
        fs::create_dir_all(&dir).unwrap();
        if !cat.is_empty() {
            fs::write(
                dir.join("meta.ini"),
                format!("[General]\ncategory=\"{cat},\"\n"),
            )
            .unwrap();
        }
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = ["Helm", "Sword", "Loose", "SEP_separator"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: root.join("mods").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);

    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Category)));
    let entries = display_entries(&app);
    let headers: Vec<String> = entries
        .iter()
        .filter_map(|e| match e {
            ListEntry::Group(l, _) => Some(l.clone()),
            _ => None,
        })
        .collect();
    // The catch-all sinks to the bottom whatever it is called - it is the
    // pile that needs sorting out, not the first thing anybody wants.
    assert_eq!(headers.last().map(String::as_str), Some("Uncategorised"));
    // Every non-separator mod appears exactly once, and the separator not at
    // all.
    let rows: Vec<usize> = entries
        .iter()
        .filter_map(|e| match e {
            ListEntry::Row(i) => Some(*i),
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 3, "the separator is not a row under a grouping");
    assert!(!rows.contains(&3));
    // The counts on the headers add up to the rows drawn.
    let counted: usize = entries
        .iter()
        .filter_map(|e| match e {
            ListEntry::Group(_, n) => Some(*n),
            _ => None,
        })
        .sum();
    assert_eq!(counted, 3);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn folding_a_group_hides_its_rows_but_keeps_its_count() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["A", "B"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));

    let label = match display_entries(&app).first() {
        Some(ListEntry::Group(l, _)) => l.clone(),
        other => panic!("expected a header, got {other:?}"),
    };
    let _ = update_inner(&mut app, Message::ToggleGroupFold(label.clone()));
    let entries = display_entries(&app);
    assert_eq!(entries.len(), 1, "only the header is left");
    // The count still says how many are inside, which is the whole reason to
    // fold rather than filter.
    assert_eq!(entries[0], ListEntry::Group(label.clone(), 2));

    // And the same click unfolds it.
    let _ = update_inner(&mut app, Message::ToggleGroupFold(label));
    assert_eq!(display_entries(&app).len(), 3);
}

#[test]
fn leaving_a_grouping_returns_the_list_to_load_order() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["A", "B"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    let _ = update_inner(&mut app, Message::SetGroupBy(Some(GroupBy::Source)));
    let _ = update_inner(&mut app, Message::ToggleGroupFold("From Nexus".to_string()));

    let _ = update_inner(&mut app, Message::SetGroupBy(None));
    assert!(app.group_by.is_none());
    // The folds go with it: they key on labels that no longer exist, and a
    // stale one would fold a group with the same name next time.
    assert!(app.groups_collapsed.is_empty());
    assert_eq!(
        display_entries(&app),
        vec![ListEntry::Row(0), ListEntry::Row(1)],
        "back to the real list"
    );
}

#[test]
fn sorting_leaves_the_real_order_alone_and_takes_the_separators_out() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["Zeta", "01 CITIES_separator", "Alpha", "Mid"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;

    // Load order is 0..len, which is what keeps the drag strips valid.
    assert_eq!(display_order(&app), vec![0, 1, 2, 3]);

    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    // A separator is a HEADING; ordered by name it heads nothing, so it
    // leaves the list rather than floating into the middle of it.
    assert_eq!(
        display_order(&app),
        vec![2, 3, 0],
        "Alpha, Mid, Zeta - no separator"
    );
    // And the underlying list is untouched: sorting is a view, not a move.
    assert_eq!(app.mods[0].name, "Zeta");

    // Second click reverses, third returns to load order - which has to be
    // one click away, because it is the only order where dragging works.
    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    assert_eq!(display_order(&app), vec![0, 3, 2]);
    let _ = update_inner(&mut app, Message::CycleModSort(SortKey::Name));
    assert!(app.mod_sort.is_none());
    assert_eq!(display_order(&app), vec![0, 1, 2, 3]);
}

#[test]
fn hiding_a_column_that_the_list_is_sorted_by_returns_it_to_load_order() {
    // Ordering a list by a column nobody can see is a list that looks
    // shuffled for no reason.
    let mut app = app_for_game("skyrimse");
    let _ = update_inner(
        &mut app,
        Message::CycleModSort(SortKey::Column(ModColumn::Version)),
    );
    assert!(app.mod_sort.is_some());
    assert!(app.mod_columns.contains(&ModColumn::Version));

    let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Version));
    assert!(!app.mod_columns.contains(&ModColumn::Version));
    assert!(app.mod_sort.is_none());
}

#[test]
fn columns_are_saved_and_come_back_in_the_headers_order() {
    let mut app = app_for_game("skyrimse");
    // Turned on in a deliberately awkward order.
    let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Game));
    let _ = update_inner(&mut app, Message::ToggleModColumn(ModColumn::Author));

    // Redrawn canonically, so toggling one on cannot move another.
    let after: Vec<&str> = app.mod_columns.iter().map(|c| c.title()).collect();
    assert_eq!(
        after,
        vec!["Category", "Content", "Version", "Author", "Game", "Flags"]
    );

    // And a hand-edited settings file cannot produce a header that
    // disagrees with the rows either.
    let mut prefs = eidos_instance::settings::Settings::default();
    prefs.mod_columns = Some(vec!["game".into(), "category".into()]);
    let back: Vec<&str> = columns_from_settings(&prefs)
        .iter()
        .map(|c| c.title())
        .collect();
    assert_eq!(back, vec!["Category", "Game"]);

    // An EMPTY list is a choice, not "never chosen" - it has to survive a
    // restart rather than springing back to the defaults.
    prefs.mod_columns = Some(Vec::new());
    assert!(columns_from_settings(&prefs).is_empty());
    assert!(!columns_from_settings(&eidos_instance::settings::Settings::default()).is_empty());
}

fn downloads_instance(files: &[(&str, &str)]) -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let dl = inst.downloads_dir();
    fs::create_dir_all(&dl).unwrap();
    for (name, sidecar) in files {
        fs::write(dl.join(name), b"xxxx").unwrap();
        if !sidecar.is_empty() {
            fs::write(dl.join(format!("{name}.meta")), sidecar).unwrap();
        }
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    load_downloads(&mut app);
    (app, root)
}

#[test]
fn hiding_a_download_keeps_the_archive_and_can_be_undone() {
    let (mut app, root) = downloads_instance(&[
        ("Keep.7z", "[General]\nmodID=1\ninstalled=true\n"),
        ("Stale.7z", "[General]\nmodID=2\ninstalled=true\n"),
    ]);
    assert_eq!(app.downloads.len(), 2);

    let _ = update_inner(&mut app, Message::HideDownload("Stale.7z".to_string()));
    assert_eq!(
        app.downloads.len(),
        1,
        "hidden rows are dropped from the list"
    );
    // The whole point: putting a book away is not burning it.
    assert!(
        root.join("downloads").join("Stale.7z").is_file(),
        "the archive must still be there"
    );

    // Show hidden brings it back, and the same button unhides it.
    let _ = update_inner(&mut app, Message::ToggleShowHiddenDownloads);
    assert_eq!(app.downloads.len(), 2);
    let _ = update_inner(&mut app, Message::HideDownload("Stale.7z".to_string()));
    let _ = update_inner(&mut app, Message::ToggleShowHiddenDownloads);
    assert_eq!(app.downloads.len(), 2, "unhidden again");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_bulk_purge_takes_two_clicks_and_only_what_is_on_screen() {
    let (mut app, root) = downloads_instance(&[
        ("Done.7z", "[General]\nmodID=1\ninstalled=true\n"),
        ("AlsoDone.7z", "[General]\nmodID=2\ninstalled=true\n"),
        ("NotYet.7z", "[General]\nmodID=3\n"),
    ]);

    // The filter is how the user said which ones they meant. A bulk delete
    // that ignores it deletes things they were not looking at.
    let _ = update_inner(&mut app, Message::DownloadFilterChanged("also".to_string()));
    assert_eq!(app.downloads.len(), 1);

    // One click only arms.
    let _ = update_inner(&mut app, Message::PurgeInstalledDownloads);
    assert!(app.confirm_purge_installed);
    assert!(root.join("downloads").join("AlsoDone.7z").is_file());

    let _ = update_inner(&mut app, Message::ConfirmPurgeInstalled);
    assert!(
        !root.join("downloads").join("AlsoDone.7z").exists(),
        "the filtered one went"
    );
    assert!(
        root.join("downloads").join("Done.7z").is_file(),
        "the one off screen did not"
    );
    assert!(
        root.join("downloads").join("NotYet.7z").is_file(),
        "and nor did the uninstalled"
    );
    // The sidecar goes with the archive, or the row comes back as a ghost.
    assert!(!root.join("downloads").join("AlsoDone.7z.meta").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_downloads_list_sorts_by_what_was_asked_for() {
    let (mut app, root) = downloads_instance(&[
        ("bbb.7z", "[General]\nmodID=1\nmodName=Zebra\n"),
        ("aaa.7z", "[General]\nmodID=2\nmodName=Apple\n"),
    ]);
    let _ = update_inner(&mut app, Message::DownloadSortChanged(DownloadSort::Name));
    let names: Vec<&str> = app.downloads.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["aaa.7z", "bbb.7z"]);

    // The friendly mod name is searched too: an archive called
    // `SkyUI_5_2_SE-12604.7z` is found by typing "skyui" only if it is.
    let _ = update_inner(
        &mut app,
        Message::DownloadFilterChanged("zebra".to_string()),
    );
    assert_eq!(app.downloads.len(), 1);
    assert_eq!(app.downloads[0].name, "bbb.7z");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_mod_that_looks_like_nothing_this_game_loads_is_flagged_and_can_be_silenced() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    // One real mod, one that is just a readme, and one that ships only a
    // Root/ tree - Eidos's own convention, which must NOT be flagged.
    for (name, inner) in [("Good", "Meshes"), ("Junk", "docs"), ("RootOnly", "Root")] {
        fs::create_dir_all(root.join("mods").join(name).join(inner)).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = ["Good", "Junk", "RootOnly"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: root.join("mods").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);

    assert!(
        !app.meta_cache["Good"].invalid_data,
        "Meshes/ is data this game loads"
    );
    assert!(app.meta_cache["Junk"].invalid_data, "docs/ alone is not");
    assert!(
        !app.meta_cache["RootOnly"].invalid_data,
        "a Root/ mod is correct - flagging it would flag every Root Builder mod"
    );

    // And "Mark as valid" silences it for good, through MO2's own key.
    let _ = update_inner(&mut app, Message::ModMarkValid(1));
    assert!(!app.meta_cache["Junk"].invalid_data);
    let text = fs::read_to_string(root.join("mods").join("Junk").join("meta.ini")).unwrap();
    assert!(
        text.contains("validated=true"),
        "MO2 reads this key too: {text}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_letter_walks_every_match_rather_than_sticking_on_the_first() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["Apachii Hair", "Beyond Skyrim", "Amber Guard", "Cloaks"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;

    let _ = update_inner(&mut app, Message::JumpToLetter('a'));
    assert_eq!(app.selected_mod, Some(0), "the first A");
    let _ = update_inner(&mut app, Message::JumpToLetter('a'));
    assert_eq!(
        app.selected_mod,
        Some(2),
        "the next one, not the same one again"
    );
    // And it wraps, so the list has no dead end.
    let _ = update_inner(&mut app, Message::JumpToLetter('a'));
    assert_eq!(app.selected_mod, Some(0));
    // Case does not matter, and a letter nothing starts with moves nothing.
    let _ = update_inner(&mut app, Message::JumpToLetter('C'));
    assert_eq!(app.selected_mod, Some(3));
    let _ = update_inner(&mut app, Message::JumpToLetter('z'));
    assert_eq!(app.selected_mod, Some(3), "no match leaves the focus alone");
}

#[test]
fn a_letter_never_reaches_a_row_the_filter_is_hiding() {
    // Jumping onto a hidden row moves a highlight nobody can see, and the
    // next Space would then toggle a mod off screen.
    let mut app = app_for_game("skyrimse");
    app.mods = ["Apachii Hair", "Amber Guard"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    app.search = "amber".to_string();

    let _ = update_inner(&mut app, Message::JumpToLetter('a'));
    assert_eq!(
        app.selected_mod,
        Some(1),
        "only the row the filter still draws"
    );
}

#[test]
fn a_double_click_branches_on_the_modifiers_that_are_actually_held() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    fs::create_dir_all(root.join("mods").join("Mod")).unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = vec![ModEntry {
        name: "Mod".to_string(),
        enabled: true,
        path: root.join("mods").join("Mod"),
        unmanaged: false,
    }];
    app.screen = Screen::Main;

    // Plain: Information. The closure that emits the double-click cannot see
    // the modifier set, which is why `update` reads it instead.
    let _ = update_inner(&mut app, Message::ModDoubleClick(0));
    assert_eq!(app.info_mod, Some(0));

    app.info_mod = None;
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    let _ = update_inner(&mut app, Message::ModDoubleClick(0));
    assert!(
        app.info_mod.is_none(),
        "Shift is the Nexus page, not Information"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn collapse_others_leaves_exactly_one_group_open() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["A_separator", "one", "B_separator", "two", "C_separator"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    let keep = app.mods[2].display_name().to_string();
    // Start with the kept one folded, to prove it is opened rather than
    // merely left alone.
    app.collapsed.insert(keep.clone());

    let _ = update_inner(&mut app, Message::CollapseOthers(keep.clone()));

    assert!(
        !app.collapsed.contains(&keep),
        "the one you asked to keep is open"
    );
    for m in app
        .mods
        .iter()
        .filter(|m| m.is_separator() && m.display_name() != keep)
    {
        assert!(
            app.collapsed.contains(m.display_name()),
            "{} should be folded",
            m.name
        );
    }
}

#[test]
fn a_drag_resting_on_a_folded_group_opens_it_but_brushing_past_does_not() {
    let mut app = app_for_game("skyrimse");
    app.mods = ["A_separator", "one", "B_separator", "two"]
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: PathBuf::from("/tmp").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    let folded = app.mods[2].display_name().to_string();
    app.collapsed.insert(folded.clone());

    let _ = update_inner(&mut app, Message::DragStart(1));
    let _ = update_inner(&mut app, Message::DragOverGap(2));
    assert_eq!(
        app.drag_hover_group.as_ref().map(|(n, t)| (n.clone(), *t)),
        Some((folded.clone(), 0))
    );

    // One tick is not enough - brushing past a group on the way somewhere
    // else must not open it.
    let _ = update_inner(&mut app, Message::DragHoverTick);
    assert!(
        app.collapsed.contains(&folded),
        "still folded after one tick"
    );

    // Resting does open it.
    let _ = update_inner(&mut app, Message::DragHoverTick);
    assert!(!app.collapsed.contains(&folded), "resting on it opens it");

    // And moving off a group forgets it, so the counter cannot accumulate
    // across two different groups.
    app.collapsed.insert(folded.clone());
    let _ = update_inner(&mut app, Message::DragOverGap(2));
    let _ = update_inner(&mut app, Message::DragOverGap(1));
    assert!(app.drag_hover_group.is_none());
}

#[test]
fn a_collection_for_another_game_is_refused_by_name() {
    // The failure this exists for: a Skyrim collection opened while a
    // Fallout 4 instance is loaded would join its members against Fallout 4's
    // mods and downloads, so every "installed" and every "missing" is noise
    // shaped like an answer.
    let root = temp_portable("fallout4");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("fallout4");
    app.created = Some(inst);
    app.screen = Screen::Main;

    let _ = update_inner(
        &mut app,
        Message::ShowCollection(
            "nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest".to_string(),
        ),
    );
    let c = app.collection.as_ref().unwrap();
    let err = c.error.clone().unwrap_or_default();
    assert!(
        err.contains("skyrimspecialedition"),
        "it names the collection's game: {err}"
    );
    assert!(err.contains("Fallout"), "and the one that is open: {err}");
    assert!(!c.loading, "and no request was dispatched");
    assert!(c.revision.is_none());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_collection_for_the_open_game_gets_past_the_guard() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;

    let _ = update_inner(
        &mut app,
        Message::ShowCollection(
            "nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest".to_string(),
        ),
    );
    let c = app.collection.as_ref().unwrap();
    // It gets as far as the credential check, which is the next gate - the
    // game guard is not what stopped it.
    assert!(
        c.error
            .as_deref()
            .is_none_or(|e| !e.contains("instance is")),
        "{:?}",
        c.error
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_mod_link_pasted_into_the_collection_box_says_what_it_is() {
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::ShowCollection(String::new()));
    let _ = update_inner(
        &mut app,
        Message::CollectionLinkChanged(
            "nxm://skyrimspecialedition/mods/266/files/1234".to_string(),
        ),
    );
    let _ = update_inner(&mut app, Message::CollectionFetch);
    let err = app
        .collection
        .as_ref()
        .unwrap()
        .error
        .clone()
        .unwrap_or_default();
    // Not "bad link": it IS a valid link, to the wrong kind of thing, and
    // saying which sends the user to the button that handles it.
    assert!(err.contains("single mod"), "{err}");
}

#[test]
fn a_malformed_collection_link_is_reported_without_a_request() {
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::ShowCollection("not a link".to_string()));
    let c = app.collection.as_ref().unwrap();
    assert!(c.error.is_some());
    assert!(!c.loading, "nothing was dispatched");
    assert!(c.revision.is_none());
}

#[test]
fn every_timer_tick_is_ambient() {
    // A tick that counts as an ACTION cancels every armed two-click
    // confirmation before the second click can land. The saves watcher fires
    // every 2.5s and the log tail every 1.5s, so on those screens the
    // confirmations were a coin flip. Asserted as a set rather than one by
    // one, so the next tick that gets added is caught here.
    let app = nav_app(&[]);
    for m in [
        Message::DownloadTick,
        Message::SavesTick,
        Message::LogRefresh,
        // The sharpest case: this one fires sixty times a second, so
        // treating it as an action would not shorten a confirmation's life,
        // it would end it between the two clicks every time.
        Message::AnimationTick,
        Message::ArchivePoll,
        // Reached from that same tick and at the same rate, for a job that
        // runs for twenty minutes - long enough that a confirmation
        // disarmed sixty times a second reads as a broken window.
        Message::TransferPoll,
        Message::PointerAt(iced::Point::ORIGIN),
        Message::ModifiersChanged(iced::keyboard::Modifiers::default()),
    ] {
        assert!(is_ambient(&app, &m), "{m:?} must not disarm a confirmation");
    }
}

#[test]
fn a_saves_tick_does_not_cancel_the_delete_it_is_ticking_beside() {
    let (mut app, root) = saves_app();
    app.tab = Tab::Saves;
    let _ = update_inner(&mut app, Message::SaveToggleSelect(0));
    let _ = update_inner(&mut app, Message::SavesDeleteSelected);
    assert!(app.confirm_saves_delete);
    // The watcher runs on its own, between the two clicks.
    let _ = update(&mut app, Message::SavesTick);
    assert!(
        app.confirm_saves_delete,
        "the tick cancelled the user's arming"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_saves_reload_keeps_the_whole_selection_not_just_the_focus() {
    let (mut app, root) = saves_app();
    let a = app
        .saves
        .iter()
        .position(|s| s.filename == "Save1.ess")
        .unwrap();
    let b = app
        .saves
        .iter()
        .position(|s| s.filename == "Save2.ess")
        .unwrap();
    let _ = update_inner(&mut app, Message::SaveToggleSelect(a));
    let _ = update_inner(&mut app, Message::SaveToggleSelect(b));
    assert_eq!(app.selected_saves.len(), 2);

    // The game writes an autosave, renumbering everything. The batch bar is
    // built on this set, so losing it takes the bar off screen mid-gesture.
    let dir = app.created.as_ref().unwrap().active().saves_dir();
    std::thread::sleep(std::time::Duration::from_millis(15));
    fs::write(dir.join("Autosave.ess"), b"z").unwrap();
    let _ = update_inner(&mut app, Message::SavesTick);

    let names: Vec<String> = app
        .selected_saves
        .iter()
        .filter_map(|&i| app.saves.get(i))
        .map(|s| s.filename.clone())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.contains(&"Save1.ess".to_string()));
    assert!(names.contains(&"Save2.ess".to_string()));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_transfer_never_lands_a_cosave_beside_another_characters_save() {
    let (mut app, root) = saves_app();
    let inst = app.created.clone().unwrap();
    let dest = inst.profile("Second").saves_dir();
    let idx = app
        .saves
        .iter()
        .position(|s| s.filename == "Save1.ess")
        .unwrap();
    let stem = app.saves[idx]
        .path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    // The destination already holds a DIFFERENT character's save under that
    // stem. Guarding per file copied the co-save anyway - a co-save that
    // silently belongs to the wrong game state.
    fs::write(dest.join(format!("{stem}.ess")), b"SOMEONE ELSE").unwrap();

    let _ = update_inner(&mut app, Message::SaveToggleSelect(idx));
    let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));
    assert_eq!(
        fs::read(dest.join(format!("{stem}.ess"))).unwrap(),
        b"SOMEONE ELSE"
    );
    assert!(
        !dest.join(format!("{stem}.skse")).exists(),
        "the co-save must not be planted beside a save it does not belong to"
    );
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("already existed"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn deleting_selected_saves_takes_their_cosaves_and_does_not_shift_underneath_itself() {
    let (mut app, root) = saves_app();
    assert_eq!(app.saves.len(), 2);
    let dir = app.created.as_ref().unwrap().active().saves_dir();

    let _ = update_inner(&mut app, Message::SaveToggleSelect(0));
    let _ = update_inner(&mut app, Message::SaveToggleSelect(1));
    let _ = update_inner(&mut app, Message::SavesDeleteSelected);
    assert!(app.confirm_saves_delete, "the first click only arms");
    let _ = update_inner(&mut app, Message::SavesDeleteSelected);

    // Both gone, INCLUDING the co-save - which the game does not know about
    // and which orphans invisibly if it is left.
    assert!(!dir.join("Save1.ess").exists());
    assert!(
        !dir.join("Save1.skse").exists(),
        "the co-save travels with its save"
    );
    assert!(!dir.join("Save2.ess").exists());
    assert!(app.saves.is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn copying_saves_to_another_profile_copies_the_whole_group_and_never_overwrites() {
    let (mut app, root) = saves_app();
    let inst = app.created.clone().unwrap();
    let dest = inst.profile("Second").saves_dir();

    // The list is newest-first, so pick the one that HAS a co-save by name
    // rather than assuming an index.
    let idx = app
        .saves
        .iter()
        .position(|s| s.filename == "Save1.ess")
        .expect("Save1");
    let _ = update_inner(&mut app, Message::SaveToggleSelect(idx));
    let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));

    let moved = app.saves[idx].clone();
    let stem = moved
        .path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        dest.join(format!("{stem}.ess")).is_file(),
        "{:?}",
        app.status
    );
    // The co-save comes too: a save without it is one the script extender
    // cannot restore its state for.
    assert!(dest.join(format!("{stem}.skse")).is_file());
    // And the source is untouched - this is a copy, not a move.
    assert!(moved.path.is_file());

    // A second copy must not clobber somebody's character.
    fs::write(dest.join(format!("{stem}.ess")), b"DIFFERENT").unwrap();
    let _ = update_inner(&mut app, Message::SavesCopyToProfile("Second".into()));
    assert_eq!(
        fs::read(dest.join(format!("{stem}.ess"))).unwrap(),
        b"DIFFERENT"
    );
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("already existed"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_saves_tick_only_reloads_when_the_directory_really_moved() {
    let (mut app, root) = saves_app();
    let _ = update_inner(&mut app, Message::SelectSave(0));
    let picked = app.saves[0].path.clone();
    assert_eq!(app.selected_save, Some(0));

    // A quiet tick changes nothing - reloading twice a second would close
    // the details pane under the user's hands.
    let _ = update_inner(&mut app, Message::SavesTick);
    assert_eq!(app.selected_save, Some(0));

    // A new save appears. The list reloads AND the selection follows its save
    // by path, because a new autosave renumbers every index.
    let dir = app.created.as_ref().unwrap().active().saves_dir();
    std::thread::sleep(std::time::Duration::from_millis(15));
    fs::write(dir.join("Save3.ess"), b"z").unwrap();
    let _ = update_inner(&mut app, Message::SavesTick);
    assert_eq!(app.saves.len(), 3);
    assert_eq!(
        app.selected_save
            .and_then(|i| app.saves.get(i))
            .map(|s| s.path.clone()),
        Some(picked),
        "the pane must not silently start describing a different save"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_conflict_map_is_not_the_same_as_nothing_to_send_back() {
    // "The question has not been asked" and "the answer is none" must not
    // look the same: one is fixed by opening a tab, the other by doing
    // something else entirely.
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = mods(&["a"]);
    app.screen = Screen::Main;

    assert!(app.conflicts.is_none());
    assert!(overwrite_owners(&app).is_none());
    let _ = update_inner(&mut app, Message::OverwriteSyncToMods);
    assert!(app.confirm_sync, "the first click only arms");
    let _ = update_inner(&mut app, Message::OverwriteSyncToMods);
    assert!(
        app.status
            .as_deref()
            .unwrap_or("")
            .contains("Conflicts tab"),
        "{:?}",
        app.status
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_owner_of_an_overwrite_file_is_the_best_mod_under_it() {
    use eidos_conflicts::{ConflictMap, FileNode};
    let mut app = nav_app(&["Low", "High"]);
    let mut map = ConflictMap::default();
    // The Overwrite wins; under it, mod index 1 ("High", origin 2) beats
    // index 0 ("Low", origin 1), and the game's own Data (origin 0) is never
    // a destination - Eidos does not write there.
    map.files.insert(
        "meshes/a.nif".to_string(),
        FileNode {
            winner: u32::MAX,
            alternatives: vec![2, 1, 0],
            display_path: "meshes/a.nif".to_string(),
        },
    );
    // A file only the game provides underneath: nowhere to send it.
    map.files.insert(
        "vanilla.esm".to_string(),
        FileNode {
            winner: u32::MAX,
            alternatives: vec![0],
            display_path: "vanilla.esm".into(),
        },
    );
    // A file the Overwrite does NOT win is not its business at all.
    map.files.insert(
        "other.txt".to_string(),
        FileNode {
            winner: 2,
            alternatives: vec![1],
            display_path: "other.txt".into(),
        },
    );
    app.conflicts = Some(map);

    let owners = overwrite_owners(&app).unwrap();
    assert_eq!(owners.get("meshes/a.nif").map(String::as_str), Some("High"));
    assert!(
        !owners.contains_key("vanilla.esm"),
        "the game's Data is not a destination"
    );
    assert!(!owners.contains_key("other.txt"));
}

#[test]
fn renaming_a_portable_instance_moves_the_folder_and_the_registry_with_it() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    // NEVER the real user registry: these handlers persist.
    app.registry_path = root
        .parent()
        .unwrap()
        .join(format!("reg-{}.ini", std::process::id()));
    app.known = vec![KnownInstance {
        label: "Skyrim - portable".into(),
        inst: inst.clone(),
        game_index: 0,
        portable: true,
    }];
    app.instances_open = true;

    let dest = root
        .parent()
        .unwrap()
        .join(format!("renamed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dest);
    let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
    let _ = update_inner(
        &mut app,
        Message::InstanceRenameChanged(
            dest.file_name().unwrap().to_string_lossy().into_owned(),
        ),
    );
    let _ = update_inner(&mut app, Message::InstanceRenameCommit);

    assert!(dest.is_dir(), "the folder moved: {:?}", app.status);
    assert!(!root.exists());
    let _ = fs::remove_dir_all(&dest);
}

#[test]
fn the_open_instance_cannot_be_renamed_out_from_under_the_window() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.registry_path = root
        .parent()
        .unwrap()
        .join(format!("reg2-{}.ini", std::process::id()));
    app.created = Some(inst.clone());
    app.known = vec![KnownInstance {
        label: "Skyrim - portable".into(),
        inst,
        game_index: 0,
        portable: true,
    }];

    let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
    let _ = update_inner(
        &mut app,
        Message::InstanceRenameChanged("something-else".into()),
    );
    let _ = update_inner(&mut app, Message::InstanceRenameCommit);
    // Every cached path in the window - and the lock it holds - points at
    // the old root.
    assert!(root.is_dir(), "it was renamed anyway");
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("Switch to another"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_rename_that_is_not_a_folder_name_is_refused() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.registry_path = root
        .parent()
        .unwrap()
        .join(format!("reg3-{}.ini", std::process::id()));
    app.known = vec![KnownInstance {
        label: "x".into(),
        inst,
        game_index: 0,
        portable: true,
    }];
    for bad in ["", "  ", "..", ".", "a/b", "a\\b"] {
        let _ = update_inner(&mut app, Message::InstanceRenameStart(0));
        let _ = update_inner(&mut app, Message::InstanceRenameChanged(bad.into()));
        let _ = update_inner(&mut app, Message::InstanceRenameCommit);
        assert!(root.is_dir(), "{bad:?} moved something");
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn forgetting_an_instance_needs_two_clicks_and_never_touches_the_disk() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.registry_path = root
        .parent()
        .unwrap()
        .join(format!("reg4-{}.ini", std::process::id()));
    app.known = vec![KnownInstance {
        label: "x".into(),
        inst,
        game_index: 0,
        portable: true,
    }];

    let _ = update_inner(&mut app, Message::InstanceForget(0));
    assert_eq!(app.confirm_forget, Some(0), "the first click only arms");
    let _ = update_inner(&mut app, Message::InstanceForget(0));
    // Forgotten is not deleted, and that distinction is the whole design: an
    // instance is a mod pool, not a preference.
    assert!(root.is_dir(), "the folder must survive");
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("Nothing on disk"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_global_instance_offers_neither_rename_nor_forget() {
    let mut app = app_for_game("skyrimse");
    app.registry_path = std::env::temp_dir().join(format!("reg5-{}.ini", std::process::id()));
    app.known = vec![KnownInstance {
        label: "Skyrim - global".into(),
        inst: Instance::global("skyrimse"),
        game_index: 0,
        portable: false,
    }];
    let _ = update_inner(&mut app, Message::InstanceForget(0));
    let _ = update_inner(&mut app, Message::InstanceForget(0));
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("derived from the game id"));
}

#[test]
fn an_update_check_records_what_is_left_of_the_budget() {
    let mut app = nav_app(&[]);
    let result = eidos_nexus::UpdateCheckResult {
        checked: 12,
        queried: 12,
        updates_found: 1,
        updates: Vec::new(),
        rate_limited: false,
        hourly_remaining: Some(1388),
        daily_remaining: Some(2100),
        unavailable: Vec::new(),
        failures: Vec::new(),
    };
    let _ = update_inner(&mut app, Message::UpdatesChecked(Ok(result)));
    // It arrived in the result and used to be dropped on the floor.
    assert_eq!(app.nexus_hourly_left, Some(1388));
    assert_eq!(app.nexus_daily_left, Some(2100));
    assert!(nexus_budget_suffix(&app).contains("1388"));
}

#[test]
fn an_update_check_reports_failed_requests_as_incomplete() {
    let mut app = nav_app(&[]);
    let result = eidos_nexus::UpdateCheckResult {
        checked: 1,
        queried: 1,
        failures: vec![("Example Mod".into(), "HTTP 503".into())],
        ..Default::default()
    };
    let _ = update_inner(&mut app, Message::UpdatesChecked(Ok(result)));
    let status = app.status.as_deref().unwrap();
    assert!(status.contains("1 mod(s) could not be checked"));
    assert!(status.contains("0 update(s) found"));
}

#[test]
fn the_plugin_priority_field_never_outlives_its_menu() {
    let mut app = nav_app(&[]);
    let mut list = PluginList::default();
    for n in ["A.esp", "B.esp", "C.esp"] {
        list.plugins.push(plugin_row(n, "Some Mod"));
    }
    app.plugins = Some(list);
    app.screen = Screen::Main;

    let _ = update_inner(&mut app, Message::OpenPluginMenu(2));
    let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
    assert_eq!(app.plugin_send_priority.as_ref().map(|(r, _)| *r), Some(2));
    assert_eq!(app.menu_plugin, Some(2), "the field lives INSIDE the card");

    // Dismissing the card drops it - a half-typed index must not be armed
    // for whatever menu opens next.
    let _ = update_inner(&mut app, Message::ClosePluginMenu);
    assert!(app.plugin_send_priority.is_none());

    // And reopening on another row never inherits the old aim.
    let _ = update_inner(&mut app, Message::OpenPluginMenu(0));
    let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
    let _ = update_inner(&mut app, Message::PluginSendToPriorityChanged("2".into()));
    let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
    assert!(
        app.plugin_send_priority.is_none(),
        "a new menu starts clean"
    );
}

#[test]
fn a_plugin_priority_that_is_not_a_number_says_so_and_moves_nothing() {
    let mut app = nav_app(&[]);
    let mut list = PluginList::default();
    for n in ["A.esp", "B.esp"] {
        list.plugins.push(plugin_row(n, "Some Mod"));
    }
    app.plugins = Some(list);
    app.screen = Screen::Main;
    let before: Vec<String> = app
        .plugins
        .as_ref()
        .unwrap()
        .plugins
        .iter()
        .map(|p| p.name.clone())
        .collect();

    let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
    let _ = update_inner(&mut app, Message::PluginSendToPriorityStart);
    let _ = update_inner(&mut app, Message::PluginSendToPriorityChanged("  ".into()));
    let _ = update_inner(&mut app, Message::PluginSendToPriorityCommit);
    // "Row number", not "load index": the only numeric column the pane
    // draws is the game's hex load index, which this field does NOT take.
    assert!(app.status.as_deref().unwrap_or("").contains("row number"));
    let after: Vec<String> = app
        .plugins
        .as_ref()
        .unwrap()
        .plugins
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(before, after);
    assert!(
        app.plugin_send_priority.is_none(),
        "the field closes either way"
    );
}

/// A menu-bar dropdown opens in TWO steps: the click asks iced where the
/// button is, and the answer opens the menu at that rectangle. Driven here
/// the way the runtime drives it, because the Task the click returns does
/// not run in a test.
fn open_bar_menu(app: &mut App, click: Message, at: Message) {
    let _ = update_inner(app, click);
    let _ = update_inner(app, at);
}

#[test]
fn only_one_menu_bar_dropdown_is_open_at_a_time() {
    // Two cards at the same corner would overlap, and the one underneath
    // would eat clicks aimed at the one on top.
    let mut app = nav_app(&[]);
    let some = |x: f32| {
        Some(iced::Rectangle {
            x,
            y: 38.0,
            width: 36.0,
            height: 30.9,
        })
    };
    open_bar_menu(
        &mut app,
        Message::OpenFileMenu,
        Message::FileMenuAt(some(5.0)),
    );
    assert!(app.file_menu_open && !app.view_menu_open);
    open_bar_menu(
        &mut app,
        Message::OpenViewMenu,
        Message::ViewMenuAt(some(41.0)),
    );
    assert!(app.view_menu_open && !app.file_menu_open);
    open_bar_menu(
        &mut app,
        Message::OpenFileMenu,
        Message::FileMenuAt(some(5.0)),
    );
    assert!(app.file_menu_open && !app.view_menu_open);
    let _ = update_inner(&mut app, Message::CloseFileMenu);
    assert!(!app.file_menu_open);
}

/// The measurement is what a dropdown hangs from, and it must be REMEMBERED:
/// the answer that arrives is the button's real rectangle, and a menu drawn
/// from a stale or absent one is the defect this whole mechanism replaced.
#[test]
fn a_dropdown_remembers_where_its_button_was_measured() {
    let mut app = nav_app(&[]);
    assert_eq!(app.file_menu_at, None, "nothing measured yet");
    let real = iced::Rectangle {
        x: 5.0,
        y: 38.0,
        width: 36.4,
        height: 30.9,
    };
    let _ = update_inner(&mut app, Message::FileMenuAt(Some(real)));
    assert_eq!(app.file_menu_at, Some(real));
    assert!(app.file_menu_open, "the answer is what opens it");

    // A measurement that comes back empty must not erase the last good one -
    // the menu would then fall back to a guess it had already improved on.
    let _ = update_inner(&mut app, Message::CloseFileMenu);
    let _ = update_inner(&mut app, Message::FileMenuAt(None));
    assert_eq!(app.file_menu_at, Some(real), "kept, not cleared");
    assert!(app.file_menu_open, "and it still opens");
}

/// The filter pane is the third surface with the same defect, and it toggles
/// rather than opening - so closing must NOT go looking for a rectangle.
#[test]
fn the_filter_pane_measures_on_the_way_open_and_not_on_the_way_shut() {
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::ToggleFilterPane);
    assert!(!app.filters_open, "the click only asks where the button is");
    let _ = update_inner(
        &mut app,
        Message::FiltersAt(Some(iced::Rectangle {
            x: 411.0,
            y: 72.9,
            width: 56.0,
            height: 26.0,
        })),
    );
    assert!(app.filters_open);
    let _ = update_inner(&mut app, Message::ToggleFilterPane);
    assert!(!app.filters_open, "shut, with no second round trip");
}

#[test]
fn the_file_menu_offers_every_folder_that_resolves_and_no_others() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    app.file_menu_open = true;

    // Eidos's own folders stay live even before they exist - several are
    // created on first use, and "not there yet" is a worse answer than an
    // empty folder.
    let i = app.created.as_ref().unwrap();
    let downloads = i.downloads_dir();
    assert!(!downloads.exists(), "not created until the first download");
    let _ = update_inner(&mut app, Message::OpenFolder(downloads.clone()));
    assert!(downloads.is_dir(), "opening it created it");
    // The card builds without an open Proton prefix - that entry is simply
    // the inert one, which is the whole point of drawing rather than hiding.
    assert!(app
        .games
        .first()
        .and_then(|g| g.compatdata.as_ref())
        .is_none());
    let _ = file_menu_card(&app);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_colour_can_be_set_on_an_ordinary_mod_but_never_on_the_games_own_content() {
    let root = temp_portable("skyrimse");
    let mut app = app_for_game("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    fs::create_dir_all(root.join("mods/Real")).unwrap();
    app.created = Some(inst);
    app.mods = vec![
        ModEntry {
            name: "Real".into(),
            enabled: true,
            path: root.join("mods/Real"),
            unmanaged: false,
        },
        ModEntry {
            name: "Skyrim.esm".into(),
            enabled: true,
            path: PathBuf::from("/game/Data/Skyrim.esm"),
            unmanaged: true,
        },
    ];
    app.screen = Screen::Main;

    let _ = update_inner(
        &mut app,
        Message::SetSeparatorColor(0, Some([0x2e, 0x5e, 0x8b])),
    );
    let meta = app.created.as_ref().unwrap().mod_meta("Real");
    assert_eq!(
        meta.color(),
        Some([0x2e, 0x5e, 0x8b]),
        "an ordinary mod takes a colour now"
    );

    // The game's own Data is never written to.
    let before = app.status.clone();
    let _ = update_inner(
        &mut app,
        Message::SetSeparatorColor(1, Some([0x8b, 0x2e, 0x2e])),
    );
    assert_ne!(
        app.status, before,
        "it says something rather than silently doing it"
    );
    assert!(!PathBuf::from("/game/Data/Skyrim.esm/meta.ini").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_saved_note_reaches_the_row_immediately() {
    let root = temp_portable("skyrimse");
    let mut app = app_for_game("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    fs::create_dir_all(root.join("mods/Real")).unwrap();
    app.created = Some(inst);
    app.mods = vec![ModEntry {
        name: "Real".into(),
        enabled: true,
        path: root.join("mods/Real"),
        unmanaged: false,
    }];
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);
    assert_eq!(app.meta_cache["Real"].notes, None);

    app.info_mod = Some(0);
    app.notes_edit = "needs the AE patch".to_string();
    let _ = update_inner(&mut app, Message::NotesSave);
    // Without dropping the cached row the glyph would not appear until the
    // next full refresh - which is exactly the first time anyone adds a note.
    assert_eq!(
        app.meta_cache["Real"].notes.as_deref(),
        Some("needs the AE patch")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn installing_from_a_menu_lands_where_the_menu_was_opened() {
    let mut app = nav_app(&["a", "b", "c"]);
    // "Install below b" = the gap after index 1.
    let _ = update_inner(&mut app, Message::InstallAt(2));
    assert_eq!(
        app.install_gap,
        Some(2),
        "the place is held until an archive names itself"
    );
    assert_eq!(
        app.install_at, None,
        "and it is NOT an aim yet - there is no archive"
    );

    // The picker returns one: now it becomes a real aim, paired.
    let _ = update_inner(
        &mut app,
        Message::ModPicked(Some(PathBuf::from("/tmp/M.7z"))),
    );
    assert_eq!(app.install_gap, None, "consumed");
    assert_eq!(
        app.install_at,
        Some((2, PathBuf::from("/tmp/M.7z"))),
        "paired with the archive, so a later failure cannot move something else"
    );
}

/// The window must come back from `update()` with the archive UNOPENED.
/// Doing the extraction here is what froze it: iced processes no events
/// while this call is on the stack, so the compositor stops getting pings
/// and any resize it sent goes unanswered until 7-Zip finishes.
#[test]
fn a_picked_archive_is_handed_to_a_worker_instead_of_blocking_the_window() {
    let (mut app, root) = data_app(&[], &[]);
    let archive = root.join("Some Mod.7z");
    fs::write(&archive, b"not really an archive").unwrap();

    let _ = update_inner(&mut app, Message::ModPicked(Some(archive.clone())));

    let job = app.install_job.as_ref().expect("a job was started");
    assert_eq!(job.archive, archive);
    assert_eq!(job.name, eidos_install::mod_name_for(&archive));
    assert!(
        app.fomod.is_none() && app.picker.is_none() && app.collision.is_none(),
        "nothing was decided yet - the archive has not been read"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A job in flight has no animation running, so the old gate left the window
/// subscribed to nothing: the dialog would never repaint and the finished
/// worker would never be noticed.
#[test]
fn the_window_keeps_receiving_frames_while_an_archive_extracts() {
    let mut app = nav_app(&[]);
    assert!(!anim::needs_frames(&app), "idle asks for nothing");
    app.install_job = Some(fake_job(None));
    assert!(anim::needs_frames(&app));
}

/// A pack or unpack with no worker behind it. `outcome` decides whether it
/// reads as finished, so the whole state machine can be driven without
/// 7-Zip, which CI does not have.
fn fake_transfer(kind: TransferKind, outcome: Option<Result<String, String>>) -> TransferJob {
    use std::sync::atomic::{AtomicBool, AtomicU8};
    use std::sync::{Arc, Mutex};
    let done = outcome.is_some();
    TransferJob {
        kind,
        title: "Packing Eidos-Skyrim".into(),
        target: PathBuf::from("/tmp/eidos-gui-test-nowhere/skyrimse.eidos"),
        percent: Arc::new(AtomicU8::new(0)),
        outcome: Arc::new(Mutex::new(outcome)),
        done: Arc::new(AtomicBool::new(done)),
        finished: None,
    }
}

/// The same gate as the extraction, and the same silent failure without it:
/// no frames means no repaint and no poll, so the job finishes on its thread
/// and the window never finds out.
#[test]
fn the_window_keeps_receiving_frames_while_an_instance_is_packed() {
    let mut app = nav_app(&[]);
    assert!(!anim::needs_frames(&app), "idle asks for nothing");
    app.transfer_job = Some(fake_transfer(TransferKind::Pack, None));
    assert!(anim::needs_frames(&app));
    // And stops when there is only a report left to read: sixty frames a
    // second to redraw a sentence is what this gate exists to avoid.
    let _ = update_inner(&mut app, Message::TransferPoll);
    app.transfer_job.as_mut().unwrap().finished = Some(Ok("done".into()));
    assert!(!anim::needs_frames(&app), "a finished job wants nothing");
}

/// The report has to SURVIVE the poll. Clearing the job the way the install
/// does would take the only thing the welcome screen can show - there is no
/// status bar there to fall back on.
#[test]
fn a_finished_transfer_keeps_its_report_until_it_is_closed() {
    let mut app = nav_app(&[]);
    app.transfer_job = Some(fake_transfer(
        TransferKind::Pack,
        Some(Err("Cannot pack now: in use by the Eidos window.".into())),
    ));
    let _ = update_inner(&mut app, Message::TransferPoll);
    let job = app.transfer_job.as_ref().expect("the card stays up");
    let msg = match job.finished.as_ref().expect("collected") {
        Ok(s) | Err(s) => s.clone(),
    };
    assert!(msg.contains("in use by the Eidos window"), "{msg}");
    // A second poll must not collect again - it arrives sixty times a second.
    let _ = update_inner(&mut app, Message::TransferPoll);
    assert!(app.transfer_job.as_ref().unwrap().finished.is_some());
    let _ = update_inner(&mut app, Message::CloseTransferResult);
    assert!(app.transfer_job.is_none());
}

/// A worker that panicked leaves `outcome` empty behind a `done` flag. The
/// user is watching a dialog, so it must say something rather than sit there.
#[test]
fn a_transfer_whose_worker_vanished_still_reports() {
    let mut app = nav_app(&[]);
    let mut job = fake_transfer(TransferKind::Pack, None);
    job.done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    app.transfer_job = Some(job);
    let _ = update_inner(&mut app, Message::TransferPoll);
    let job = app.transfer_job.as_ref().unwrap();
    assert!(matches!(job.finished, Some(Err(_))), "must not hang");
}

/// Unpacking is the one action that has to work with NO instance open: a
/// fresh machine has none, which is why somebody is holding a .eidos file.
#[test]
fn the_unpack_dialog_opens_with_no_instance_and_the_pack_dialog_does_not() {
    let mut app = nav_app(&[]);
    assert!(app.created.is_none());
    let _ = update_inner(&mut app, Message::ShowUnpackDialog);
    assert!(app.unpack.is_some(), "the whole point of the feature");
    let _ = update_inner(&mut app, Message::CloseUnpackDialog);
    assert!(app.unpack.is_none());
    // Pack acts ON an instance, so it says so instead of opening onto one.
    let _ = update_inner(&mut app, Message::ShowPackDialog);
    assert!(app.pack.is_none());
    assert!(app.status.unwrap_or_default().contains("instance"));
}

/// A collection install is the third long job, and it shares the one slot
/// the other two use - they are all minutes of archive work on one instance.
#[test]
fn a_collection_install_uses_the_same_single_job_slot() {
    let mut app = nav_app(&[]);
    app.transfer_job = Some(fake_transfer(TransferKind::Collection, None));
    assert!(
        anim::needs_frames(&app),
        "it must get frames like the others"
    );
    // And nothing else may start beside it.
    let _ = update_inner(&mut app, Message::ShowUnpackDialog);
    assert!(app.unpack.is_none());
    let _ = update_inner(&mut app, Message::CollectionInstall);
    assert!(
        app.status.clone().unwrap_or_default().contains("already"),
        "{:?}",
        app.status
    );
}

#[test]
fn collection_runtime_mismatch_requires_a_current_explicit_continuation() {
    let (mut app, root) = collection_app(&[], &[]);
    let target = CollectionTarget {
        instance: root.clone(),
        profile: app.created.as_ref().unwrap().active_profile(),
        installation: app.games[0].selection_id(),
    };
    app.collection = Some(CollectionState {
        install_check: Some(7),
        runtime: None,
        link: "request".into(),
        revision: Some(captured_revision()),
        states: Vec::new(),
        loading: true,
        error: None,
        confirm_fetch: false,
        asked: Default::default(),
    });
    let mismatch = eidos_collections::recipe::RuntimeCheck::Mismatch {
        observed: "1.7.104.0".into(),
        expected: vec!["1.6.1170.0".into()],
    };
    for (request, changed) in [(6, false), (7, true)] {
        let mut stale = target.clone();
        if changed {
            stale.profile = "another profile".into();
        }
        let _ = update_inner(
            &mut app,
            Message::CollectionInstallChecked {
                request,
                target: stale,
                link: "request".into(),
                result: Ok(mismatch.clone()),
            },
        );
        assert!(app.collection.as_ref().unwrap().runtime.is_none());
        assert!(app.transfer_job.is_none());
    }
    let _ = update_inner(
        &mut app,
        Message::CollectionInstallChecked {
            request: 7,
            target,
            link: "request".into(),
            result: Ok(mismatch),
        },
    );
    assert!(
        app.transfer_job.is_none(),
        "checking an incompatible recipe installs nothing"
    );
    assert!(app.collection.as_ref().unwrap().runtime.is_some());
    assert!(app
        .collection
        .as_ref()
        .unwrap()
        .error
        .as_ref()
        .unwrap()
        .contains("1.7.104.0"));
    let _ = update_inner(
        &mut app,
        Message::CollectionLinkChanged("other recipe".into()),
    );
    assert!(
        app.collection.as_ref().unwrap().runtime.is_none(),
        "a changed recipe needs its own decision"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn collision_backup_choice_preserves_payload_metadata_and_disabled_state() {
    for merge in [false, true] {
        let (mut app, root) = collection_app(&[("Existing", 1)], &[]);
        let inst = app.created.as_ref().unwrap().clone();
        fs::write(root.join("mods/Existing/chosen.esp"), b"old payload").unwrap();
        let old_meta = fs::read(root.join("mods/Existing/meta.ini")).unwrap();
        app.mods[0].enabled = false;
        inst.active().save_modlist(&app.mods).unwrap();
        let archive = root.join("reinstall.zip");
        fs::write(&archive, include_bytes!("../../tests/fixtures/reinstall.zip")).unwrap();
        app.collision = Some(CollisionPrompt {
            target: update::collection_target(&app).unwrap(),
            backup: false,
            archive,
            name: "Existing".into(),
            game_id: "skyrimse".into(),
            rename_to: "Existing2".into(),
            fomod: false,
            tree: None,
            pick: None,
        });
        let _ = update_inner(&mut app, Message::CollisionBackupChanged(true));
        assert!(app.prefs.retain_install_backup);
        assert!(eidos_instance::Settings::parse(&app.prefs.to_ini()).retain_install_backup);
        let _ = update_inner(
            &mut app,
            if merge {
                Message::CollisionMerge
            } else {
                Message::CollisionReplace
            },
        );
        assert_eq!(
            fs::read(root.join("mods/Existing/chosen.esp")).unwrap(),
            b"new payload",
            "{:?}",
            app.status
        );
        assert_eq!(
            fs::read(root.join("mods/Existing_backup/chosen.esp")).unwrap(),
            b"old payload"
        );
        assert_eq!(
            fs::read(root.join("mods/Existing_backup/meta.ini")).unwrap(),
            old_meta
        );
        assert!(
            !inst
                .modlist()
                .iter()
                .find(|m| m.name == "Existing")
                .unwrap()
                .enabled
        );
        assert!(inst
            .modlist()
            .iter()
            .filter(|m| m.is_backup())
            .all(|m| !m.is_active()));
        assert!(app.status.as_ref().unwrap().contains("Backup retained"));
        fs::remove_dir_all(root).unwrap();
    }
}

/// Installing a collection acts ON an instance, so with none open it says so
/// rather than starting a worker with nowhere to put anything.
#[test]
fn installing_a_collection_needs_an_instance() {
    let mut app = nav_app(&[]);
    assert!(app.created.is_none());
    let _ = update_inner(&mut app, Message::CollectionInstall);
    assert!(app.transfer_job.is_none());
    assert!(app.status.unwrap_or_default().contains("instance"));
}

/// Two 7-Zips on one disk racing each other is slower than either alone, and
/// a pack reading an instance a drop is installing into is worse than slow.
#[test]
fn nothing_else_starts_while_a_transfer_runs() {
    let mut app = nav_app(&[]);
    app.transfer_job = Some(fake_transfer(TransferKind::Pack, None));
    let _ = update_inner(&mut app, Message::ShowUnpackDialog);
    assert!(app.unpack.is_none(), "one at a time");
    assert!(app.status.clone().unwrap_or_default().contains("already"));
    app.dropped = vec![PathBuf::from("/tmp/a.7z")];
    let _ = update_inner(&mut app, Message::DrainDrops);
    assert_eq!(app.dropped.len(), 1, "the drop waits its turn");
    assert!(app.install_job.is_none());
}

/// Polling before the worker is done must leave the job alone - taking the
/// (empty) outcome would drop the install on the floor.
#[test]
fn polling_an_unfinished_job_leaves_it_running() {
    let mut app = nav_app(&[]);
    app.install_job = Some(fake_job(None));
    let _ = update_inner(&mut app, Message::InstallPoll);
    assert!(app.install_job.is_some(), "still extracting");
}

/// And a failure has to reach the status bar: the worker is off-screen, so
/// an error it swallows is an install that silently never happened.
#[test]
fn a_finished_job_reports_its_failure_and_clears_itself() {
    let mut app = nav_app(&[]);
    app.install_job = Some(fake_job(Some(Err(eidos_install::InstallError::Extract(
        "Cannot open the file as archive".into(),
    )))));
    let _ = update_inner(&mut app, Message::InstallPoll);
    assert!(app.install_job.is_none(), "the finished job is taken");
    let s = app.status.clone().unwrap_or_default();
    assert!(s.contains("Cannot open the file as archive"), "{s}");
}

/// A multi-file drop is walked one archive at a time. Without this the queue
/// would spawn a worker per file at once: several 7-Zip processes competing
/// for the disk, and their extractions racing to create `mods/<name>/`.
#[test]
fn a_queued_archive_waits_for_the_one_being_extracted() {
    let (mut app, root) = data_app(&[], &[]);
    app.install_job = Some(fake_job(None));
    app.dropped = vec![root.join("Second.7z")];
    fs::write(root.join("Second.7z"), b"x").unwrap();

    let _ = update_inner(&mut app, Message::DrainDrops);

    assert_eq!(app.dropped.len(), 1, "still queued, untouched");
    assert_eq!(
        app.install_job.as_ref().map(|j| j.name.clone()).as_deref(),
        Some("Some Mod"),
        "and the running job was not replaced"
    );
    let _ = fs::remove_dir_all(&root);
}

/// An `InstallJob` with no worker behind it: `outcome` decides whether it
/// reads as finished, so the state machine can be driven without 7-Zip
/// (which CI does not have).
fn fake_job(
    outcome: Option<Result<eidos_install::Opened, eidos_install::InstallError>>,
) -> InstallJob {
    use std::sync::atomic::{AtomicBool, AtomicU8};
    use std::sync::{Arc, Mutex};
    let done = outcome.is_some();
    InstallJob {
        fresh: false,
        cancel: Arc::new(AtomicBool::new(false)),
        target: None,
        name: "Some Mod".into(),
        archive: PathBuf::from("/tmp/Some Mod.7z"),
        percent: Arc::new(AtomicU8::new(0)),
        outcome: Arc::new(Mutex::new(outcome)),
        done: Arc::new(AtomicBool::new(done)),
    }
}

#[test]
fn cancelling_the_picker_forgets_the_position() {
    let mut app = nav_app(&["a", "b"]);
    let _ = update_inner(&mut app, Message::InstallAt(1));
    let _ = update_inner(&mut app, Message::ModPicked(None));
    assert_eq!(
        app.install_gap, None,
        "a cancelled pick must not aim the NEXT install"
    );
    assert_eq!(app.install_at, None);
}

#[test]
fn a_menu_install_under_a_filter_says_it_is_going_to_the_end() {
    let mut app = nav_app(&["Alpha", "Bravo"]);
    app.search = "Alpha".to_string();
    let _ = update_inner(&mut app, Message::InstallAt(1));
    // The gap between two visible rows means nothing when rows are hidden -
    // the same promise the drag makes, made the same way.
    assert_eq!(app.install_gap, None);
    assert!(app
        .pending_note
        .as_deref()
        .unwrap_or("")
        .contains("end of the list"));
}

#[test]
fn an_empty_mod_can_be_created_at_a_position() {
    let root = temp_portable("skyrimse");
    let mut app = app_for_game("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    app.created = Some(inst);
    app.mods = mods(&["a", "b", "c"]);
    app.screen = Screen::Main;

    let _ = update_inner(&mut app, Message::CreateEmptyModAt(1));
    assert_eq!(app.mods.len(), 4);
    assert_eq!(
        app.selected_mod,
        Some(1),
        "the new row is where it was asked for"
    );
    assert!(app.mods[1].name.starts_with("New Mod"));
    assert_eq!(
        app.mods[2].name, "b",
        "everything after it shifted, not got overwritten"
    );
    assert!(app.selected_mods.is_empty(), "stale indices are dropped");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bulk_enable_touches_only_what_is_on_screen_and_needs_two_clicks() {
    let mut app = nav_app(&["Alpha", "Bravo", "Ivy"]);
    for m in app.mods.iter_mut() {
        m.enabled = false;
    }
    // A filter that hides Ivy.
    app.search = "a".to_string(); // Alpha, Bravo
    assert_eq!(mods_visible_for_bulk(&app), vec![0, 1]);

    // First click only arms.
    let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
    assert_eq!(app.confirm_set_all, Some(true));
    assert!(!app.mods[0].enabled, "nothing happened yet");

    let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
    assert!(app.mods[0].enabled && app.mods[1].enabled);
    assert!(!app.mods[2].enabled, "the hidden row was NOT touched");
    assert_eq!(app.confirm_set_all, None, "disarmed after firing");
}

#[test]
fn arming_enable_all_and_clicking_disable_all_does_not_fire() {
    let mut app = nav_app(&["a", "b"]);
    let _ = update_inner(&mut app, Message::SetAllModsEnabled(true));
    assert_eq!(app.confirm_set_all, Some(true));
    // A different TARGET is a different action: it re-arms, it does not fire.
    let _ = update_inner(&mut app, Message::SetAllModsEnabled(false));
    assert_eq!(app.confirm_set_all, Some(false));
    assert!(app.mods.iter().all(|m| m.enabled), "nothing was disabled");
}

#[test]
fn a_download_can_be_dropped_into_an_empty_mod_list() {
    // A fresh instance has no rows, so there is nothing to aim at except the
    // trailing strip. Without it the drag can never become aimed and the
    // release installs nothing, silently.
    let mut app = nav_app(&[]);
    app.downloads = vec![dl_row("First.7z")];
    let _ = update_inner(&mut app, Message::DownloadDragStart(0));
    let _ = update_inner(&mut app, Message::DownloadDragOverGap(0));
    assert!(app.download_drag.as_ref().is_some_and(|d| d.aimed));
    let _ = update_inner(&mut app, Message::DownloadDragDrop);
    assert_eq!(app.install_at.as_ref().map(|(g, _)| *g), Some(0));
}

#[test]
fn merging_onto_an_existing_mod_never_moves_it() {
    // The mod already has a place in the load order; honouring a drop's
    // target priority would yank it out and flip every conflict it is in.
    let mut app = nav_app(&["a", "b"]);
    app.install_at = Some((0, PathBuf::from("/tmp/Mod.7z")));
    let _ = update_inner(&mut app, Message::CollisionMerge);
    assert_eq!(app.install_at, None, "the aim is discarded, not applied");
}

#[test]
fn a_cancelled_install_does_not_leave_its_target_for_the_next_one() {
    let mut app = nav_app(&["a"]);
    for cancel in [
        Message::FomodCancel,
        Message::PickerCancel,
        Message::CollisionCancel,
    ] {
        app.install_at = Some((0, PathBuf::from("/tmp/Mod.7z")));
        let _ = update_inner(&mut app, cancel);
        assert_eq!(app.install_at, None);
    }
}

#[test]
fn a_multi_file_drop_is_drained_one_file_at_a_time() {
    let mut app = nav_app(&["a"]);
    // Three files arrive as three messages, not one.
    for n in ["one.7z", "two.zip", "three.rar"] {
        let _ = update_inner(
            &mut app,
            Message::FileDropped(PathBuf::from("/tmp").join(n)),
        );
    }
    assert_eq!(app.dropped.len(), 3, "queued, not handled inline");

    // With no instance open, the queue is dropped with one explanation
    // rather than three.
    assert!(app.created.is_none());
    let _ = update_inner(&mut app, Message::DrainDrops);
    assert!(app.dropped.is_empty());
    assert!(app
        .status
        .as_deref()
        .unwrap_or("")
        .contains("game instance"));
}

#[test]
fn a_dropped_file_that_is_not_a_mod_archive_is_named_and_skipped() {
    let mut app = app_for_game("skyrimse");
    let root = temp_portable("skyrimse");
    app.created = Some(Instance::portable(root.clone()));
    let _ = update_inner(&mut app, Message::FileDropped(root.join("notes.txt")));
    let _ = update_inner(&mut app, Message::DrainDrops);
    let msg = app.status.clone().unwrap_or_default();
    assert!(msg.contains("notes.txt"), "{msg}");
    assert!(msg.contains(".7z"), "and it says what IS accepted: {msg}");
    let _ = fs::remove_dir_all(&root);
}

/// A throwaway logs dir with one session file, and the pane loaded from it.
fn log_fixture(body: &str) -> (LogPaneState, PathBuf) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "eidos-logs-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::create_dir_all(&dir);
    let f = dir.join("gui.20260824-170411.1234.log");
    fs::write(&f, body).unwrap();
    (
        load_log_pane(vec![f.clone()], f, eidos_log::Level::Info),
        dir,
    )
}

#[test]
fn a_tool_extension_with_an_unresolvable_placeholder_is_refused_not_run() {
    let mut app = nav_app(&[]);
    assert!(
        app.created.is_none(),
        "no instance, so the instance placeholder cannot resolve"
    );
    app.addons = vec![eidos_addons::parse_addon(
        "id='x'\nname='X'\nkind='tool'\nexec='sh'\nargs=['-c','ls {instance}']\n",
        std::path::Path::new("/x.toml"),
    )
    .unwrap()];
    let _ = update_inner(&mut app, Message::RunAddon("x".to_string()));
    let msg = app.status.clone().unwrap_or_default();
    assert!(msg.contains("instance"), "it names what is missing: {msg}");
    assert!(msg.contains("needs"), "{msg}");
}

#[test]
fn the_log_pane_filters_by_level_and_says_what_it_is_hiding() {
    let (pane, dir) = log_fixture(
        "2026-08-24 17:04:11.238 DEBUG resolving layers\n\
         2026-08-24 17:04:11.239 INFO  mounted 412 layers\n\
         2026-08-24 17:04:11.240 ERROR could not open the prefix\n",
    );
    assert_eq!(pane.total, 3, "every record is counted, filtered or not");
    assert_eq!(pane.lines.len(), 2, "Debug is below the floor");
    assert_eq!(
        pane.lines[0],
        (eidos_log::Level::Info, "mounted 412 layers".to_string())
    );
    assert_eq!(pane.lines[1].0, eidos_log::Level::Error);
    assert!(!pane.truncated);

    // Lowering the floor shows everything.
    let all = load_log_pane(
        pane.files.clone(),
        pane.current.clone(),
        eidos_log::Level::Debug,
    );
    assert_eq!(all.lines.len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_multi_line_message_stays_attached_to_its_record() {
    let (pane, dir) = log_fixture(
        "2026-08-24 17:04:11.240 ERROR mount failed:\n    \
         fuse: device not found\n    try: modprobe fuse\n",
    );
    // Three lines on disk, ONE record: the continuation lines carry no
    // level, and inventing one for them would put text at a severity
    // nothing claimed.
    assert_eq!(pane.total, 1);
    assert_eq!(pane.lines.len(), 1);
    assert!(
        pane.lines[0].1.contains("device not found"),
        "{:?}",
        pane.lines[0].1
    );
    assert!(pane.lines[0].1.contains("modprobe fuse"));

    // And filtering it out takes its continuations with it.
    let quiet = load_log_pane(
        pane.files.clone(),
        pane.current.clone(),
        eidos_log::Level::Debug,
    );
    assert_eq!(quiet.lines.len(), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_filtered_out_records_continuations_do_not_land_on_the_record_before_it() {
    // `concat!` rather than a `\`-continued literal: the continuation eats
    // the following source indentation INTO the next line, which pushes the
    // timestamp off the offset `parse_line` reads.
    let (pane, dir) = log_fixture(concat!(
        "2026-08-24 17:04:11.238 ERROR mount failed\n",
        "2026-08-24 17:04:11.239 DEBUG probing layers\n",
        "    layer 3: /mods/AAA\n",
        "    layer 4: /mods/BBB\n",
    ));
    // The Debug record is below the floor; its two continuation lines belong
    // to IT, not to the error above. Attaching them there would put a debug
    // trace under an error's severity and call it evidence.
    assert_eq!(pane.lines.len(), 1);
    assert_eq!(pane.lines[0].1, "mount failed", "{:?}", pane.lines[0].1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_line_orphaned_by_the_tail_seek_is_dropped_rather_than_guessed_at() {
    // Reading only the END of a big file lands mid-line. That fragment
    // belongs to a record that is not in the buffer, so there is nothing to
    // attach it to and nothing honest to say about its level.
    let (pane, dir) = log_fixture("ount 412 layers\n2026-08-24 17:04:11.239 INFO  mounted\n");
    assert_eq!(pane.lines.len(), 1);
    assert_eq!(pane.lines[0].1, "mounted");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_or_unreadable_session_is_not_a_panic() {
    let (pane, dir) = log_fixture("");
    assert_eq!(pane.total, 0);
    assert!(pane.lines.is_empty());
    let _ = fs::remove_dir_all(&dir);

    // A file that is not there at all.
    let gone = std::env::temp_dir().join("eidos-no-such.log");
    let pane = load_log_pane(vec![gone.clone()], gone, eidos_log::Level::Info);
    assert!(pane.lines.is_empty());
}

#[test]
fn the_ini_editor_writes_the_profiles_copy_in_the_encoding_it_found() {
    let (mut app, root) = data_app(&[], &[]);
    let inst = app.created.clone().unwrap();
    let prof = inst.active();
    // A CP1252 file: Windows-written game INIs are as often this as UTF-8,
    // and re-encoding one silently mangles every accented value in it.
    let path = prof.ini_path("Skyrim.ini");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"[General]\nsLanguage=Fran\xe7ais").unwrap();

    let _ = update_inner(&mut app, Message::ShowIniEditor);
    let ed = app.ini_editor.as_ref().expect("the editor opened");
    assert_eq!(ed.current, "Skyrim.ini");
    assert!(
        ed.cp1252,
        "the file was not UTF-8 and must be written back as it was read"
    );
    assert!(ed.original.contains("Français"), "decoded: {}", ed.original);
    assert!(!ed.dirty, "opening a file is not an edit");
    assert!(!ed.missing);

    // Saving an untouched buffer must not grow the file. `Content::text()`
    // always ends with a newline, and this one did not.
    let _ = update_inner(&mut app, Message::IniEditorSave);
    assert_eq!(
        fs::read(&path).unwrap(),
        b"[General]\nsLanguage=Fran\xe7ais",
        "byte-identical after a no-op save"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_ini_editor_refuses_to_switch_away_from_unsaved_edits() {
    let (mut app, root) = data_app(&[], &[]);
    let _ = update_inner(&mut app, Message::ShowIniEditor);
    {
        let ed = app.ini_editor.as_mut().unwrap();
        ed.dirty = true;
    }
    let _ = update_inner(
        &mut app,
        Message::IniEditorPick("SkyrimPrefs.ini".to_string()),
    );
    assert_eq!(
        app.ini_editor.as_ref().unwrap().current,
        "Skyrim.ini",
        "switching would have thrown the edits away without saying so"
    );
    assert!(app.status.as_deref().unwrap_or("").contains("unsaved"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_ini_the_profile_does_not_have_yet_opens_empty_and_says_so() {
    let (mut app, root) = data_app(&[], &[]);
    let _ = update_inner(&mut app, Message::ShowIniEditor);
    let ed = app.ini_editor.as_ref().unwrap();
    assert!(ed.missing, "a fresh profile owns none of them");
    assert!(ed.original.is_empty());
    assert!(
        !ed.cp1252,
        "an absent file is not CP1252 - it would be written back wrong"
    );

    // Saving creates it, and the flag clears.
    let _ = update_inner(&mut app, Message::IniEditorSave);
    assert!(!app.ini_editor.as_ref().unwrap().missing);
    assert!(app
        .created
        .as_ref()
        .unwrap()
        .active()
        .ini_path("Skyrim.ini")
        .is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_button_says_how_many_criteria_are_narrowing_the_list() {
    // A list that looks short must always say why.
    let mut app = nav_app(&["a"]);
    assert_eq!(app.filters.active_count(), 0);
    app.filters.active = Criterion::Require;
    app.filters.update = Criterion::Exclude;
    assert_eq!(app.filters.active_count(), 2);
    let _ = update_inner(&mut app, Message::ClearFilters);
    assert_eq!(app.filters.active_count(), 0, "Clear really clears");
}

#[test]
fn send_to_top_moves_as_far_as_the_engine_allows_not_to_row_zero() {
    // The defect this shipped with: gap 0 is refused for every plugin,
    // because the game's own masters sit above them - so the action did
    // nothing at all, silently.
    let spec = GameSpec::for_id("skyrimse").unwrap();
    let mut list = PluginList::default();
    list.plugins.push(plugin_row("Skyrim.esm", ""));
    for n in ["A.esp", "B.esp", "C.esp"] {
        list.plugins.push(plugin_row(n, "Some Mod"));
    }
    list.plugins[0].is_master = true;
    // C is last; sending it to the top must land it above A, not above the
    // master.
    let gap = list
        .edge_gap(&[3], true, &spec)
        .expect("a reachable destination");
    assert!(gap >= 1, "never above the game's own master, got {gap}");
    assert!(
        list.move_plugins_to(&[3], gap, &spec),
        "and the move is accepted"
    );
    list.refresh(&spec);
    let names: Vec<&str> = list.plugins.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names[0], "Skyrim.esm", "the master did not move");
    assert_eq!(names[1], "C.esp", "C went as high as it may: {names:?}");

    // Already there: reported as such rather than rewritten.
    assert_eq!(list.edge_gap(&[1], true, &spec), None);
}

#[test]
fn send_to_bottom_lands_the_selection_last() {
    let spec = GameSpec::for_id("skyrimse").unwrap();
    let mut list = PluginList::default();
    for n in ["A.esp", "B.esp", "C.esp"] {
        list.plugins.push(plugin_row(n, "Some Mod"));
    }
    let gap = list.edge_gap(&[0], false, &spec).expect("a destination");
    assert!(list.move_plugins_to(&[0], gap, &spec));
    list.refresh(&spec);
    let names: Vec<&str> = list.plugins.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names.last(), Some(&"A.esp"), "{names:?}");
    assert_eq!(list.edge_gap(&[2], false, &spec), None, "already last");
}

#[test]
fn activate_all_leaves_the_engines_own_plugins_alone() {
    // Writing them would be a lie the next refresh silently corrects.
    let mut app = app_for_game("skyrimse");
    let mut list = PluginList::default();
    list.plugins.push(plugin_row("Skyrim.esm", ""));
    list.plugins.push(plugin_row("Mine.esp", "Some Mod"));
    list.plugins[0].enabled = true;
    list.plugins[1].enabled = false;
    app.plugins = Some(list);
    let _ = update_inner(&mut app, Message::PluginsSetAll(true));
    let list = app.plugins.as_ref().unwrap();
    assert!(
        list.plugins
            .iter()
            .find(|p| p.name == "Mine.esp")
            .unwrap()
            .enabled
    );
    // And saying nothing changed is itself an outcome worth stating.
    let _ = update_inner(&mut app, Message::PluginsSetAll(true));
    assert!(
        app.status.as_deref().unwrap_or("").contains("already"),
        "{:?}",
        app.status
    );
}

#[test]
fn a_second_identify_cannot_start_while_one_is_running() {
    // Two whole-file hashes racing to write one sidecar.
    let mut app = nav_app(&[]);
    app.identifying_download = Some("a.zip".into());
    let _ = update_inner(&mut app, Message::IdentifyDownload("b.zip".into()));
    assert_eq!(
        app.identifying_download.as_deref(),
        Some("a.zip"),
        "the first one still owns it"
    );
}

#[test]
fn right_clicking_a_plugin_outside_the_selection_takes_that_row_alone() {
    // Same rule as the mod list: otherwise a batch action would run on rows
    // the user can no longer see.
    let mut app = app_for_game("skyrimse");
    let mut list = PluginList::default();
    for n in ["A.esp", "B.esp", "C.esp"] {
        list.plugins.push(plugin_row(n, "Some Mod"));
    }
    app.plugins = Some(list);
    app.selected_plugins.extend([0, 1]);
    let _ = update_inner(&mut app, Message::OpenPluginMenu(2));
    assert_eq!(app.menu_plugin, Some(2), "the menu opens on the row");
    assert!(app.selected_plugins.is_empty(), "the old set is dropped");
    assert_eq!(app.selected_plugin, Some(2));

    // Right-clicking INSIDE the selection keeps it whole.
    app.selected_plugins.extend([0, 1]);
    let _ = update_inner(&mut app, Message::OpenPluginMenu(1));
    assert_eq!(app.selected_plugins.len(), 2, "the set survives");
}

#[test]
fn opening_the_origin_of_a_vanilla_plugin_says_so_instead_of_doing_nothing() {
    // A menu action that silently no-ops reads as a broken button.
    let mut app = app_for_game("skyrimse");
    let mut list = PluginList::default();
    list.plugins.push(plugin_row("Skyrim.esm", ""));
    app.plugins = Some(list);
    let _ = update_inner(&mut app, Message::OpenPluginOrigin(0));
    assert!(
        app.status
            .as_deref()
            .unwrap_or("")
            .contains("game's own Data"),
        "{:?}",
        app.status
    );
}

#[test]
fn the_backups_dialog_reads_both_lists_and_restores_through_the_instance() {
    use eidos_instance::BackupKind;
    let root = std::env::temp_dir().join(format!("eidos-gui-bk-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let inst = eidos_instance::Instance::portable(root.clone());
    inst.create().unwrap();
    let prof = inst.active();
    fs::write(prof.dir().join("modlist.txt"), "+Kept\n").unwrap();

    let mut app = nav_app(&[]);
    app.created = Some(inst);
    let _ = update_inner(&mut app, Message::CreateBackup(BackupKind::ModList));
    let _ = update_inner(&mut app, Message::ShowBackupsDialog);
    let state = app.backups.as_ref().expect("the dialog opened");
    assert_eq!(state.mods.len(), 1, "the mod-list restore point is listed");
    let stamp = state.mods[0].stamp;

    // Destroy the list, restore it through the message the button sends.
    fs::write(prof.dir().join("modlist.txt"), "+Ruined\n").unwrap();
    let _ = update_inner(&mut app, Message::RestoreBackup(BackupKind::ModList, stamp));
    assert_eq!(
        fs::read_to_string(prof.dir().join("modlist.txt")).unwrap(),
        "+Kept\n"
    );
    assert!(app.backups.is_none(), "the dialog closes on a restore");
    assert!(
        app.status.as_deref().unwrap_or("").contains("Restored"),
        "the outcome is said out loud: {:?}",
        app.status
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn releasing_the_button_that_armed_a_confirmation_does_not_cancel_it() {
    // The reported bug, and the half the previous fix missed: the pointer
    // fix covered MOVING, but every left-button release publishes
    // PointerReleased, so letting go of the very click that armed Delete
    // disarmed it again. The confirmation flashed for as long as the button
    // was held down - about a tenth of a second - and could never be
    // completed.
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    let _ = update_inner(&mut app, Message::PointerReleased);
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("a.zip"),
        "letting go is the end of that same click, not a new action"
    );
    // The same release must not cancel the other confirmations either.
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::DeleteSave(0));
    let _ = update_inner(&mut app, Message::PointerReleased);
    assert!(app.confirm_delete_save.is_some(), "saves too");

    // Every drag the release ladder can cancel has to be exempt. Adding one
    // and forgetting it here reintroduces the exact bug above, through a
    // different door: the release emits the cancel, the cancel counts as an
    // action, and the confirmation is gone before the second click lands.
    let mut app = nav_app(&[]);
    for cancel in [
        Message::DragCancel,
        Message::PluginDragCancel,
        Message::DownloadDragCancel,
    ] {
        app.confirm_delete_download = Some("a.zip".into());
        let _ = update_inner(&mut app, cancel.clone());
        assert_eq!(
            app.confirm_delete_download.as_deref(),
            Some("a.zip"),
            "cancelling a drag is the absence of an action"
        );
    }
}

#[test]
fn a_release_that_commits_a_drag_still_counts_as_an_action() {
    // The exemption is not blanket: a release that DROPS a dragged mod is a
    // real action, and it must cancel an armed confirmation like any other.
    let mut app = nav_app(&["a", "b", "c"]);
    app.drag_state = Some(DragState {
        from: 0,
        gap: 2,
        aimed: true,
    });
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    let _ = update_inner(&mut app, Message::PointerReleased);
    assert_eq!(
        app.confirm_delete_download, None,
        "a committed drop is an action"
    );
}

#[test]
fn holding_a_modifier_does_not_cancel_an_armed_confirmation() {
    // Ctrl is how a multi-selection is made, so pressing or releasing it
    // around a batch action is part of that gesture, not a decision to do
    // something else. The handler only stores the modifier set.
    let mut app = nav_app(&["a", "b"]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    let _ = update_inner(
        &mut app,
        Message::ModifiersChanged(iced::keyboard::Modifiers::CTRL),
    );
    assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"));
}

#[test]
fn a_frame_of_animation_does_not_cancel_an_armed_confirmation() {
    // Sixty a second while a tab is cross-fading. Arm Delete, switch tabs so
    // the strip animates, and the arming must survive every frame of it.
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"));

    for _ in 0..12 {
        let _ = update(&mut app, Message::AnimationTick);
    }
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("a.zip"),
        "the frame timer disarmed the user's confirmation"
    );
}

#[test]
fn changing_tab_crossfades_from_the_one_being_left() {
    let mut app = nav_app(&[]);
    app.tab = Tab::Data;
    assert!(
        !anim::animating(&app),
        "an idle window must ask for no frames"
    );

    let _ = update(&mut app, Message::SelectTab(Tab::Saves));
    assert_eq!(app.tab, Tab::Saves);
    assert_eq!(
        app.tab_prev,
        Some(Tab::Data),
        "the strip needs both ends to cross-fade"
    );
    assert!(
        anim::animating(&app),
        "the frame timer should be running now"
    );

    // Mid-flight the two ends share the transition between them.
    let t = anim::at(&app, &app.tab_anim);
    let arriving = anim::tab_mix(t, &app.tab, app.tab_prev.as_ref(), &Tab::Saves);
    let leaving = anim::tab_mix(t, &app.tab, app.tab_prev.as_ref(), &Tab::Data);
    assert!((arriving + leaving - 1.0).abs() < 1e-6);
    // And a tab that took no part in it is simply unselected.
    assert_eq!(
        anim::tab_mix(t, &app.tab, app.tab_prev.as_ref(), &Tab::Archives),
        0.0
    );
}

#[test]
fn re_selecting_the_tab_already_open_starts_nothing() {
    // Otherwise clicking the current tab restarts a transition from itself
    // to itself, which flashes.
    let mut app = nav_app(&[]);
    app.tab = Tab::Data;
    let _ = update(&mut app, Message::SelectTab(Tab::Data));
    assert_eq!(app.tab_prev, None);
    assert!(!anim::animating(&app));
}

#[test]
fn a_new_status_message_fades_in_however_it_was_set() {
    // The fade is armed by comparison after the message, not by each of the
    // dozen places that assign `status` - so this must work for a path that
    // knows nothing about animation.
    let mut app = nav_app(&[]);
    assert!(!anim::animating(&app));

    app.status = Some("Installed 3 mods.".to_string());
    let _ = update(&mut app, Message::Noop);
    assert!(anim::animating(&app), "a new status message must fade in");
    assert_eq!(app.status_shown.as_deref(), Some("Installed 3 mods."));

    // The SAME message arriving again is not a new one, and must not restart
    // the fade - otherwise a repeating status strobes.
    app.status_anim = anim::Phase::default();
    let _ = update(&mut app, Message::Noop);
    assert!(
        !anim::animating(&app),
        "an unchanged status restarted the fade"
    );
}

#[test]
fn with_motion_off_nothing_animates_and_no_frames_are_asked_for() {
    // "Reduced motion" has to mean none, not faster: every animated value is
    // drawn at its destination and the frame timer is never subscribed.
    let mut app = nav_app(&[]);
    let _ = update(&mut app, Message::ToggleMotion(false));
    assert!(!app.prefs.motion, "the preference is what gets saved");
    assert!(
        !app.motion,
        "the live copy has to follow, or it keeps animating"
    );

    app.tab = Tab::Data;
    let _ = update(&mut app, Message::SelectTab(Tab::Saves));
    app.status = Some("Something happened.".to_string());
    let _ = update(&mut app, Message::Noop);

    assert!(
        !anim::animating(&app),
        "motion is off; nothing may ask for frames"
    );
    // Drawn at the end state: the selected tab is fully selected, the one
    // left behind fully unselected, on the very first frame.
    let t = anim::at(&app, &app.tab_anim);
    assert_eq!(t, 1.0);
    assert_eq!(
        anim::tab_mix(t, &app.tab, app.tab_prev.as_ref(), &Tab::Saves),
        1.0
    );
    assert_eq!(
        anim::tab_mix(t, &app.tab, app.tab_prev.as_ref(), &Tab::Data),
        0.0
    );
    assert_eq!(anim::at(&app, &app.status_anim), 1.0);
}

#[test]
fn moving_the_mouse_does_not_cancel_an_armed_confirmation() {
    // The reported bug: arm Delete on a download, twitch the mouse, and the
    // confirmation is gone. The pointer HAS to move to reach the button, so
    // the two-click guard could never be completed.
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("a.zip"),
        "the first click arms it"
    );

    let _ = update_inner(&mut app, Message::PointerAt(iced::Point::new(10.0, 10.0)));
    let _ = update_inner(
        &mut app,
        Message::WindowResized(iced::Size::new(800.0, 600.0)),
    );
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("a.zip"),
        "ambient messages are not actions"
    );
}

#[test]
fn a_real_action_still_cancels_every_confirmation() {
    // The guard must not become decorative: the whole point is that doing
    // anything ELSE takes the loaded gun out of your hand.
    let mut app = nav_app(&[]);
    for (arm, check) in [
        (Message::DeleteDownload("a.zip".into()), 0),
        (Message::DeleteSave(0), 1),
        (Message::ClearOverwrite, 2),
        (Message::BatchRemoveMods, 3),
    ] {
        let _ = update_inner(&mut app, arm);
        let _ = update_inner(&mut app, Message::Refresh);
        match check {
            0 => assert_eq!(app.confirm_delete_download, None),
            1 => assert_eq!(app.confirm_delete_save, None),
            2 => assert!(!app.confirm_clear),
            _ => assert!(!app.confirm_batch_remove),
        }
    }
}

#[test]
fn arming_one_row_disarms_another() {
    let mut app = nav_app(&[]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    let _ = update_inner(&mut app, Message::DeleteDownload("b.zip".into()));
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("b.zip"),
        "only one may be armed"
    );
}

#[test]
fn the_delete_confirmation_survives_the_background_tick() {
    // The tick re-sorts the list twice a second. Keyed by index, arming a
    // row and confirming it could delete a DIFFERENT archive.
    let mut app = downloads_app(&[("a.zip", b"a"), ("b.zip", b"b")], &[]);
    let _ = update_inner(&mut app, Message::DeleteDownload("a.zip".into()));
    assert_eq!(app.confirm_delete_download.as_deref(), Some("a.zip"));
    let _ = update_inner(&mut app, Message::DownloadTick);
    assert_eq!(
        app.confirm_delete_download.as_deref(),
        Some("a.zip"),
        "a periodic re-scan is not an action"
    );
}

#[test]
fn deleting_a_stalled_download_takes_the_partial_with_it() {
    // Otherwise the file that produced the row is still there, and the row
    // is back on the next tick - an entry the user cannot get rid of.
    let mut app = downloads_app(
        &[("dead.zip.unfinished", b"half")],
        &[("dead.zip.meta", "[General]\ntotalSize=999\n")],
    );
    let dl = app.created.as_ref().unwrap().downloads_dir();
    // BOTH files have to be old. A fresh sidecar means a retry is in its
    // API/latency window, and calling that stalled is what used to hand the
    // user a Delete button aimed at a live transfer.
    let old = std::time::SystemTime::now() - STALLED_AFTER - std::time::Duration::from_secs(30);
    for f in ["dead.zip.unfinished", "dead.zip.meta"] {
        fs::File::options()
            .write(true)
            .open(dl.join(f))
            .unwrap()
            .set_modified(old)
            .unwrap();
    }
    load_downloads(&mut app);
    assert_eq!(app.downloads[0].state, DownloadState::Stalled);

    let _ = update_inner(&mut app, Message::DeleteDownload("dead.zip".into()));
    let _ = update_inner(&mut app, Message::ConfirmDeleteDownload("dead.zip".into()));
    assert!(
        !dl.join("dead.zip.unfinished").exists(),
        "the partial must go"
    );
    assert!(!dl.join("dead.zip.meta").exists(), "and its sidecar");
    assert!(app.downloads.is_empty(), "so the row does not come back");
}

#[test]
fn cleaning_debris_only_touches_eidos_install_folders() {
    // The handler deletes recursively inside `mods/`, which is where every
    // mod the user owns also lives. The prefix is the ONLY thing standing
    // between the two, so it is worth a test of its own.
    let mut app = nav_app(&[]);
    let root = std::env::temp_dir().join(format!("eidos-debris-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mods = root.join("mods");
    // Dead pids (past the kernel's default pid_max, so /proc never has
    // them): genuine debris from crashed installs.
    for d in [".eidos-install-4194305-0", ".eidos-install-4194306-0"] {
        fs::create_dir_all(mods.join(d).join("00 Core")).unwrap();
        fs::write(mods.join(d).join("00 Core/a.esp"), b"x").unwrap();
    }
    // A temp whose embedded pid is ALIVE (this very test process) is a
    // running install's workspace - deleting it failed that install while
    // the button called it safe debris.
    let live = format!(".eidos-install-{}-0", std::process::id());
    fs::create_dir_all(mods.join(&live)).unwrap();
    fs::write(mods.join(&live).join("extracting.7z.part"), b"x").unwrap();
    // Everything that must survive: a real mod, a separator, and a dotfile
    // that is not ours.
    fs::create_dir_all(mods.join("A Real Mod/meshes")).unwrap();
    fs::write(mods.join("A Real Mod/meshes/m.nif"), b"keep").unwrap();
    fs::create_dir_all(mods.join("Group_separator")).unwrap();
    fs::create_dir_all(mods.join(".git")).unwrap();
    app.created = Some(eidos_instance::Instance::portable(root.clone()));

    let _ = update_inner(&mut app, Message::CleanInstallDebris);

    assert!(!mods.join(".eidos-install-4194305-0").exists());
    assert!(!mods.join(".eidos-install-4194306-0").exists());
    assert!(
        mods.join(&live).is_dir(),
        "a live install's temp is not debris"
    );
    assert_eq!(
        fs::read(mods.join("A Real Mod/meshes/m.nif")).unwrap(),
        b"keep"
    );
    assert!(mods.join("Group_separator").is_dir());
    assert!(
        mods.join(".git").is_dir(),
        "a dotfile that is not ours is not ours to delete"
    );
    assert!(
        app.status.as_deref().unwrap_or("").contains('2'),
        "{:?}",
        app.status
    );
    assert!(
        app.status.as_deref().unwrap_or("").contains("in use"),
        "the skip must be said, not silent: {:?}",
        app.status
    );
    let _ = fs::remove_dir_all(&root);
}
