use super::*;

pub(super) const BSA_COPIES: &[(&str,&str,&[&str])] = &[
    ("DLCHorseArmor.bsa","textures/creatures/horse/armorelven_n.dds", &["textures/creatures/horse/armorelven_n.dds"]),
    ("DLCHorseArmor.bsa","textures/creatures/horse/armorsteel.dds", &["textures/creatures/horse/armorsteel.dds"]),
    ("DLCHorseArmor.bsa","textures/creatures/horse/armorsteel_n.dds", &["textures/creatures/horse/armorsteel_n.dds"]),
    ("DLCHorseArmor.bsa","meshes/creatures/horse/bridleelven.nif", &["meshes/creatures/horse/bridleelven.nif"]),
    ("DLCHorseArmor.bsa","meshes/creatures/horse/bridlesteel.nif", &["meshes/creatures/horse/bridlesteel.nif"]),
    ("DLCHorseArmor.bsa","meshes/creatures/horse/armorelven.nif", &["meshes/creatures/horse/armorelven.nif"]),
    ("DLCHorseArmor.bsa","meshes/creatures/horse/armorsteel.nif", &["meshes/creatures/horse/armorsteel.nif"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_01.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_01.wav"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_02.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_02.wav"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_03.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_03.wav"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_04.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_04.wav"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_05.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_05.wav"]),
    ("DLCHorseArmor.bsa","sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_06.wav", &["sound/fx/npc/horse/foot/armor/npc_horse_foot_armor_06.wav"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopic_00000cf0_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00000cf0_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A40_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A41_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A42_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A43_1.lip"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopic_00000cf0_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00000cf0_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A40_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A41_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A42_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopic_00002A43_1.mp3"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopicbuy_0000210e_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicDaedric_0000351E_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicEbony_0000C7D1_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicElven_0000C7D3_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicGlass_0000C7D2_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicSteel_0000C7D4_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicChain_00006181_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicCloth_0000A5E0_1.lip"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopicbuy_0000210e_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicDaedric_0000351E_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicEbony_0000C7D1_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicElven_0000C7D3_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicGlass_0000C7D2_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicSteel_0000C7D4_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicChain_00006181_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicCloth_0000A5E0_1.mp3"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopicelven_00002613_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EDF_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE0_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE1_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE2_1.lip"]),
    ("DLCHorseArmor.bsa","sound/voice/dlchorsearmor.esp/nord/f/dlchorsearmor_dlchorsearmortopicelven_00002613_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EDF_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE0_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE1_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicHelp_00000EE2_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterexit_000091e5_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicCancel_0000D57C_1.lip","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonCancel_0000B3A3_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterexit_000091e5_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicCancel_0000D57C_1.mp3","sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonCancel_0000B3A3_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_goodbye_0002b7ac_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicRefund_0000C150_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_goodbye_0002b7ac_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmor_HRMHorseArmorTopicRefund_0000C150_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterexit_000091ea_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragon_0000B39F_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterexit_000091ea_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragon_0000B39F_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/emfriddemo_greeting_00028a26_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragon_0000B3A0_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/emfriddemo_greeting_00028a26_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragon_0000B3A0_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterfail_0000921e_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonBuy_0000B3A5_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_barterfail_0000921e_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonBuy_0000B3A5_1.mp3"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_persuasionenter_0018b2e5_1.lip", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonBuy_0000B3A6_1.lip"]),
    ("Oblivion - Voices2.bsa","sound/voice/oblivion.esm/nord/f/generic_persuasionenter_0018b2e5_1.mp3", &["sound/Voice/HRMHorseArmor.esp/nord/f/HRMHorseArmorDragon_HRMHorseArmorTopicDragonBuy_0000B3A6_1.mp3"]),
];
const CLOTH: &[&str] = &["Knight", "King", "Green-Black", "Black-Gray"];
pub(super) fn evaluate(m: &mut Machine<'_>) -> Step<Option<HorseArmorRecipe>> {
    m.version(&[1, 1, 9])?;
    if !m.exists("DLCHorseArmor.bsa") {
        return Err(error("DLCHorseArmor.bsa is absent; this mod requires the official Bethesda Horse Armor Plugin").into());
    }
    if m.exists("Slof's Horses Essential.esp") {
        m.message(43,"Slof's Horses upgrade","Slof's Horses Essential.esp was found. Upgrade Slof's Horses to version 2.0 at www.slofshive.co.uk.")?;
    }
    if m.exists("Slof's Horses Base.esp") {
        let version = if m.exists("textures/creatures/horse/ashorse_bay1.dds") {
            "20"
        } else if m.exists("textures/as/ashorsebay1.dds") {
            "14"
        } else {
            let choices = ["Version 1.4", "Version 2.0"]
                .iter()
                .map(|s| {
                    let mut o = m.option(s, false);
                    o.description = Some(
                        "The latest version of Slof's Horses may be found at www.slofshive.co.uk"
                            .into(),
                    );
                    o
                })
                .collect();
            if m.select(
                64,
                "Which version of Slof's Horses are you running?",
                false,
                1,
                choices,
            )?[0]
                == "Version 1.4"
            {
                "14"
            } else {
                "20"
            }
        };
        m.copy(
            &format!("HRMHorseArmorSlofsHorsesPatch{version}.esp"),
            "HRMHorseArmorSlofsHorsesPatch.esp",
            OmodFileKind::Plugin,
        )?;
        m.source("HRMHorseArmor.esp", OmodFileKind::Plugin)?;
        m.effect(
            55,
            "LoadBefore",
            &["HRMHorseArmor.esp", "HRMHorseArmorSlofsHorsesPatch.esp"],
        );
    }
    let choices = CLOTH
        .iter()
        .zip([
            "For those of the highest honor",
            "For those of noble birth",
            "For the traveller",
            "For the wanderer",
        ])
        .map(|(s, desc)| {
            let mut o = m.option(s, false);
            o.description = Some(desc.into());
            o.preview = Some(format!("Textures/creatures/horse/{s}/{s}.jpg"));
            o
        })
        .collect();
    let cloth = m.select(84, "Choose a Cloth Horse Cover Type", false, 1, choices)?[0].clone();
    for file in [
        "armorcloth.dds",
        "armorcloth_n.dds",
        "HRMHorseArmor_cloth.dds",
    ] {
        m.source(
            &format!("textures/creatures/horse/{cloth}/{file}"),
            OmodFileKind::Data,
        )?;
    }
    if m.members.contains_key("hrmhorsearmor.bsa") {
        return Err(
            error("generated HRMHorseArmor.bsa would replace an existing archive member").into(),
        );
    }
    m.source("HRMHorseArmor.esp", OmodFileKind::Plugin)?;
    m.files.retain(|_, f| {
        !(f.kind == OmodFileKind::Data
            && (under(&f.destination, "harlanrm")
                || CLOTH
                    .iter()
                    .any(|s| under(&f.destination, &format!("textures/creatures/horse/{s}")))))
    });
    for p in [
        "HRMHorseArmorSlofsHorsesPatch14.esp",
        "HRMHorseArmorSlofsHorsesPatch20.esp",
    ] {
        m.source(p, OmodFileKind::Plugin)?;
        m.remove(p);
    }
    let members = m
        .members
        .iter()
        .filter(|(_, m)| m.kind == OmodFileKind::Data)
        .map(|(p, m)| (p.clone(), (*m).clone()))
        .collect();
    Ok(Some(HorseArmorRecipe { cloth, members }))
}
fn put(
    tree: &mut BTreeMap<String, Vec<u8>>,
    total: &mut usize,
    path: &str,
    bytes: Vec<u8>,
) -> E<()> {
    let path = checked_path(path)?.to_ascii_lowercase();
    if bytes.len() > MAX_MEMBER as usize {
        return Err(error("generated member exceeds 64 MiB"));
    }
    *total = total
        .checked_sub(tree.get(&path).map_or(0, Vec::len))
        .and_then(|n| n.checked_add(bytes.len()))
        .ok_or_else(|| error("generated size overflow"))?;
    if *total > MAX_GENERATED || tree.len() >= 100_000 {
        return Err(error("generated tree exceeds bounds"));
    }
    tree.insert(path, bytes);
    Ok(())
}
fn source_bytes(
    recipe: &HorseArmorRecipe,
    path: &str,
    read: &mut impl FnMut(&str, u64) -> E<Vec<u8>>,
) -> E<Vec<u8>> {
    let member = recipe
        .members
        .get(&path.replace('\\', "/").to_ascii_lowercase())
        .ok_or_else(|| error(format!("missing archive member {path}")))?;
    if member.size > MAX_MEMBER {
        return Err(error("source member exceeds 64 MiB"));
    }
    let bytes = read(&member.path, MAX_MEMBER)?;
    if bytes.len() as u64 != member.size || crc32fast::hash(&bytes) != member.crc32 {
        return Err(error(format!("source size/CRC changed: {path}")));
    }
    Ok(bytes)
}
pub(super) fn generate(
    recipe: &HorseArmorRecipe,
    read_data: &mut impl FnMut(&str, u64) -> E<Vec<u8>>,
    read_bsa: &mut impl FnMut(&str, &str, u64) -> E<Vec<u8>>,
    cancel: &AtomicBool,
) -> E<Vec<GeneratedFile>> {
    let mut tree = BTreeMap::new();
    let mut total = 0;
    for m in recipe.members.values() {
        check_cancel(cancel)?;
        let path = m.path.replace('\\', "/");
        if under(&path, "harlanrm") {
            let bytes = source_bytes(recipe, &path, read_data)?;
            put(
                &mut tree,
                &mut total,
                path.get(9..)
                    .ok_or_else(|| error("invalid harlanrm member"))?,
                bytes,
            )?;
        }
    }
    for file in [
        "armorcloth.dds",
        "armorcloth_n.dds",
        "HRMHorseArmor_cloth.dds",
    ] {
        check_cancel(cancel)?;
        let bytes = source_bytes(
            recipe,
            &format!("textures/creatures/horse/{}/{file}", recipe.cloth),
            read_data,
        )?;
        let out = if file == "HRMHorseArmor_cloth.dds" {
            format!("textures/menus/icons/armor/{file}")
        } else {
            format!("textures/creatures/horse/{file}")
        };
        put(&mut tree, &mut total, &out, bytes)?;
    }
    for (archive, path, outputs) in BSA_COPIES {
        check_cancel(cancel)?;
        let bytes = read_bsa(archive, path, MAX_MEMBER)?;
        if bytes.len() > MAX_MEMBER as usize {
            return Err(error("BSA reader exceeded requested byte bound"));
        }
        for out in *outputs {
            put(&mut tree, &mut total, out, bytes.clone())?;
        }
    }
    for (base, variants) in [
        ("bridleelven", &["glass", "chainmail", "cloth"][..]),
        ("bridlesteel", &["legion", "dragon", "ebony", "daedric"][..]),
        ("armorelven", &["cloth", "chainmail", "glass"][..]),
        ("armorsteel", &["ebony", "daedric", "legion", "dragon"][..]),
    ] {
        let bytes = tree
            .get(&format!("meshes/creatures/horse/{base}.nif"))
            .ok_or_else(|| error("missing NIF base"))?
            .clone();
        for variant in variants {
            check_cancel(cancel)?;
            let prefix = if base.starts_with("bridle") {
                "bridle"
            } else {
                "armor"
            };
            put(
                &mut tree,
                &mut total,
                &format!("meshes/creatures/horse/{prefix}{variant}.nif"),
                transform(base, variant, &bytes)?,
            )?;
        }
    }
    check_cancel(cancel)?;
    let bytes = bsa::generate(&tree, cancel)?;
    Ok(vec![GeneratedFile {
        destination: HORSE_OUTPUT.into(),
        bytes,
    }])
}

/// Fixed layouts from HorseArmorRevamped.cs:157–264. Guard the complete texture
/// string and its length, header, and all edited ranges before changing a copy.
pub(super) fn transform(base: &str, variant: &str, original: &[u8]) -> E<Vec<u8>> {
    let (length, token, kind) = match base {
        "bridleelven" => (0xBDE9, 0xBE0B, "Elven"),
        "bridlesteel" => (0xD141, 0xD163, "Steel"),
        "armorelven" => (0x9C64, 0x9C86, "Elven"),
        "armorsteel" => (0x11CDB, 0x11CFD, "Steel"),
        _ => return Err(error("unknown NIF base")),
    };
    let header = b"Gamebryo File Format, Version 20.0.0.5\n";
    let texture = format!("textures\\creatures\\horse\\armor{kind}.dds");
    if !original.starts_with(header)
        || original.get(header.len()..header.len() + 4) != Some(&0x14000005u32.to_le_bytes())
        || original.get(length..length + 4) != Some(&39u32.to_le_bytes())
        || !original
            .get(length + 4..length + 4 + texture.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(texture.as_bytes()))
    {
        return Err(error(format!(
            "unsupported {base}.nif layout; expected the fixed Oblivion texture record"
        )));
    }
    let name = match variant {
        "glass" => "Glass",
        "chainmail" => "Chainmail",
        "cloth" => "Cloth",
        "legion" => "Legion",
        "dragon" => "Dragon",
        "ebony" => "Ebony",
        "daedric" => "Daedric",
        _ => return Err(error("unknown NIF variant")),
    };
    let valid = if kind == "Elven" {
        matches!(variant, "glass" | "chainmail" | "cloth")
    } else {
        matches!(variant, "legion" | "dragon" | "ebony" | "daedric")
    };
    if !valid {
        return Err(error("variant does not belong to NIF base"));
    }
    let mut out = original.to_vec();
    out[length..length + 4].copy_from_slice(&(39u32 + name.len() as u32 - 5).to_le_bytes());
    out.splice(token..token + 5, name.bytes());
    if base == "armorelven" && variant == "glass" {
        // Check the shorter material-string edit as well; its following offsets
        // are deliberately changed only before the final four-byte shortening.
        if original.get(0x9BB3..0x9BB7) != Some(&11u32.to_le_bytes())
            || !original
                .get(0x9BB7..0x9BC2)
                .is_some_and(|s| s.iter().all(u8::is_ascii))
        {
            return Err(error("unsupported glass material record layout"));
        }
        let white = [0x80, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0x80, 0x3F];
        out[0x9BCC..0x9BD6].copy_from_slice(&white);
        out[0x9BE2..0x9BEE].fill(0);
        out[0x9BF0..0x9BFA].copy_from_slice(&white);
        out[0x9C33] = 3;
        out[0x9BB3] = 7;
        out.splice(0x9BB7..0x9BC2, b"EnvMap2".iter().copied());
    }
    Ok(out)
}
