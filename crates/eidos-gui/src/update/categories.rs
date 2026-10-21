//! Mod categories: the assignment dialog, the category editor and the Nexus
//! category import.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Backups dialog (MO2's Create Backup / Restore Backup) -----------
        // ---- Categories (MO2's Change Categories + the category editor) ------
        Message::ShowCategoriesDialog(row) => {
            app.menu_mod = None;
            let Some(inst) = app.created.clone() else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            // The right-clicked row, or the whole selection when it is part of
            // one (MO2's multi-row Change Categories).
            let sel = real_selection(app);
            let targets: Vec<usize> = if sel.len() > 1 && sel.contains(&row) {
                sel
            } else if app
                .mods
                .get(row)
                .is_some_and(|m| !m.is_separator() && !m.is_unmanaged())
            {
                vec![row]
            } else {
                Vec::new()
            };
            if targets.is_empty() {
                app.status = Some(t("status_no_categories_here").to_string());
                return Task::none();
            }
            let names: Vec<String> = targets
                .iter()
                .filter_map(|&i| app.mods.get(i))
                .map(|m| m.name.clone())
                .collect();
            // The starting selection is the FIRST target's, so a single mod opens
            // on what it actually has. Across a multi-selection MO2 does the same
            // and applying overwrites the rest - the dialog says so.
            let chosen = names
                .first()
                .map(|n| {
                    eidos_instance::parse_all(&inst.mod_meta(n).category().unwrap_or_default())
                })
                .unwrap_or_default();
            app.categories_dialog = Some(CategoriesDialogState {
                names,
                chosen,
                catalog: inst.category_factory(),
                editing: false,
                new_name: String::new(),
                new_parent: 0,
                rename: None,
                confirm_delete: None,
                query: String::new(),
            });
        }
        Message::CloseCategoriesDialog => app.categories_dialog = None,
        Message::ToggleCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                match d.chosen.iter().position(|&c| c == id) {
                    Some(at) => {
                        d.chosen.remove(at);
                    }
                    None => d.chosen.push(id),
                }
            }
        }
        Message::SetPrimaryCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                // Primary is simply "first in the list" on disk, so promoting is a
                // move, not a separate field. Checking it too, if it was not, keeps
                // the two controls from disagreeing.
                d.chosen.retain(|&c| c != id);
                d.chosen.insert(0, id);
            }
        }
        Message::CategoryQueryChanged(q) => {
            if let Some(d) = &mut app.categories_dialog {
                d.query = q;
            }
        }
        Message::ToggleCategoryEditor => {
            if let Some(d) = &mut app.categories_dialog {
                d.editing = !d.editing;
                d.rename = None;
                d.confirm_delete = None;
            }
        }
        Message::NewCategoryNameChanged(name) => {
            if let Some(d) = &mut app.categories_dialog {
                d.new_name = name;
            }
        }
        Message::NewCategoryParentChanged(parent) => {
            if let Some(d) = &mut app.categories_dialog {
                d.new_parent = parent;
            }
        }
        Message::AddCategory => {
            if let Some(d) = &mut app.categories_dialog {
                let name = d.new_name.trim().to_string();
                if name.is_empty() {
                    app.status = Some(t("status_category_needs_name").to_string());
                    return Task::none();
                }
                let id = d.catalog.add(&name, d.new_parent);
                d.new_name.clear();
                app.status = Some(tf("status_category_added", &[("name", &name), ("id", &id)]));
            }
        }
        Message::RenameCategoryStart(id) => {
            if let Some(d) = &mut app.categories_dialog {
                let current = d.catalog.name_for_id(id).unwrap_or_default().to_string();
                d.rename = Some((id, current));
                d.confirm_delete = None;
            }
        }
        Message::RenameCategoryChanged(name) => {
            if let Some(d) = &mut app.categories_dialog {
                if let Some((_, pending)) = &mut d.rename {
                    *pending = name;
                }
            }
        }
        Message::RenameCategoryCommit => {
            if let Some(d) = &mut app.categories_dialog {
                if let Some((id, name)) = d.rename.take() {
                    if !name.trim().is_empty() {
                        d.catalog.rename(id, &name);
                    }
                }
            }
        }
        Message::DeleteCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                // Two clicks: the first arms, the second deletes.
                if d.confirm_delete == Some(id) {
                    d.catalog.remove(id);
                    d.confirm_delete = None;
                    // A deleted category cannot stay assigned in the pending pick.
                    d.chosen.retain(|&c| c != id);
                } else {
                    d.confirm_delete = Some(id);
                }
            }
        }
        Message::FetchNexusCategories => {
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(t("status_connect_nexus_first").to_string());
                return Task::none();
            }
            let Some(domain) = selected_game(app).map(|g| g.def.nexus_game.to_string()) else {
                return Task::none();
            };
            app.status = Some(t("status_fetching_categories").to_string());
            return Task::perform(
                smol::unblock(move || {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    nexus.game_categories(&domain)
                }),
                Message::NexusCategoriesFetched,
            );
        }
        Message::NexusCategoriesFetched(result) => {
            let Some(d) = &mut app.categories_dialog else {
                return Task::none();
            };
            let remote = match result {
                Ok(r) => r,
                Err(e) => {
                    app.status = Some(tf("status_fetch_categories_failed", &[("error", &e)]));
                    return Task::none();
                }
            };
            // Match a remote category onto a local one by NAME first - MO2's
            // built-in list was derived from Nexus's, so most of them already
            // line up and mapping by name avoids creating 40 duplicates named
            // exactly like the ones already there. Only the rest are created.
            let mut mapped = 0usize;
            let mut created = 0usize;
            // Two passes so a child's parent exists before it is placed, whatever
            // order Nexus returned them in.
            let mut local_of: HashMap<i32, i32> = HashMap::new();
            let mut created_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for (nexus_id, name, _) in &remote {
                let existing = d
                    .catalog
                    .all()
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .map(|c| c.id);
                let local = match existing {
                    Some(id) => id,
                    None => {
                        created += 1;
                        let id = d.catalog.add(name, 0);
                        created_ids.insert(id);
                        id
                    }
                };
                local_of.insert(*nexus_id, local);
                if d.catalog.learn_nexus_id(local, *nexus_id, name) {
                    mapped += 1;
                }
            }
            for (nexus_id, _, parent) in &remote {
                // ONLY the categories this fetch created. The previous rule was
                // "currently top-level", which is not the same thing at all: it
                // swept in every pre-existing top-level category - MO2's own
                // defaults included - and re-parented the user's shared tree on
                // every fetch.
                let (Some(&local), Some(parent)) = (local_of.get(nexus_id), *parent) else {
                    continue;
                };
                if !created_ids.contains(&local) {
                    continue;
                }
                if let Some(&local_parent) = local_of.get(&parent) {
                    d.catalog.set_parent(local, local_parent);
                }
            }
            app.status = Some(tf(
                "status_categories_fetched",
                &[
                    ("n", &remote.len()),
                    ("created", &created),
                    ("mapped", &mapped),
                ],
            ));
        }
        Message::AssignCategoriesFromNexus => {
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let Some(d) = &mut app.categories_dialog else {
                return Task::none();
            };
            if d.catalog.all().iter().all(|c| c.nexus.is_empty()) {
                app.status = Some(t("status_no_nexus_mappings").to_string());
                return Task::none();
            }
            // Only the targeted mods, so this stays the dialog's own action and
            // never rewrites the whole instance from a button the user pressed to
            // categorise one mod. Batch-select to do more.
            let mut hits: Vec<i32> = Vec::new();
            for name in &d.names {
                let Some(remote) = nexus_category_of(&inst, name) else {
                    continue;
                };
                if let Some(local) = d.catalog.for_nexus_id(remote) {
                    hits.push(local);
                }
            }
            match hits.first() {
                Some(&first) => {
                    // The dialog holds ONE pending pick for the whole selection,
                    // so a mixed batch would have to pick a winner. Say which.
                    let all_same = hits.iter().all(|&h| h == first);
                    d.chosen = vec![first];
                    let label = d.catalog.name_for_id(first).unwrap_or("?").to_string();
                    app.status = Some(if all_same {
                        tf("status_nexus_category_one", &[("name", &label)])
                    } else {
                        tf("status_nexus_category_many", &[("name", &label)])
                    });
                }
                None => {
                    app.status = Some(t("status_no_nexus_category").to_string());
                }
            }
        }
        Message::ApplyCategories => {
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let Some(d) = app.categories_dialog.as_ref() else {
                return Task::none();
            };
            // The catalog first: if it fails, nothing has been written yet, and a
            // mod pointing at an id the catalog does not carry would show a bare
            // number with no way to name it.
            //
            // The dialog is NOT taken until that save succeeds. Taking it first
            // meant a failed write - a read-only instance, a full disk - threw
            // away every pending edit with nothing to retry from.
            if let Err(e) = d.catalog.save(inst.categories_root()) {
                app.status = Some(tf("status_category_list_save_failed", &[("error", &e)]));
                return Task::none();
            }
            let Some(d) = app.categories_dialog.take() else {
                return Task::none();
            };
            let (primary, others) = match d.chosen.split_first() {
                Some((p, rest)) => (Some(*p), rest),
                None => (None, &[][..]),
            };
            let mut written = 0usize;
            let mut failed: Vec<String> = Vec::new();
            for name in &d.names {
                match edit_mod_meta(&inst, name, |meta| meta.set_categories(primary, others)) {
                    Ok(()) => written += 1,
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
            // The cache maps name -> category; every touched mod's row is stale.
            app.meta_cache.clear();
            refresh_meta_cache(app);
            app.categories = Some(inst.category_factory());
            recompute_counts(app);
            // Recategorising changes what a category filter shows, so the same
            // rule applies as to the filter pane itself: nothing may stay aimed
            // at a row that just left the screen.
            forget_hidden_rows(app);
            app.status = Some(if failed.is_empty() {
                match (primary, written) {
                    (None, n) => tf("status_category_cleared", &[("n", &n)]),
                    (Some(p), n) => {
                        let label = d.catalog.name_for_id(p).unwrap_or("?");
                        tf("status_category_set", &[("name", &label), ("n", &n)])
                    }
                }
            } else {
                tf(
                    "status_categories_partial",
                    &[
                        ("n", &written),
                        ("failed", &failed.len()),
                        ("errors", &failed.join("; ")),
                    ],
                )
            });
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
