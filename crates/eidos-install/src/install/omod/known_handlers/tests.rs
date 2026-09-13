use super::*;
#[test]
fn recognizes_only_the_three_crc_identities() {
    for (crc, expected) in [
        (0x45738C31, KnownHandler::DarNifiedUi132),
        (0xF39B4EF8, KnownHandler::DarkUidDarn16),
        (0x9646E015, KnownHandler::HorseArmorRevamped18),
    ] {
        assert_eq!(KnownHandler::from_crc(crc), Some(expected));
    }
    assert_eq!(KnownHandler::from_crc(0), None);
    let script = OmodScript {
        kind: OmodScriptKind::CSharp,
        bytes: b"arbitrary code".to_vec(),
        body_start: 0,
    };
    assert_eq!(KnownHandler::recognize(&script), None);
}

// Golden member inventory derived from the pinned C# calls, with synthetic bytes.
const UI_INPUTS: &[&str] = &[
    "Docs/fixture.bin",
    "custom_files/KCAS_levelup_menu.xml",
    "custom_files/atmo_loading_menu.xml",
    "custom_files/classic_inventory/fixture.bin",
    "custom_files/dark_loading_menu.xml",
    "custom_files/darkuid_loading_screens/fixture.bin",
    "custom_files/empty.xml",
    "custom_files/light_system_config.xml",
    "custom_files/trollf_dark_loading_menu.xml",
    "custom_files/trollf_loading_menu.xml",
    "fonts/fixture.bin",
    "installfiles/ui/DUI_Install1bg.jpg",
    "installfiles/ui/abort1.png",
    "installfiles/ui/abort2.png",
    "installfiles/ui/dui_logo.png",
    "installfiles/ui/everything1.png",
    "installfiles/ui/everything2.png",
    "installfiles/ui/select1.png",
    "installfiles/ui/select2.png",
    "menus/book_menu.xml",
    "menus/breath_meter_menu.xml",
    "menus/chargen/fixture.bin",
    "menus/container_menu.xml",
    "menus/dialog/alchemy.xml",
    "menus/dialog/dialog_menu.xml",
    "menus/dialog/enchantment.xml",
    "menus/dialog/enchantmentsetting_menu.xml",
    "menus/dialog/persuasion_menu.xml",
    "menus/dialog/sigilstone.xml",
    "menus/dialog/spell_purchase.xml",
    "menus/dialog/spellmaking.xml",
    "menus/dialog/texteditmenu.xml",
    "menus/fixture.bin",
    "menus/generic/quest_added.xml",
    "menus/generic/skill_perk.xml",
    "menus/levelup_menu.xml",
    "menus/loading_menu.xml",
    "menus/lockpick_menu.xml",
    "menus/main/hud_info_menu.xml",
    "menus/main/hud_main_menu.xml",
    "menus/main/hud_reticle.xml",
    "menus/main/hud_subtitle_menu.xml",
    "menus/main/inventory_menu.xml",
    "menus/main/magic_menu.xml",
    "menus/main/magic_popup_menu.xml",
    "menus/main/map_menu.xml",
    "menus/main/quickkeys_menu.xml",
    "menus/main/stats_menu.xml",
    "menus/message_menu.xml",
    "menus/negotiate_menu.xml",
    "menus/options/credits_menu.xml",
    "menus/options/fixture.bin",
    "menus/options/main_menu.xml",
    "menus/prefabs/darn/fill_bar.xml",
    "menus/prefabs/darn/fixture.bin",
    "menus/quantity_menu.xml",
    "menus/recharge_menu.xml",
    "menus/repair_menu.xml",
    "menus/sleep_wait_menu.xml",
    "menus/training_menu.xml",
    "meshes/Menus/darn/fixture.bin",
    "meshes/fixture.bin",
    "textures/darkui/menus/alchemy/fixture.bin",
    "textures/darkui/menus/armorrepair/fixture.bin",
    "textures/darkui/menus/book/fixture.bin",
    "textures/darkui/menus/container/fixture.bin",
    "textures/darkui/menus/dialog/fixture.bin",
    "textures/darkui/menus/enchanting/fixture.bin",
    "textures/darkui/menus/focus/fixture.bin",
    "textures/darkui/menus/genericbackground/fixture.bin",
    "textures/darkui/menus/hud/fixture.bin",
    "textures/darkui/menus/icons/fixture.bin",
    "textures/darkui/menus/inventory/fixture.bin",
    "textures/darkui/menus/loading/loading_save_center_folddui.dds",
    "textures/darkui/menus/loading/loading_save_linesdui.dds",
    "textures/darkui/menus/loading/loading_save_wide_framedui.dds",
    "textures/darkui/menus/magic/fixture.bin",
    "textures/darkui/menus/map/fixture.bin",
    "textures/darkui/menus/recharge/fixture.bin",
    "textures/darkui/menus/shared/fixture.bin",
    "textures/darkui/menus/spellmaking/fixture.bin",
    "textures/darkui/menus/stats/fixture.bin",
    "textures/fixture.bin",
    "textures/menus/darn/fixture.bin",
    "textures/menus/stats/fixture.bin",
    "textures/menus50/stats/fixture.bin",
    "textures/menus80/stats/fixture.bin",
];
const MENU_GOLDEN: &[(&str, &[&str])] = &[
    ("Breathmeter", &["menus/breath_meter_menu.xml"]),
    ("Info Menu", &["menus/main/hud_info_menu.xml"]),
    ("Subtitles", &["menus/main/hud_subtitle_menu.xml"]),
    (
        "Inventory",
        &[
            "menus/main/inventory_menu.xml",
            "menus/main/magic_popup_menu.xml",
        ],
    ),
    ("Dialog Menu", &["menus/dialog/dialog_menu.xml"]),
    ("Magic Menu", &["menus/main/magic_menu.xml"]),
    ("Map Menu", &["menus/main/map_menu.xml"]),
    ("Spell Purchase Menu", &["menus/dialog/spell_purchase.xml"]),
    ("Container Menu", &["menus/container_menu.xml"]),
    ("Repair Menu", &["menus/repair_menu.xml"]),
    ("Alchemy Menu", &["menus/dialog/alchemy.xml"]),
    ("Persuasion Menu", &["menus/dialog/persuasion_menu.xml"]),
    ("Lockpick Menu", &["menus/lockpick_menu.xml"]),
    ("Recharge Menu", &["menus/recharge_menu.xml"]),
    ("Training Menu", &["menus/training_menu.xml"]),
    ("Spellmaking Menu", &["menus/dialog/spellmaking.xml"]),
    ("Enchantment Menu", &["menus/dialog/enchantment.xml"]),
    ("System Menus", &["menus/options/fixture.bin"]),
    ("Quest Added Menu", &["menus/generic/quest_added.xml"]),
    (
        "Barter Pack",
        &["menus/negotiate_menu.xml", "menus/quantity_menu.xml"],
    ),
    ("SleepWait Menu", &["menus/sleep_wait_menu.xml"]),
    ("LevelUp Menu", &["menus/levelup_menu.xml"]),
    ("Chargen Pack", &["menus/chargen/fixture.bin"]),
    ("TextEdit Menu", &["menus/dialog/texteditmenu.xml"]),
    ("Sigilstone Menu", &["menus/dialog/sigilstone.xml"]),
    ("Skill Perk Menu", &["menus/generic/skill_perk.xml"]),
    (
        "Enchantment Setting Menu",
        &["menus/dialog/enchantmentsetting_menu.xml"],
    ),
    ("Message Menu", &["menus/message_menu.xml"]),
    ("Loading Menu", &["menus/loading_menu.xml"]),
];
fn member(path: &str, kind: OmodFileKind, bytes: &[u8]) -> OmodMember {
    OmodMember {
        path: path.into(),
        kind,
        size: bytes.len() as u64,
        crc32: crc32fast::hash(bytes),
    }
}
fn context() -> ObmmContext {
    ObmmContext {
        versions: BTreeMap::from([("OBMM".into(), "1.1.12.0".into())]),
        ..Default::default()
    }
}
fn ui_members() -> Vec<OmodMember> {
    let mut paths: BTreeSet<String> = UI_INPUTS.iter().map(|s| s.to_string()).collect();
    for f in ui::FONTS.iter().skip(1) {
        for ext in ["fnt", "tex"] {
            paths.insert(format!("custom_files/fonts/DarN_{f}.{ext}"));
        }
    }
    paths.insert("menus/prefabs/darn/stats_config.xml".into());
    paths
        .into_iter()
        .map(|p| member(&p, OmodFileKind::Data, p.as_bytes()))
        .collect()
}
fn play(
    handler: KnownHandler,
    members: &[OmodMember],
    ctx: &ObmmContext,
    mut answer: impl FnMut(&ObmmPrompt) -> ObmmAnswer,
) -> E<(KnownPlan, Vec<ObmmRecordedAnswer>)> {
    let mut answers = vec![];
    for _ in 0..64 {
        match handler.evaluate(members, ctx, &answers, &AtomicBool::new(false))? {
            KnownEvaluation::Complete(p) => return Ok((p, answers)),
            KnownEvaluation::NeedPrompt(prompt) => {
                let answer = answer(&prompt);
                answers.push(ObmmRecordedAnswer { prompt, answer });
            }
        }
    }
    panic!("unbounded prompt loop")
}
fn select_labels(p: &ObmmPrompt, labels: &[&str]) -> ObmmAnswer {
    let ObmmPromptKind::Select { options, .. } = &p.kind else {
        panic!("selection expected")
    };
    ObmmAnswer::Select(
        labels
            .iter()
            .map(|label| {
                options
                    .iter()
                    .position(|o| o.label == *label)
                    .unwrap_or_else(|| panic!("missing option {label}"))
            })
            .collect(),
    )
}
// Keep the independent wizard answers explicit in the fixture matrix.
#[allow(clippy::too_many_arguments)]
fn ui_run(
    handler: KnownHandler,
    members: &[OmodMember],
    ctx: &ObmmContext,
    menu: Option<&str>,
    options: &[&str],
    font: &str,
    size: &str,
    name: &str,
) -> E<(KnownPlan, Vec<ObmmRecordedAnswer>)> {
    play(handler, members, ctx, |p| match p.line {
        1116 => ObmmAnswer::Text(name.into()),
        269 => select_labels(
            p,
            &[if menu.is_some() {
                "Select"
            } else {
                "Everything"
            }],
        ),
        641 => select_labels(p, &[menu.unwrap()]),
        649 => select_labels(p, options),
        656 => select_labels(p, &[font]),
        660 => select_labels(p, &[size]),
        292 => ObmmAnswer::Acknowledge,
        _ => panic!("unexpected UI prompt {p:?}"),
    })
}
fn output<'a>(plan: &'a KnownPlan, path: &str) -> Option<&'a ObmmFile> {
    plan.plan
        .files
        .iter()
        .find(|f| f.destination.eq_ignore_ascii_case(path))
}
#[test]
fn all_menu_rows_have_exact_selected_payload_and_no_plugins() {
    let members = ui_members();
    assert_eq!(MENU_GOLDEN.len(), 29);
    for handler in [KnownHandler::DarNifiedUi132, KnownHandler::DarkUidDarn16] {
        for (menu, expected) in MENU_GOLDEN {
            let (p, _) = ui_run(
                handler,
                &members,
                &context(),
                Some(menu),
                &[],
                "Default",
                "Normal",
                "",
            )
            .unwrap();
            for path in *expected {
                assert!(
                    output(&p, path).is_some(),
                    "{handler:?}/{menu} missing {path}"
                );
            }
            assert!(output(&p, "menus/main/hud_reticle.xml").is_some());
            assert!(p.plan.files.iter().all(|f| f.kind == OmodFileKind::Data
                && !under(&f.destination, "installfiles")
                && !under(&f.destination, "custom_files")));
            if *menu != "Inventory" {
                assert!(output(&p, "menus/main/inventory_menu.xml").is_none());
            }
            assert!(p
                .generate(|_, _| panic!(), |_, _, _| panic!(), &AtomicBool::new(false))
                .unwrap()
                .is_empty());
        }
    }
}
#[test]
fn ui_context_filters_and_xp_exclusions_use_profile_snapshot() {
    let members = ui_members();
    let mut ctx = context();
    ctx.plugins.insert("REALISTICLEVELING.ESP".into(), true);
    ctx.files.insert("loadingscreens.esp".into());
    let (p, answers) = ui_run(
        KnownHandler::DarNifiedUi132,
        &members,
        &ctx,
        None,
        &[
            "KCAS-AF Menus",
            "Trollf Loading Screens",
            "Classic Inventory",
        ],
        "Default",
        "Large",
        "",
    )
    .unwrap();
    assert!(p
        .plan
        .effects
        .iter()
        .any(|e| e.command == "EditXMLReplace" && e.arguments[0].ends_with("stats_config.xml")));
    assert_eq!(
        output(&p, "menus/levelup_menu.xml").unwrap().source,
        "custom_files/KCAS_levelup_menu.xml"
    );
    assert_eq!(
        output(&p, "menus/loading_menu.xml").unwrap().source,
        "custom_files/trollf_loading_menu.xml"
    );
    ctx.plugins.insert("Oblivion XP.esp".into(), true);
    let (xp, xpa) = ui_run(
        KnownHandler::DarNifiedUi132,
        &members,
        &ctx,
        None,
        &[],
        "Default",
        "Normal",
        "",
    )
    .unwrap();
    for file in [
        "menus/levelup_menu.xml",
        "menus/main/stats_menu.xml",
        "menus/prefabs/darn/stats_config.xml",
    ] {
        assert!(output(&xp, file).is_none());
    }
    let opts = xpa.iter().find(|a| a.prompt.line == 649).unwrap();
    let ObmmPromptKind::Select { options, .. } = &opts.prompt.kind else {
        panic!()
    };
    assert!(!options.iter().any(|o| o.label == "KCAS-AF Menus"));
    assert!(KnownHandler::DarNifiedUi132
        .evaluate(&members, &ctx, &answers, &AtomicBool::new(false))
        .is_err());
    ctx = context();
    let (_, answers) = ui_run(
        KnownHandler::DarkUidDarn16,
        &members,
        &ctx,
        Some("Breathmeter"),
        &["Atmospheric Loading Screens"],
        "Default",
        "Normal",
        "",
    )
    .unwrap();
    let ObmmPromptKind::Select { options, .. } = &answers
        .iter()
        .find(|a| a.prompt.line == 649)
        .unwrap()
        .prompt
        .kind
    else {
        panic!()
    };
    for label in [
        "Classic Inventory",
        "Trollf Loading Screens",
        "KCAS-AF Menus",
    ] {
        assert!(!options.iter().any(|o| o.label == label));
    }
    for label in [
        "Atmospheric Loading Screens",
        "Trollf Loading Screens - DarkUI Version",
        "DarkUI'd DarN Loading Screens",
    ] {
        assert!(options.iter().any(|o| o.label == label));
    }
}
#[test]
fn every_custom_option_and_font_has_owned_files_or_explicit_effects() {
    let members = ui_members();
    let mut ctx = context();
    ctx.plugins.insert("AFLevelMod.esp".into(), true);
    ctx.files.insert("LoadingScreensSI.esp".into());
    let options = [
        "Custom Font 1",
        "KCAS-AF Menus",
        "Trollf Loading Screens",
        "Trollf Loading Screens - DarkUI Version",
        "DarkUI'd DarN Loading Screens",
        "Atmospheric Loading Screens",
        "Lighter Main Menu Text",
        "Classic Inventory",
        "Documentation",
        "Colored Local Map",
        "No Quest Added popup",
    ];
    for option in options {
        ui_run(
            KnownHandler::DarkUidDarn16,
            &members,
            &ctx,
            None,
            &[option],
            "Default",
            "Normal",
            "",
        )
        .unwrap();
    }
    let (all, _) = ui_run(
        KnownHandler::DarkUidDarn16,
        &members,
        &ctx,
        None,
        &options,
        "Default",
        "Normal",
        "",
    )
    .unwrap();
    assert_eq!(
        output(&all, "menus/loading_menu.xml").unwrap().source,
        "custom_files/atmo_loading_menu.xml"
    );
    assert_eq!(
        output(&all, "menus/generic/quest_added.xml")
            .unwrap()
            .source,
        "custom_files/empty.xml"
    );
    assert!(output(&all, "Docs/fixture.bin").is_some());
    assert!(all
        .plan
        .effects
        .iter()
        .any(|e| e.command == "EditINI" && e.arguments == ["[Display]", "bLocalMapShader", "0"]));
    for font in ui::FONTS {
        for size in ["Normal", "Large"] {
            let (p, _) = ui_run(
                KnownHandler::DarNifiedUi132,
                &members,
                &ctx,
                None,
                &["Custom Font 1"],
                font,
                size,
                "A<&",
            )
            .unwrap();
            let font1 = p
                .plan
                .effects
                .iter()
                .find(|e| e.command == "EditINI" && e.arguments[1] == "SFontFile_1");
            assert_eq!(font1.is_some(), *font != "Default");
            if let Some(e) = font1 {
                assert_eq!(e.arguments[2], format!("Data\\Fonts\\DarN_{font}.fnt"));
                assert!(output(&p, &format!("Fonts/DarN_{font}.tex")).is_some());
            }
            assert!(p.plan.effects.iter().any(|e| e.command == "EditXMLReplace"
                && e.arguments[2] == "<string>A&lt;&amp;</string>"));
            assert_eq!(
                p.plan
                    .effects
                    .iter()
                    .find(|e| e.command == "EditINI" && e.arguments[1] == "SFontFile_3")
                    .unwrap()
                    .arguments[2]
                    .contains("LG_"),
                size == "Large"
            );
        }
    }
    ctx.ini.insert(
        "Fonts".into(),
        BTreeMap::from([("SFontFile_5".into(), "Data\\Fonts\\Handwritten.fnt".into())]),
    );
    let (p, _) = ui_run(
        KnownHandler::DarNifiedUi132,
        &members,
        &ctx,
        None,
        &[],
        "Default",
        "Normal",
        "",
    )
    .unwrap();
    assert!(!p
        .plan
        .effects
        .iter()
        .any(|e| e.arguments.get(1).is_some_and(|v| v == "SFontFile_5")));
}
#[test]
fn invalid_answers_sources_versions_and_cancellation_fail_closed() {
    let members = ui_members();
    let mut ctx = context();
    let h = KnownHandler::DarNifiedUi132;
    ctx.versions.clear();
    assert!(h
        .evaluate(&members, &ctx, &[], &AtomicBool::new(false))
        .is_err());
    ctx.versions.insert("OBMM".into(), "1.1.11".into());
    assert!(h
        .evaluate(&members, &ctx, &[], &AtomicBool::new(false))
        .is_err());
    ctx = context();
    assert!(h
        .evaluate(&members, &ctx, &[], &AtomicBool::new(true))
        .is_err());
    assert!(play(h, &members, &ctx, |_| ObmmAnswer::Cancel).is_err());
    assert!(ui_run(
        h,
        &members,
        &ctx,
        None,
        &[],
        "Default",
        "Normal",
        &"X".repeat(51)
    )
    .is_err());
    let (_, mut answers) = ui_run(h, &members, &ctx, None, &[], "Default", "Normal", "").unwrap();
    answers[1].answer = ObmmAnswer::Select(vec![0, 0]);
    assert!(h
        .evaluate(&members, &ctx, &answers, &AtomicBool::new(false))
        .is_err());
    let mut bad = members.clone();
    bad.push(member("../outside", OmodFileKind::Data, b"x"));
    assert!(h
        .evaluate(&bad, &ctx, &[], &AtomicBool::new(false))
        .is_err());
    let mut bad = members.clone();
    bad.push(members[0].clone());
    assert!(h
        .evaluate(&bad, &ctx, &[], &AtomicBool::new(false))
        .is_err());
}
fn nif(base: &str) -> Vec<u8> {
    let (length, kind) = match base {
        "bridleelven" => (0xBDE9, "Elven"),
        "bridlesteel" => (0xD141, "Steel"),
        "armorelven" => (0x9C64, "Elven"),
        "armorsteel" => (0x11CDB, "Steel"),
        _ => panic!(),
    };
    let mut bytes = vec![0xAA; 0x12000];
    let h = b"Gamebryo File Format, Version 20.0.0.5\n";
    bytes[..h.len()].copy_from_slice(h);
    bytes[h.len()..h.len() + 4].copy_from_slice(&0x14000005u32.to_le_bytes());
    bytes[length..length + 4].copy_from_slice(&39u32.to_le_bytes());
    let path = format!("textures\\creatures\\horse\\armor{kind}.dds");
    bytes[length + 4..length + 4 + path.len()].copy_from_slice(path.as_bytes());
    if base == "armorelven" {
        bytes[0x9BB3..0x9BB7].copy_from_slice(&11u32.to_le_bytes());
        bytes[0x9BB7..0x9BC2].copy_from_slice(b"Material #1");
    }
    bytes
}
fn horse_fixture() -> (Vec<OmodMember>, BTreeMap<String, Vec<u8>>) {
    let mut data = BTreeMap::new();
    for c in ["Knight", "King", "Green-Black", "Black-Gray"] {
        for f in [
            "armorcloth.dds",
            "armorcloth_n.dds",
            "HRMHorseArmor_cloth.dds",
        ] {
            let p = format!("textures/creatures/horse/{c}/{f}");
            data.insert(p.clone(), p.into_bytes());
        }
    }
    data.insert(
        "harlanrm/textures/original.dds".into(),
        b"original archive payload".to_vec(),
    );
    data.insert(
        "harlanrm2/retained.bin".into(),
        b"retained sibling".to_vec(),
    );
    let mut members: Vec<_> = data
        .iter()
        .map(|(p, b)| member(p, OmodFileKind::Data, b))
        .collect();
    for p in [
        "HRMHorseArmor.esp",
        "HRMHorseArmorSlofsHorsesPatch14.esp",
        "HRMHorseArmorSlofsHorsesPatch20.esp",
    ] {
        members.push(member(p, OmodFileKind::Plugin, b"plugin"));
    }
    (members, data)
}
fn horse_run(
    members: &[OmodMember],
    ctx: &ObmmContext,
    cloth: &str,
    version: &str,
) -> E<KnownPlan> {
    play(
        KnownHandler::HorseArmorRevamped18,
        members,
        ctx,
        |p| match p.line {
            43 => ObmmAnswer::Acknowledge,
            64 => select_labels(p, &[version]),
            84 => select_labels(p, &[cloth]),
            _ => panic!(),
        },
    )
    .map(|(p, _)| p)
}
#[test]
fn horse_slof_versions_cloth_and_exclusions() {
    let (members, _) = horse_fixture();
    let mut ctx = context();
    assert!(horse_run(&members, &ctx, "Knight", "Version 1.4").is_err());
    ctx.files.insert("DLCHorseArmor.bsa".into());
    for cloth in ["Knight", "King", "Green-Black", "Black-Gray"] {
        let p = horse_run(&members, &ctx, cloth, "Version 1.4").unwrap();
        assert!(output(&p, "HRMHorseArmorSlofsHorsesPatch.esp").is_none());
        assert!(output(&p, "harlanrm2/retained.bin").is_some());
        assert!(output(&p, "HRMHorseArmor.esp").is_some());
        assert!(!p
            .plan
            .files
            .iter()
            .any(|f| under(&f.destination, "harlanrm")
                || under(&f.destination, "textures/creatures/horse")));
    }
    ctx.files.insert("Slof's Horses Base.esp".into());
    ctx.files.insert("Slof's Horses Essential.esp".into());
    for version in ["Version 1.4", "Version 2.0"] {
        let p = horse_run(&members, &ctx, "Knight", version).unwrap();
        let f = output(&p, "HRMHorseArmorSlofsHorsesPatch.esp").unwrap();
        assert!(f.source.contains(if version.ends_with("1.4") {
            "Patch14"
        } else {
            "Patch20"
        }));
        assert!(p.plan.effects.iter().any(|e| e.command == "LoadBefore"
            && e.arguments == ["HRMHorseArmor.esp", "HRMHorseArmorSlofsHorsesPatch.esp"]));
    }
    ctx.files.insert("textures/as/ashorsebay1.dds".into());
    ctx.files
        .insert("textures/creatures/horse/ashorse_bay1.dds".into());
    let p = horse_run(&members, &ctx, "Knight", "Version 1.4").unwrap();
    assert!(output(&p, "HRMHorseArmorSlofsHorsesPatch.esp")
        .unwrap()
        .source
        .contains("Patch20"));
}
#[test]
fn all_four_nif_bases_and_fourteen_derivatives_are_checked() {
    let mut count = 0;
    for (base, variants) in [
        ("bridleelven", &["glass", "chainmail", "cloth"][..]),
        ("bridlesteel", &["legion", "dragon", "ebony", "daedric"]),
        ("armorelven", &["cloth", "chainmail", "glass"]),
        ("armorsteel", &["ebony", "daedric", "legion", "dragon"]),
    ] {
        let bytes = nif(base);
        for v in variants {
            let out = horse::transform(base, v, &bytes).unwrap();
            count += 1;
            let delta = if base == "armorelven" && *v == "glass" {
                -4
            } else {
                v.len() as isize - 5
            };
            assert_eq!(out.len() as isize, bytes.len() as isize + delta);
            let name = match *v {
                "glass" => b"Glass".as_slice(),
                "chainmail" => b"Chainmail",
                "cloth" => b"Cloth",
                "legion" => b"Legion",
                "dragon" => b"Dragon",
                "ebony" => b"Ebony",
                _ => b"Daedric",
            };
            assert!(out.windows(name.len()).any(|w| w == name));
            assert_eq!(&out[..100], &bytes[..100]);
            assert!(horse::transform(base, v, &bytes[..100]).is_err());
            let mut changed = bytes.clone();
            changed[0] = b'X';
            assert!(horse::transform(base, v, &changed).is_err());
        }
    }
    assert_eq!(count, 14);
}
fn temp_archive(bytes: &[u8]) -> (std::path::PathBuf, std::fs::File) {
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "eidos-known-bsa-{}-{}.bsa",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&p)
        .unwrap();
    std::io::Write::write_all(&mut f, bytes).unwrap();
    (p, f)
}
#[test]
fn generated_horse_bsa_roundtrips_every_member_and_source_bytes() {
    let (members, data) = horse_fixture();
    let mut ctx = context();
    ctx.files.insert("DLCHorseArmor.bsa".into());
    for cloth in ["Knight", "King", "Green-Black", "Black-Gray"] {
        let p = horse_run(&members, &ctx, cloth, "Version 1.4").unwrap();
        let mut reads = vec![];
        let generated = p
            .generate(
                |path, _| Ok(data[path].clone()),
                |archive, path, _| {
                    reads.push((archive.to_owned(), path.to_owned()));
                    Ok(if path.ends_with(".nif") {
                        nif(path.rsplit('/').next().unwrap().trim_end_matches(".nif"))
                    } else {
                        format!("{archive}:{path}").into_bytes()
                    })
                },
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(reads.len(), 31);
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0].destination, "HRMHorseArmor.bsa");
        assert_eq!(
            p.generated_destinations(),
            generated
                .iter()
                .map(|f| f.destination.as_str())
                .collect::<Vec<_>>()
        );
        let (path, mut file) = temp_archive(&generated[0].bytes);
        let names = eidos_conflicts::archives::read_archive_members(&path).unwrap();
        assert_eq!(names.len(), 77);
        assert!(!names.iter().any(|p| p.starts_with("harlanrm")));
        for name in &names {
            let mut bytes = vec![];
            eidos_conflicts::archives::write_archive_member(
                &mut file,
                &name.to_ascii_uppercase(),
                MAX_MEMBER,
                &mut bytes,
            )
            .unwrap();
            if name == "textures/original.dds" {
                assert_eq!(bytes, b"original archive payload");
            } else if let Some((a, src, _)) = horse::BSA_COPIES
                .iter()
                .find(|(_, _, outs)| outs.iter().any(|o| o.eq_ignore_ascii_case(name)))
            {
                let expected = if src.ends_with(".nif") {
                    nif(src.rsplit('/').next().unwrap().trim_end_matches(".nif"))
                } else {
                    format!("{a}:{src}").into_bytes()
                };
                assert_eq!(bytes, expected, "{name}");
            } else if name.ends_with("armorcloth.dds")
                || name.ends_with("armorcloth_n.dds")
                || name.ends_with("hrmhorsearmor_cloth.dds")
            {
                assert!(String::from_utf8(bytes).unwrap().contains(cloth));
            }
        }
        std::fs::remove_file(path).unwrap();
        assert!(p
            .generate(
                |path, _| Ok(data[path].clone()),
                |_, _, _| Err(error("missing BSA member")),
                &AtomicBool::new(false)
            )
            .is_err());
        assert!(p
            .generate(
                |_, _| Ok(vec![]),
                |_, _, _| panic!(),
                &AtomicBool::new(false)
            )
            .is_err());
    }
}
#[test]
fn bsa_writer_rejects_unsafe_aliases_and_is_deterministic() {
    let good = BTreeMap::from([
        ("textures/z.dds".into(), vec![1, 2]),
        ("meshes/a.nif".into(), vec![3]),
        ("root.txt".into(), vec![]),
    ]);
    let a = bsa::generate(&good, &AtomicBool::new(false)).unwrap();
    assert_eq!(a, bsa::generate(&good, &AtomicBool::new(false)).unwrap());
    let (path, _) = temp_archive(&a);
    assert_eq!(
        eidos_conflicts::archives::read_archive_members(&path)
            .unwrap()
            .len(),
        3
    );
    std::fs::remove_file(path).unwrap();
    for bad in [
        BTreeMap::from([("../escape".into(), vec![])]),
        BTreeMap::from([("A.dds".into(), vec![]), ("a.dds".into(), vec![])]),
    ] {
        assert!(bsa::generate(&bad, &AtomicBool::new(false)).is_err());
    }
    assert!(bsa::generate(&good, &AtomicBool::new(true)).is_err());
}

// SHA-256 goldens from an independent literal-operation extraction of H:157–264.
#[test]
fn nif_derivative_bytes_match_pinned_source_goldens() {
    use sha2::Digest;
    for (base, variant, expected) in [
        (
            "bridleelven",
            "glass",
            "1cfe676d25a4f05545cf23cf56b2f05516e89d21a0dd1f7f694a0e4c8435402b",
        ),
        (
            "bridleelven",
            "chainmail",
            "9cfb7dcfca60daf2b5031ea21aface4ed736a4fea3e476320d738fba882d7c4a",
        ),
        (
            "bridleelven",
            "cloth",
            "998e1ae7d71230aaafc2e10f303371003cd35fe3e6b5a4ed9a10116c2e63a131",
        ),
        (
            "bridlesteel",
            "legion",
            "ecb65963adeb68510ad1ef6dd832e64dd370532060cf9e335070c8e81f212a31",
        ),
        (
            "bridlesteel",
            "dragon",
            "82b04e893726bf216ba4d2e8b1e03af64a55e3b266936dcac231803df396eea5",
        ),
        (
            "bridlesteel",
            "ebony",
            "fa3c3293388cb7490af5320b68d8fd82461133292000a688ea25b21c7b89b8e2",
        ),
        (
            "bridlesteel",
            "daedric",
            "783067d03f52dbec7505c20d743d707e2f949776e534d45c64583e954496b20f",
        ),
        (
            "armorelven",
            "cloth",
            "1afe92bf2bec1d462bf171bd4176a8709d8c83b515599f2865a61a49407718c1",
        ),
        (
            "armorelven",
            "chainmail",
            "5a73e4a20b1d5b8649b2ed6b902dcce4764a207722978b0c7341cab6947c6684",
        ),
        (
            "armorelven",
            "glass",
            "ce1e568162850535f142b5dde955aba335fd8949731bd9c499d28d8372923f98",
        ),
        (
            "armorsteel",
            "ebony",
            "b3530ff1c980bb9ea3fd5703c119b22c8d2edae6071da0a1ed8f4ba6b0373f66",
        ),
        (
            "armorsteel",
            "daedric",
            "3b3720dc712e7f5c829cd59777fe1c17c4ec2f53608eaf3f761449cff3c75ee0",
        ),
        (
            "armorsteel",
            "legion",
            "f9a9567e466e310aee307b4806a9fb2d89f411823b8fd91f39d4fa3fbf7a452c",
        ),
        (
            "armorsteel",
            "dragon",
            "051cc11a4248e07a13ac4f2709d8fda9d5df87c440a97eae534b33da4fda3cef",
        ),
    ] {
        let out = horse::transform(base, variant, &nif(base)).unwrap();
        assert_eq!(
            sha2::Sha256::digest(&out)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            expected,
            "{base}/{variant}"
        );
    }
}
#[test]
fn menu_minimum_name_bound_and_nonrecursive_assets_are_preserved() {
    let mut members = ui_members();
    members.extend([
        member(
            "installfiles/previews/Inventory.png",
            OmodFileKind::Data,
            b"image",
        ),
        member(
            "installfiles/previews/Inventory.a/nested.png",
            OmodFileKind::Data,
            b"wrong",
        ),
        member(
            "installfiles/descriptions/Inventory.rtf",
            OmodFileKind::Data,
            b"{\\rtf1 info}",
        ),
    ]);
    let (p, mut answers) = ui_run(
        KnownHandler::DarNifiedUi132,
        &members,
        &context(),
        Some("Inventory"),
        &[],
        "Default",
        "Normal",
        &"X".repeat(50),
    )
    .unwrap();
    assert!(output(&p, "menus/main/inventory_menu.xml").is_some());
    let a = answers.iter_mut().find(|a| a.prompt.line == 641).unwrap();
    let ObmmPromptKind::Select {
        options,
        min_choices,
        ..
    } = &a.prompt.kind
    else {
        panic!()
    };
    assert_eq!(*min_choices, 1);
    let o = options.iter().find(|o| o.label == "Inventory").unwrap();
    assert_eq!(
        o.preview.as_deref(),
        Some("installfiles/previews/Inventory.png")
    );
    assert_eq!(
        o.description_file.as_deref(),
        Some("installfiles/descriptions/Inventory.rtf")
    );
    a.answer = ObmmAnswer::Select(vec![]);
    assert!(KnownHandler::DarNifiedUi132
        .evaluate(&members, &context(), &answers, &AtomicBool::new(false))
        .is_err());
}
#[test]
fn source_and_generated_output_ancestor_collisions_fail_before_publication() {
    let mut members = ui_members();
    members.push(member("menus", OmodFileKind::Data, b"file"));
    assert!(KnownHandler::DarNifiedUi132
        .evaluate(&members, &context(), &[], &AtomicBool::new(false))
        .is_err());
    let mut members = ui_members();
    members.push(member(
        "custom_files/classic_inventory/menus",
        OmodFileKind::Data,
        b"file",
    ));
    assert!(ui_run(
        KnownHandler::DarNifiedUi132,
        &members,
        &context(),
        None,
        &["Classic Inventory"],
        "Default",
        "Normal",
        ""
    )
    .is_err());
    let (mut members, _) = horse_fixture();
    members.push(member("HRMHorseArmor.bsa", OmodFileKind::Data, b"old"));
    let mut ctx = context();
    ctx.files.insert("DLCHorseArmor.bsa".into());
    assert!(horse_run(&members, &ctx, "Knight", "Version 1.4").is_err());
}
#[test]
fn fixed_nif_guards_reject_wrong_texture_lengths_and_material_ranges() {
    for (base, length) in [
        ("bridleelven", 0xBDE9),
        ("bridlesteel", 0xD141),
        ("armorelven", 0x9C64),
        ("armorsteel", 0x11CDB),
    ] {
        let variant = if base.ends_with("elven") {
            "glass"
        } else {
            "ebony"
        };
        let bytes = nif(base);
        for offset in [length, length + 4, length + 34] {
            let mut altered = bytes.clone();
            altered[offset] ^= 1;
            assert!(
                horse::transform(base, variant, &altered).is_err(),
                "{base}/{offset}"
            );
        }
        assert!(horse::transform(base, variant, &bytes[..length + 4 + 38]).is_err());
    }
    let mut bytes = nif("armorelven");
    bytes[0x9BB3] = 12;
    assert!(horse::transform("armorelven", "glass", &bytes).is_err());
}
#[test]
fn bsa_writer_rejects_member_ancestors() {
    let tree = BTreeMap::from([
        ("textures".into(), vec![1]),
        ("textures/a.dds".into(), vec![2]),
    ]);
    assert!(bsa::generate(&tree, &AtomicBool::new(false)).is_err());
}
