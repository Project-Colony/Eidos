//! What the install actually did, including everything it could not do.
//!
//! The single most important design decision in this feature. The worst possible
//! outcome is not a failure - it is a collection that reports "installed" and
//! plays wrong, because patches were skipped, an ordering was half applied and a
//! FOMOD replay quietly fell back to defaults. Vortex logs that class of thing at
//! level "info" with the words "This is normal", and its user is never told.
//!
//! So: anything not applied goes in here, and this is SHOWN, not logged.

/// One thing that did not go as the collection asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// What it is about - a member's name, a rule, a file.
    pub subject: String,
    pub detail: String,
}

/// The outcome of installing a collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Persistence failed: callers must not apply any further collection changes.
    pub aborted: bool,
    pub collection: String,
    pub revision: u32,
    /// Members installed exactly as asked.
    pub installed: Vec<String>,
    /// Installed, but the author's installer answers could not all be replayed -
    /// so the FILES ON DISK differ from the collection author's. Its own list,
    /// because it looks like success and is not.
    pub approximate: Vec<Note>,
    pub skipped: Vec<Note>,
    pub failed: Vec<Note>,
    /// Members Eidos cannot fetch by itself: a free account's Nexus files, and
    /// the `browse` / `manual` sources that were never automatable.
    pub needs_you: Vec<Note>,
    /// Rules that named nothing this collection contains.
    pub rules_lost: Vec<Note>,
    /// Members caught in a cycle of ordering rules.
    pub rule_cycles: Vec<String>,
    /// Plugin rules the LOOT merge would not take, and why.
    pub loot_notes: Vec<Note>,
    /// Tools the collection expects to already exist here.
    pub tools_expected: Vec<Note>,
    /// Top-level manifest sections this build does not understand.
    pub unknown_sections: Vec<String>,
    /// Members installed under a different folder name because a mod that is
    /// not this collection's already had that one.
    pub renamed: Vec<Note>,
    /// Unverified requirements and remaining collection-wide choices, including
    /// unknown runtime evidence, an approved runtime mismatch, and INI selection.
    pub deferred: Vec<Note>,
}

impl Report {
    /// Whether the collection is on disk the way its author built it.
    ///
    /// Deliberately strict: an approximate member counts as NOT faithful, and so
    /// does a lost ordering rule. The point of the word is to be worth trusting.
    pub fn is_faithful(&self) -> bool {
        self.approximate.is_empty()
            && self.failed.is_empty()
            && self.needs_you.is_empty()
            && self.rules_lost.is_empty()
            && self.rule_cycles.is_empty()
            && self.loot_notes.is_empty()
            // A section this build has never seen is a part of the recipe that
            // was not followed, and it is the whole reason the parser records
            // them. Printing "not applied" and "as its author built it" in one
            // report makes the word worthless.
            && self.unknown_sections.is_empty()
            && self.deferred.is_empty()
    }

    /// Whether anything at all is still to do.
    pub fn is_complete(&self) -> bool {
        self.failed.is_empty() && self.needs_you.is_empty()
    }

    /// The report as a person reads it. Empty sections are omitted, so a clean
    /// install is three lines and a busy one is exactly as long as it needs to
    /// be - a report padded with "0 problems" headings is one nobody reads.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{} (revision {})\n",
            self.collection, self.revision
        ));
        out.push_str(&format!("  {} installed\n", self.installed.len()));

        let section = |out: &mut String, title: &str, notes: &[Note]| {
            if notes.is_empty() {
                return;
            }
            out.push_str(&format!("\n{title}\n"));
            for n in notes {
                out.push_str(&format!("  {}: {}\n", n.subject, n.detail));
            }
        };

        section(
            &mut out,
            "Installed, but NOT the way the collection asks - the files on disk differ:",
            &self.approximate,
        );
        section(
            &mut out,
            "You need to fetch these yourself:",
            &self.needs_you,
        );
        section(&mut out, "Failed:", &self.failed);
        section(&mut out, "Skipped:", &self.skipped);
        section(
            &mut out,
            "Ordering rules that named nothing in this collection:",
            &self.rules_lost,
        );
        if !self.rule_cycles.is_empty() {
            out.push_str(
                "\nThese mods are in a loop of ordering rules, so the collection is asking \
                 for something impossible. They keep the order it listed them in:\n",
            );
            for m in &self.rule_cycles {
                out.push_str(&format!("  {m}\n"));
            }
        }
        section(
            &mut out,
            "Installed under a different name, because a mod of yours already had it:",
            &self.renamed,
        );
        section(&mut out, "Load order:", &self.loot_notes);
        section(
            &mut out,
            "Collection differences and unverified requirements:",
            &self.deferred,
        );
        section(
            &mut out,
            "Tools this collection expects you to already have:",
            &self.tools_expected,
        );
        if !self.unknown_sections.is_empty() {
            out.push_str(&format!(
                "\nThis collection carries something this version of Eidos does not \
                 understand, so that part was not applied: {}\n",
                self.unknown_sections.join(", ")
            ));
        }
        // Skipped members are the USER's decision, so they do not make an
        // install unfaithful - `--no-optional` would otherwise brand every later
        // resumed run of that collection. They do make this sentence false,
        // though, so it is not printed beside a list of what is missing.
        if self.is_faithful() && self.skipped.is_empty() {
            out.push_str("\nThe collection is installed as its author built it.\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(s: &str, d: &str) -> Note {
        Note {
            subject: s.into(),
            detail: d.into(),
        }
    }

    #[test]
    fn a_clean_install_says_so_in_three_lines() {
        let r = Report {
            collection: "The Great Cities".into(),
            revision: 1,
            installed: vec!["A".into(), "B".into()],
            ..Report::default()
        };
        assert!(r.is_faithful() && r.is_complete());
        let text = r.render();
        assert!(text.contains("2 installed"));
        assert!(text.contains("as its author built it"));
        // Nothing else: a report padded with empty headings is one nobody reads.
        assert_eq!(text.lines().filter(|l| !l.trim().is_empty()).count(), 3);
    }

    #[test]
    fn a_member_installed_with_different_options_is_not_a_success() {
        // It looks like one, which is exactly why it has its own list. The files
        // on disk are not the author's.
        let r = Report {
            installed: vec!["A".into()],
            approximate: vec![note("A Patch", "\"8K\" is no longer an option")],
            ..Report::default()
        };
        assert!(!r.is_faithful());
        assert!(
            r.is_complete(),
            "nothing is left TO DO, it is just not faithful"
        );
        assert!(r.render().contains("files on disk differ"));
    }

    #[test]
    fn everything_not_applied_appears_somewhere() {
        let r = Report {
            collection: "X".into(),
            revision: 2,
            failed: vec![note("B", "404")],
            needs_you: vec![note("C", "a free account cannot fetch this")],
            skipped: vec![note("D", "you said no")],
            rules_lost: vec![note("A before Z", "Z is not in this collection")],
            rule_cycles: vec!["E".into(), "F".into()],
            loot_notes: vec![note("G.esp", "your own group was kept")],
            tools_expected: vec![note("xEdit", "SSEEdit.exe")],
            unknown_sections: vec!["somethingNew".into()],
            ..Report::default()
        };
        let t = r.render();
        for expected in [
            "404",
            "free account",
            "you said no",
            "Z is not in this collection",
            "impossible",
            "your own group was kept",
            "SSEEdit.exe",
            "somethingNew",
        ] {
            assert!(t.contains(expected), "missing {expected:?} from:\n{t}");
        }
        assert!(!r.is_faithful() && !r.is_complete());
    }

    #[test]
    fn work_left_to_do_and_faithfulness_are_different_questions() {
        // A collection can be finished and wrong, or unfinished and correct so
        // far. Collapsing the two is how "installed" comes to mean nothing.
        let unfinished = Report {
            needs_you: vec![note("A", "fetch it")],
            ..Report::default()
        };
        assert!(!unfinished.is_complete() && !unfinished.is_faithful());

        let finished_but_wrong = Report {
            approximate: vec![note("A", "different options")],
            ..Report::default()
        };
        assert!(finished_but_wrong.is_complete() && !finished_but_wrong.is_faithful());
    }
    #[test]
    fn a_report_never_says_both_not_applied_and_as_its_author_built_it() {
        for r in [
            Report {
                unknown_sections: vec!["somethingNew".into()],
                ..Report::default()
            },
            Report {
                deferred: vec![note("A Patch", "2 binary patch(es)")],
                ..Report::default()
            },
        ] {
            let t = r.render();
            assert!(!r.is_faithful(), "{t}");
            assert!(!t.contains("as its author built it"), "{t}");
        }
        // A member the USER skipped is their decision, not a defect: it must not
        // brand every later resumed run of that collection unfaithful. It does
        // make the closing sentence false, so that sentence is not printed.
        let skipped = Report {
            skipped: vec![note("D", "you said no")],
            ..Report::default()
        };
        assert!(skipped.is_faithful());
        assert!(!skipped.render().contains("as its author built it"));
    }
}
