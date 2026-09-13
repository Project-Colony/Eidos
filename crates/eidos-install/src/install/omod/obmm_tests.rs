use super::*;

fn members() -> Vec<OmodMember> {
    [
        ("A.esp", OmodFileKind::Plugin),
        ("Option/a.txt", OmodFileKind::Data),
        ("Option/sub/b.txt", OmodFileKind::Data),
        ("preview.png", OmodFileKind::Data),
    ]
    .into_iter()
    .map(|(path, kind)| OmodMember {
        path: path.into(),
        kind,
        crc32: 0,
        size: 8,
    })
    .collect()
}
fn run(source: &str, answers: &[ObmmRecordedAnswer]) -> Result<ObmmEvaluation, ObmmError> {
    ObmmProgram::parse(source)?.evaluate(
        &members(),
        &ObmmContext::default(),
        answers,
        &AtomicBool::new(false),
    )
}
fn complete(source: &str) -> ObmmPlan {
    match run(source, &[]).unwrap() {
        ObmmEvaluation::Complete(p) => p,
        x => panic!("unexpected {x:?}"),
    }
}
#[test]
fn nested_conditions_skip_effects_and_preserve_explicit_file_choices() {
    let p = complete("DontInstallAnyDataFiles\nIf Equal yes yes\nIfNot GreaterThan 1 2\nCopyDataFile Option/a.txt result.txt\nElse\nFatalError\nEndIf\nElse\nInstallDataFile missing.txt\nEndIf\nDontInstallAnyPlugins");
    assert_eq!(
        p.files,
        vec![ObmmFile {
            kind: OmodFileKind::Data,
            source: "Option/a.txt".into(),
            destination: "result.txt".into()
        }]
    );
}
#[test]
fn malformed_blocks_and_quotes_report_original_source_lines() {
    for (source, line) in [
        ("\nElse", 2),
        ("If Equal 1 1\nEndFor", 2),
        ("Message \"unclosed", 1),
        ("Label x\nLabel x", 2),
    ] {
        assert_eq!(ObmmProgram::parse(source).unwrap_err().line, line);
    }
}
#[test]
fn selection_requires_an_answer_and_replays_exact_prompt() {
    let source = "DontInstallAnyDataFiles\nSelectWithDescriptionsAndPreviews \"Pick\" \"|One\" preview.png \"first\" Two None second\nCase One\nCopyDataFile Option/a.txt chosen.txt\nBreak\nCase Two\nFatalError\nBreak\nEndSelect";
    let ObmmEvaluation::NeedPrompt(prompt) = run(source, &[]).unwrap() else {
        panic!("default installed without an answer")
    };
    let ObmmPromptKind::Select { options, many, .. } = &prompt.kind else {
        panic!()
    };
    assert!(!many);
    assert!(options[0].default);
    assert_eq!(options[0].description.as_deref(), Some("first"));
    assert_eq!(options[0].preview.as_deref(), Some("preview.png"));
    let record = ObmmRecordedAnswer {
        prompt,
        answer: ObmmAnswer::Select(vec![0]),
    };
    let result = run(source, std::slice::from_ref(&record)).unwrap();
    assert_eq!(result, run(source, std::slice::from_ref(&record)).unwrap());
    let ObmmEvaluation::Complete(p) = result else {
        panic!()
    };
    assert!(p.files.iter().any(|f| f.destination == "chosen.txt"));
    assert!(run(&source.replace("Pick", "Changed"), &[record]).is_err());
}
#[test]
fn loop_and_goto_budgets_and_cancel_prevent_completion() {
    assert!(run("Label again\nGoto again", &[])
        .unwrap_err()
        .message
        .contains("budget"));
    assert!(run("For Count i 0 3 0\nEndFor", &[]).is_err());
    let p = ObmmProgram::parse("InstallAllDataFiles").unwrap();
    assert!(p
        .evaluate(
            &members(),
            &ObmmContext::default(),
            &[],
            &AtomicBool::new(true)
        )
        .unwrap_err()
        .message
        .contains("cancel"));
}
#[test]
fn unsafe_missing_and_dynamic_commands_fail_without_a_plan() {
    for script in [
        "CopyDataFile Option/a.txt ../escape",
        "InstallDataFile missing.txt",
        "ExecLine \"Return\"",
        "SystemRun powershell",
    ] {
        assert!(run(script, &[]).is_err(), "{script}");
    }
}

fn message(source: &str, context: &ObmmContext) -> String {
    let script = ObmmProgram::parse(source).unwrap();
    let result = script
        .evaluate(&members(), context, &[], &AtomicBool::new(false))
        .unwrap();
    let ObmmEvaluation::NeedPrompt(ObmmPrompt {
        kind: ObmmPromptKind::Message { message, .. },
        ..
    }) = result
    else {
        panic!("{result:?}")
    };
    message
}
#[test]
fn for_each_count_continue_exit_and_goto_keep_current_variable_values() {
    assert_eq!(message("SetVar result x\nFor Count i 1 6 2\nSetVar result %result%%i%\nEndFor\nMessage %result%",&ObmmContext::default()),"x135");
    assert_eq!(message("SetVar result x\nFor Each DataFile f Option True *.txt\nGetFileName n %f%\nSetVar result %result%-%n%\nEndFor\nMessage %result%",&ObmmContext::default()),"x-a.txt-b.txt");
    assert_eq!(message("SetVar n 0\nFor Count i 1 5\nIf Equal %i% 2\nContinue\nEndIf\nIf Equal %i% 4\nExit\nEndIf\niSet n %n% + %i%\nEndFor\nGoto done\nFatalError\nLabel done\nMessage %n%",&ObmmContext::default()),"4");
}
#[test]
fn folder_copy_recursion_and_destinations_use_original_archive_sources() {
    let p=complete("DontInstallAnyPlugins\nDontInstallAnyDataFiles\nCopyDataFolder Option copied False\nCopyDataFile Option/sub/b.txt copied/a.txt\nInstallDataFolder Option True\nDontInstallDataFolder Option False");
    let paths: Vec<_> = p
        .files
        .iter()
        .map(|f| (f.source.as_str(), f.destination.as_str()))
        .collect();
    assert_eq!(
        paths,
        vec![
            ("Option/sub/b.txt", "copied/a.txt"),
            ("Option/sub/b.txt", "Option/sub/b.txt")
        ]
    );
}
#[test]
fn string_path_expression_and_context_reads_feed_following_instructions() {
    let mut context = ObmmContext::default();
    context.ini.insert(
        "General".into(),
        BTreeMap::from([("Name".into(), "Player".into())]),
    );
    context.renderer.insert("Vendor".into(), "Example".into());
    let source="AllowRunOnLines\nReadINI name general name\nReadRendererInfo vendor vendor\nCombinePaths p Folder file.txt\nGetDirectoryName dir %p%\nGetFileNameWithoutExtension base %p%\nSubstring short abcdef 1 3\nRemoveString remain abcdef 1 3\nStringLength len %short%\niSet x ( 2 + 3 ) * 4\nfSet y 3 / 2\nMessage \"%name% %vendor% %dir% %base% \\\n%short% %remain% %len% %x% %y%\"";
    assert_eq!(
        message(source, &context),
        "Player Example Folder file bcd aef 3 20 1.5"
    );
    for source in [
        "iSet x 2147483647 + 1",
        "iSet x 1 / 0",
        "fSet x ln -1",
        "iSet x ( 1 + 2",
        "Substring a test 5",
    ] {
        assert!(run(source, &[]).is_err(), "{source}");
    }
}
#[test]
fn actual_context_predicates_and_unknown_versions_do_not_guess() {
    let mut c = ObmmContext::default();
    c.files.insert("meshes/winner.nif".into());
    c.plugins.insert("A.esp".into(), true);
    c.active_mods.insert("TestMod".into());
    c.versions.insert("Oblivion".into(), "1.2.416.0".into());
    c.script_extender_present = true;
    let script="If DataFileExists MESHES/winner.nif\nIf PluginActive a.ESP\nIf ActiveMod testmod\nIf OblivionNewerThan 1.2.0.0\nIf ScriptExtenderPresent\nMessage yes\nEndIf\nEndIf\nEndIf\nEndIf\nEndIf";
    assert_eq!(message(script, &c), "yes");
    assert!(run("If OblivionNewerThan 1.2\nReturn\nEndIf", &[])
        .unwrap_err()
        .message
        .contains("version evidence"));
}
#[test]
fn effects_remain_ordered_unapplied_proposals_and_validate_their_operands() {
    let p=complete("EditINI General Name Player\nEditXMLLine Option/a.txt 0 value\nEditXMLReplace Option/a.txt find replace\nSetPluginByte A.esp 2 255\nSetPluginShort A.esp 3 -10\nSetPluginInt A.esp 4 100\nSetPluginLong A.esp 8 1000\nSetPluginFloat A.esp 12 1.5\nSetGMST A.esp sName value\nSetGlobal A.esp global 2\nEditShader 1 Shader Option/a.txt\nRegisterBSA archive.bsa\nUnregisterBSA old.bsa\nUncheckESP A.esp\nLoadEarly A.esp\nLoadBefore A.esp Existing.esp\nLoadAfter A.esp Other.esp\nSetDeactivationWarning A.esp WarnAgainst\nConflictsWith OtherMod reason Major\nDependsOn BaseMod\nPatchDataFile Option/a.txt patched.txt True");
    assert_eq!(p.effects.len(), 21);
    assert_eq!(p.warnings.len(), 21);
    assert_eq!(p.effects[0].command, "EditINI");
    assert_eq!(p.effects[20].command, "PatchDataFile");
    for source in [
        "SetPluginByte A.esp 0 256",
        "SetPluginInt A.esp -1 4",
        "EditShader 256 shader Option/a.txt",
        "PatchDataFile absent x True",
        "EditXMLLine ../x 1 text",
        "SetDeactivationWarning A.esp Random",
    ] {
        assert!(run(source, &[]).is_err(), "{source}");
    }
}
#[test]
fn text_input_messages_previews_yesno_and_cancellation_replay() {
    let source="InputString name \"Your name\" Player\nIf DialogYesNo \"Continue %name%?\" Title\nDisplayImage preview.png Image\nDisplayText Option/a.txt Text\nMessage done\nElse\nFatalError\nEndIf";
    let mut records = vec![];
    for answer in [
        ObmmAnswer::Text("Ada".into()),
        ObmmAnswer::YesNo(true),
        ObmmAnswer::Acknowledge,
        ObmmAnswer::Acknowledge,
        ObmmAnswer::Acknowledge,
    ] {
        let ObmmEvaluation::NeedPrompt(prompt) = run(source, &records).unwrap() else {
            panic!()
        };
        let canceled = ObmmRecordedAnswer {
            prompt: prompt.clone(),
            answer: ObmmAnswer::Cancel,
        };
        let mut aborted = records.clone();
        aborted.push(canceled);
        assert!(run(source, &aborted).is_err());
        records.push(ObmmRecordedAnswer { prompt, answer });
    }
    assert!(matches!(
        run(source, &records).unwrap(),
        ObmmEvaluation::Complete(_)
    ));
}
#[test]
fn each_selection_variant_retains_options_and_many_runs_only_selected_cases() {
    for (op, args) in [
        ("Select", "One Two"),
        ("SelectMany", "One Two"),
        ("SelectWithPreview", "One preview.png Two None"),
        ("SelectManyWithPreview", "One preview.png Two None"),
        ("SelectWithDescriptions", "One first Two second"),
        ("SelectManyWithDescriptions", "One first Two second"),
        (
            "SelectWithDescriptionsAndPreviews",
            "One preview.png first Two None second",
        ),
        (
            "SelectManyWithDescriptionsAndPreviews",
            "One preview.png first Two None second",
        ),
    ] {
        let source=format!("DontInstallAnyDataFiles\n{op} Title {args}\nCase One\nInstallDataFile Option/a.txt\nBreak\nCase Two\nInstallDataFile Option/sub/b.txt\nBreak\nDefault\nFatalError\nBreak\nEndSelect");
        let ObmmEvaluation::NeedPrompt(prompt) = run(&source, &[]).unwrap() else {
            panic!()
        };
        let indices = if op.starts_with("SelectMany") {
            vec![0, 1]
        } else {
            vec![1]
        };
        let ObmmEvaluation::Complete(p) = run(
            &source,
            &[ObmmRecordedAnswer {
                prompt,
                answer: ObmmAnswer::Select(indices),
            }],
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(
            p.files
                .iter()
                .filter(|f| f.kind == OmodFileKind::Data)
                .count(),
            if op.starts_with("SelectMany") { 2 } else { 1 }
        );
    }
    assert_eq!(message("SetVar v missing\nSelectVar v\nCase yes\nFatalError\nBreak\nDefault\nMessage fallback\nBreak\nEndSelect",&ObmmContext::default()),"fallback");
    assert_eq!(
        message(
            "SelectString x\nCase x\nMessage selected\nBreak\nEndSelect",
            &ObmmContext::default()
        ),
        "selected"
    );
}

#[test]
fn final_file_plan_rejects_file_directory_collisions_and_pins_error_line() {
    let error=run("DontInstallAnyDataFiles\nCopyDataFile Option/a.txt target\nCopyDataFile Option/sub/b.txt target/child\nReturn",&[]).unwrap_err();
    assert!(error.message.contains("ancestor"));
    let error = run(
        "Comment line one\nCopyDataFile Option/a.txt ../escape\nReturn",
        &[],
    )
    .unwrap_err();
    assert_eq!(error.line, 2);
}
#[test]
fn unicode_context_and_answer_transport_roundtrip_and_replay_rejects_extras() {
    let mut context = ObmmContext::default();
    context.ini.insert(
        "General".into(),
        BTreeMap::from([("Name".into(), "Éowyn".into())]),
    );
    let bytes = serde_json::to_vec(&context).unwrap();
    let restored: ObmmContext = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        message("ReadINI name General Name\nMessage %name%", &restored),
        "Éowyn"
    );
    let source = "Message hello";
    let ObmmEvaluation::NeedPrompt(prompt) = run(source, &[]).unwrap() else {
        panic!()
    };
    let records = vec![ObmmRecordedAnswer {
        prompt,
        answer: ObmmAnswer::Acknowledge,
    }];
    let replay: Vec<ObmmRecordedAnswer> =
        serde_json::from_slice(&serde_json::to_vec(&records).unwrap()).unwrap();
    assert!(matches!(
        run(source, &replay).unwrap(),
        ObmmEvaluation::Complete(_)
    ));
    let mut extra = replay.clone();
    extra.extend(replay);
    assert!(run(source, &extra).is_err());
    assert_eq!(
        message("StringLength n \"😀x\"\nMessage %n%", &context),
        "3"
    );
    assert!(run("Substring v \"😀x\" 1 1", &[]).is_err());
}
#[test]
fn nesting_expression_tokens_variable_growth_and_case_identity_are_bounded() {
    let deep = format!(
        "{}Return\n{}",
        "If Equal yes yes\n".repeat(65),
        "EndIf\n".repeat(65)
    );
    assert!(ObmmProgram::parse(&deep).is_err());
    let expression = format!("iSet result {}1 {}", "( ".repeat(65), ") ".repeat(65));
    assert!(run(&expression, &[]).is_err());
    assert!(
        run("SetVar s x\nLabel again\nSetVar s %s%%s%\nGoto again", &[])
            .unwrap_err()
            .message
            .contains("bound")
    );
    assert!(run(
        "Select Title same same\nCase same\nReturn\nBreak\nEndSelect",
        &[]
    )
    .is_err());
    let mut context = ObmmContext::default();
    context
        .renderer
        .insert("large".into(), "x".repeat(MAX_STRING + 1));
    assert!(ObmmProgram::parse("ReadRendererInfo result large")
        .unwrap()
        .evaluate(&members(), &context, &[], &AtomicBool::new(false))
        .is_err());
}
#[test]
fn expression_operator_families_and_obmm_precedence_are_preserved() {
    for (expression, want) in [
        ("not 0", "-1"),
        ("7 and 3", "3"),
        ("4 or 2", "6"),
        ("7 xor 3", "4"),
        ("10 mod 3", "1"),
        ("10 % 3", "1"),
        ("2 ^ 3", "8"),
        ("8 / 3", "2"),
        ("5 * 2 + 1", "11"),
        ("8 - 2 + 1", "5"),
    ] {
        assert_eq!(
            message(
                &format!("iSet n {expression}\nMessage %n%"),
                &ObmmContext::default()
            ),
            want,
            "{expression}"
        );
    }
    for expression in ["sin 0", "tan 0", "sinh 0", "tanh 0", "log 1", "ln 1"] {
        assert_eq!(
            message(
                &format!("fSet n {expression}\nMessage %n%"),
                &ObmmContext::default()
            ),
            "0"
        );
    }
    for expression in ["cos 0", "cosh 0", "exp 0"] {
        assert_eq!(
            message(
                &format!("fSet n {expression}\nMessage %n%"),
                &ObmmContext::default()
            ),
            "1"
        );
    }
}

#[test]
fn repeated_large_expansions_consume_the_shared_work_budget() {
    let source = format!(
        "SetVar payload {}\nFor Count i 1 40\nEditINI General key%i% %payload%\nEndFor",
        "x".repeat(60_000)
    );
    assert!(run(&source, &[])
        .err()
        .is_some_and(|e| e.to_string().contains("work budget")));
}
