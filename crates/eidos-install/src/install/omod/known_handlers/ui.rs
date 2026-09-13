use super::*;

pub(super) const MENUS: &[(&str, &[&str], &[&str])] = &[
    ("Breathmeter", &["menus/breath_meter_menu.xml"], &[]),
    ("Info Menu", &["menus/main/hud_info_menu.xml"], &[]),
    ("Subtitles", &["menus/main/hud_subtitle_menu.xml"], &[]),
    (
        "Inventory",
        &[
            "menus/main/inventory_menu.xml",
            "menus/main/magic_popup_menu.xml",
        ],
        &[],
    ),
    ("Dialog Menu", &["menus/dialog/dialog_menu.xml"], &[]),
    ("Magic Menu", &["menus/main/magic_menu.xml"], &[]),
    ("Map Menu", &["menus/main/map_menu.xml"], &[]),
    (
        "Spell Purchase Menu",
        &["menus/dialog/spell_purchase.xml"],
        &[],
    ),
    ("Container Menu", &["menus/container_menu.xml"], &[]),
    ("Repair Menu", &["menus/repair_menu.xml"], &[]),
    ("Alchemy Menu", &["menus/dialog/alchemy.xml"], &[]),
    (
        "Persuasion Menu",
        &["menus/dialog/persuasion_menu.xml"],
        &[],
    ),
    ("Lockpick Menu", &["menus/lockpick_menu.xml"], &[]),
    ("Recharge Menu", &["menus/recharge_menu.xml"], &[]),
    ("Training Menu", &["menus/training_menu.xml"], &[]),
    ("Spellmaking Menu", &["menus/dialog/spellmaking.xml"], &[]),
    ("Enchantment Menu", &["menus/dialog/enchantment.xml"], &[]),
    ("System Menus", &[], &["menus/options"]),
    ("Quest Added Menu", &["menus/generic/quest_added.xml"], &[]),
    (
        "Barter Pack",
        &["menus/negotiate_menu.xml", "menus/quantity_menu.xml"],
        &[],
    ),
    ("SleepWait Menu", &["menus/sleep_wait_menu.xml"], &[]),
    ("LevelUp Menu", &["menus/levelup_menu.xml"], &[]),
    ("Chargen Pack", &[], &["menus/chargen"]),
    ("TextEdit Menu", &["menus/dialog/texteditmenu.xml"], &[]),
    ("Sigilstone Menu", &["menus/dialog/sigilstone.xml"], &[]),
    ("Skill Perk Menu", &["menus/generic/skill_perk.xml"], &[]),
    (
        "Enchantment Setting Menu",
        &["menus/dialog/enchantmentsetting_menu.xml"],
        &[],
    ),
    ("Message Menu", &["menus/message_menu.xml"], &[]),
    ("Loading Menu", &["menus/loading_menu.xml"], &[]),
];
const TROLLF: &[&str] = &[
    "LoadingScreens.esp",
    "LoadingScreens-OOO.esp",
    "LoadingScreensSI.esp",
    "LoadingScreensAddOn.esp",
];
const KCAS: &[&str] = &[
    "RealisticLeveling.esp",
    "Kobu's Character Advancement System.esp",
    "AFLevelMod.esp",
];
pub(super) const FONTS: &[&str] = &[
    "Default",
    "!Sketchy_Times_36",
    "Dundalk_28",
    "Endor_20",
    "FantaisieArtistique_28",
    "Immortal_28",
    "Kingthings_Exeter_28",
    "Knights_Quest_36",
    "Morris_Roman_28",
    "Ringbearer_22",
    "Roosevelt_28",
    "Walshes_36",
    "Yataghan_24",
    "Kingthings_Calligraphica_36",
    "LaBrit_28",
    "Gushing_Meadow_28",
];
const DARK_SHARED: &[&str] = &[
    "textures/menus/stats",
    "textures/menus50/stats",
    "textures/menus80/stats",
    "textures/darkui/menus/hud",
    "textures/darkui/menus/icons",
    "textures/darkui/menus/stats",
    "textures/darkui/menus/book",
    "textures/darkui/menus/focus",
    "textures/darkui/menus/genericbackground",
    "textures/darkui/menus/shared",
];
fn dark_extra(menu: &str) -> Option<&'static str> {
    Some(match menu {
        "Inventory" => "inventory",
        "Dialog Menu" => "dialog",
        "Magic Menu" => "magic",
        "Map Menu" => "map",
        "Container Menu" => "container",
        "Repair Menu" => "armorrepair",
        "Alchemy Menu" => "alchemy",
        "Recharge Menu" => "recharge",
        "Spellmaking Menu" => "spellmaking",
        "Enchantment Menu" | "Sigilstone Menu" => "enchanting",
        _ => return None,
    })
}

pub(super) fn evaluate(m: &mut Machine<'_>) -> Step<Option<HorseArmorRecipe>> {
    m.version(&[1, 1, 12, 0])?;
    let dark = m.handler == KnownHandler::DarkUidDarn16;
    let title = if dark {
        "DarkUI'd DarN 1.6"
    } else {
        "DarNified UI 1.3.2"
    };
    let name = m.ask(
        1116,
        ObmmPromptKind::Input {
            title: "What's your name? (optional)".into(),
            initial: String::new(),
            max_length: Some(50),
        },
    )?;
    let ObmmAnswer::Text(name) = name else {
        return Err(error("expected a player name").into());
    };
    if name.encode_utf16().count() > 50 || name.chars().any(char::is_control) {
        return Err(error("player name exceeds 50 UTF-16 units or contains controls").into());
    }
    if !name.is_empty() {
        m.data("menus/options/credits_menu.xml")?;
        let escaped = name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        if escaped != name {
            m.warnings.push(
                "Player-name XML metacharacters were escaped to preserve literal text.".into(),
            );
        }
        m.effect(
            1124,
            "EditXMLReplace",
            &[
                "menus/options/credits_menu.xml",
                "<string>You</string>",
                &format!("<string>{escaped}</string>"),
            ],
        );
    }
    let options = ["Everything", "Select", "Abort"]
        .iter()
        .map(|s| m.option(s, false))
        .collect();
    let install = m.select(269, title, false, 1, options)?;
    if install[0] == "Abort" {
        return Err(error("cancelled").into());
    }
    let xp = m.active(&["Oblivion XP.esp"]);
    let everything = install[0] == "Everything";
    let mut selected = Vec::new();
    if everything {
        for p in ["textures", "menus", "meshes"] {
            m.tree(p, p)?;
        }
    } else {
        let choices = MENUS
            .iter()
            .filter(|(n, _, _)| !xp || *n != "LevelUp Menu")
            .map(|(n, _, _)| m.option(n, false))
            .collect();
        selected = m.select(641, "Choose menus", true, 1, choices)?;
        for p in [
            "menus/prefabs/darn",
            "textures/menus/darn",
            "meshes/Menus/darn",
        ] {
            m.tree(p, p)?;
        }
        for p in [
            "menus/main/quickkeys_menu.xml",
            "menus/options/credits_menu.xml",
            "menus/main/hud_main_menu.xml",
            "menus/main/hud_reticle.xml",
            "menus/book_menu.xml",
        ] {
            m.data(p)?;
        }
        if !xp {
            m.data("menus/main/stats_menu.xml")?;
        }
        if dark {
            for p in DARK_SHARED {
                m.tree(p, p)?;
            }
            m.data("menus/prefabs/darn/fill_bar.xml")?;
            for p in [
                "loading_save_center_folddui.dds",
                "loading_save_wide_framedui.dds",
                "loading_save_linesdui.dds",
            ] {
                m.data(&format!("textures/darkui/menus/loading/{p}"))?;
            }
        }
        for name in &selected {
            let (_, files, folders) = MENUS
                .iter()
                .find(|(n, _, _)| n == name)
                .expect("validated menu");
            for p in *files {
                m.data(p)?;
            }
            for p in *folders {
                m.tree(p, p)?;
            }
            if dark {
                if let Some(p) = dark_extra(name) {
                    let p = format!("textures/darkui/menus/{p}");
                    m.tree(&p, &p)?;
                }
            }
        }
    }
    if xp {
        for p in [
            "menus/main/stats_menu.xml",
            "menus/prefabs/darn/stats_config.xml",
            "menus/levelup_menu.xml",
        ] {
            m.remove(p);
        }
    }
    let flag = |name: &str| everything || selected.iter().any(|s| s == name);
    let (inventory, loading, levelup) = (
        flag("Inventory"),
        flag("Loading Menu"),
        flag("LevelUp Menu"),
    );
    let mut names = vec!["Custom Font 1"];
    if m.active(KCAS) && !xp {
        names.push("KCAS-AF Menus");
    }
    if loading && TROLLF.iter().any(|p| m.exists(p)) {
        names.push("Trollf Loading Screens");
    }
    if dark {
        names.extend([
            "Trollf Loading Screens - DarkUI Version",
            "DarkUI'd DarN Loading Screens",
            "Atmospheric Loading Screens",
            "Lighter Main Menu Text",
        ]);
    }
    if inventory {
        names.push("Classic Inventory");
    }
    names.extend(["Documentation", "Colored Local Map", "No Quest Added popup"]);
    let choices = names.iter().map(|s| m.option(s, false)).collect();
    let options = m.select(649, "Choose custom options", true, 0, choices)?;
    for name in &options {
        match name.as_str() {
            "Custom Font 1" => {}
            "KCAS-AF Menus" => {
                m.data("menus/prefabs/darn/stats_config.xml")?;
                m.effect(
                    915,
                    "EditXMLReplace",
                    &[
                        "menus/prefabs/darn/stats_config.xml",
                        "<_KCAS> &false; </_KCAS>",
                        "<_KCAS> &true; </_KCAS>",
                    ],
                );
                if levelup {
                    m.copy(
                        "custom_files/KCAS_levelup_menu.xml",
                        "menus/levelup_menu.xml",
                        OmodFileKind::Data,
                    )?;
                }
            }
            "Trollf Loading Screens" => m.copy(
                "custom_files/trollf_loading_menu.xml",
                "menus/loading_menu.xml",
                OmodFileKind::Data,
            )?,
            "Trollf Loading Screens - DarkUI Version" => m.copy(
                "custom_files/trollf_dark_loading_menu.xml",
                "menus/loading_menu.xml",
                OmodFileKind::Data,
            )?,
            "DarkUI'd DarN Loading Screens" => {
                m.copy(
                    "custom_files/dark_loading_menu.xml",
                    "menus/loading_menu.xml",
                    OmodFileKind::Data,
                )?;
                m.tree("custom_files/darkuid_loading_screens", "")?;
            }
            "Atmospheric Loading Screens" => m.copy(
                "custom_files/atmo_loading_menu.xml",
                "menus/loading_menu.xml",
                OmodFileKind::Data,
            )?,
            "Lighter Main Menu Text" => m.copy(
                "custom_files/light_system_config.xml",
                "menus/prefabs/darn/system_config.xml",
                OmodFileKind::Data,
            )?,
            "Classic Inventory" => m.tree("custom_files/classic_inventory", "")?,
            "Documentation" => m.tree("Docs", "Docs")?,
            "Colored Local Map" => m.effect(927, "EditINI", &["[Display]", "bLocalMapShader", "0"]),
            "No Quest Added popup" => m.copy(
                "custom_files/empty.xml",
                "menus/generic/quest_added.xml",
                OmodFileKind::Data,
            )?,
            _ => unreachable!("validated option"),
        }
    }
    if options.iter().any(|s| s == "Custom Font 1") {
        let choices = FONTS
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut o = m.option(s, i == 0);
                o.preview = m
                    .source(
                        "installfiles/previews/custom font 1.png",
                        OmodFileKind::Data,
                    )
                    .ok()
                    .map(|m| m.path.replace('\\', "/"));
                o.description_file = m
                    .source(
                        "installfiles/descriptions/custom font 1_2.rtf",
                        OmodFileKind::Data,
                    )
                    .ok()
                    .map(|m| m.path.replace('\\', "/"));
                o
            })
            .collect();
        let choice = m.select(656, "Choose font 1", false, 1, choices)?;
        if choice[0] != "Default" {
            for ext in ["fnt", "tex"] {
                m.copy(
                    &format!("custom_files/fonts/DarN_{}.{ext}", choice[0]),
                    &format!("Fonts/DarN_{}.{ext}", choice[0]),
                    OmodFileKind::Data,
                )?;
            }
            m.effect(
                969,
                "EditINI",
                &[
                    "[Fonts]",
                    "SFontFile_1",
                    &format!("Data\\Fonts\\DarN_{}.fnt", choice[0]),
                ],
            );
        }
    }
    let choices = ["Normal", "Large"]
        .iter()
        .enumerate()
        .map(|(i, s)| m.option(s, i == 0))
        .collect();
    let size = m.select(660, "Choose font size", false, 1, choices)?;
    m.tree("fonts", "fonts")?;
    for (key, value) in if size[0] == "Large" {
        [
            ("SFontFile_2", "DarN_LG_Kingthings_Petrock_14.fnt"),
            ("SFontFile_3", "DarN_LG_Kingthings_Petrock_18.fnt"),
        ]
    } else {
        [
            ("SFontFile_2", "DarN_Kingthings_Petrock_14.fnt"),
            ("SFontFile_3", "DarN_Kingthings_Petrock_16.fnt"),
        ]
    } {
        m.effect(
            939,
            "EditINI",
            &["[Fonts]", key, &format!("Data\\Fonts\\{value}")],
        );
    }
    m.effect(
        942,
        "EditINI",
        &[
            "[Fonts]",
            "SFontFile_4",
            "Data\\Fonts\\DarN_Oblivion_28.fnt",
        ],
    );
    if m.ini("Fonts", "SFontFile_5") != Some("Data\\Fonts\\Handwritten.fnt") {
        m.effect(
            960,
            "EditINI",
            &["[Fonts]", "SFontFile_5", "Data\\Fonts\\Handwritten.fnt"],
        );
    }
    m.message(
        292,
        "Modifying Oblivion.ini",
        "Review and approve the proposed Oblivion.ini changes to apply your font and menu choices.",
    )?;
    Ok(None)
}
