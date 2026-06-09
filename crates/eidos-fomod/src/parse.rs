//! Decode + parse `ModuleConfig.xml` into a [`ModuleConfig`].
//!
//! FOMOD XML is routinely UTF-16 (the Nexus FOMOD Creation Tool emits it), so we
//! decode by BOM first, then walk the DOM with `roxmltree`.

use roxmltree::{Document, Node};

use crate::model::*;

/// Decode a `ModuleConfig.xml` byte buffer to text, honouring a UTF-8 / UTF-16LE /
/// UTF-16BE byte-order mark (FOMODs are commonly UTF-16).
pub fn decode_xml(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let u: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let u: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Drop a leading `<?xml ... ?>` declaration so a UTF-16-declared document parses
/// once we have already decoded it to a Rust `&str` (UTF-8).
fn strip_decl(xml: &str) -> &str {
    let t = xml.trim_start_matches('\u{feff}').trim_start();
    if let Some(rest) = t.strip_prefix("<?xml") {
        if let Some(i) = rest.find("?>") {
            return rest[i + 2..].trim_start();
        }
    }
    t
}

fn norm_path(s: &str) -> String {
    s.replace('\\', "/")
}

fn find_child<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children().find(|c| c.is_element() && c.tag_name().name() == tag)
}

fn elements<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Vec<Node<'a, 'i>> {
    node.children().filter(|c| c.is_element() && c.tag_name().name() == tag).collect()
}

impl ModuleConfig {
    /// Parse decoded `ModuleConfig.xml` text.
    pub fn parse(xml: &str) -> Result<ModuleConfig, String> {
        let doc = Document::parse(strip_decl(xml)).map_err(|e| e.to_string())?;
        let root = doc.root_element();
        if root.tag_name().name() != "config" {
            return Err(format!("unexpected root <{}>, expected <config>", root.tag_name().name()));
        }
        let mut mc = ModuleConfig::default();
        let mut seq = 0u32;
        for c in root.children().filter(|n| n.is_element()) {
            match c.tag_name().name() {
                "moduleName" => mc.module_name = c.text().unwrap_or("").trim().to_string(),
                "moduleImage" => mc.module_image = c.attribute("path").map(norm_path),
                "requiredInstallFiles" => mc.required_files = parse_file_list(c, &mut seq),
                "installSteps" => {
                    for s in elements(c, "installStep") {
                        mc.steps.push(parse_step(s, &mut seq));
                    }
                }
                "conditionalFileInstalls" => mc.conditional_installs = parse_conditional_installs(c, &mut seq),
                _ => {}
            }
        }
        Ok(mc)
    }
}

fn parse_file_list(node: Node, seq: &mut u32) -> Vec<FileItem> {
    node.children()
        .filter(|c| c.is_element() && matches!(c.tag_name().name(), "file" | "folder"))
        .map(|c| {
            let source = norm_path(c.attribute("source").unwrap_or(""));
            let destination = c.attribute("destination").map(norm_path).unwrap_or_else(|| source.clone());
            let item = FileItem {
                source,
                destination,
                priority: c.attribute("priority").and_then(|p| p.parse().ok()).unwrap_or(0),
                is_folder: c.tag_name().name() == "folder",
                always_install: c.attribute("alwaysInstall") == Some("true"),
                install_if_usable: c.attribute("installIfUsable") == Some("true"),
                sequence: *seq,
            };
            *seq += 1;
            item
        })
        .collect()
}

fn parse_step(node: Node, seq: &mut u32) -> InstallStep {
    let name = node.attribute("name").unwrap_or("").to_string();
    let visible = find_child(node, "visible").map(parse_composite);
    let mut groups = Vec::new();
    if let Some(g) = find_child(node, "optionalFileGroups") {
        for gr in elements(g, "group") {
            groups.push(parse_group(gr, seq));
        }
    }
    InstallStep { name, visible, groups }
}

fn parse_group(node: Node, seq: &mut u32) -> Group {
    let name = node.attribute("name").unwrap_or("").to_string();
    let group_type = GroupType::parse(node.attribute("type").unwrap_or(""));
    let mut plugins = Vec::new();
    if let Some(p) = find_child(node, "plugins") {
        for pl in elements(p, "plugin") {
            plugins.push(parse_plugin(pl, seq));
        }
    }
    Group { name, group_type, plugins }
}

fn parse_plugin(node: Node, seq: &mut u32) -> Plugin {
    let name = node.attribute("name").unwrap_or("").to_string();
    let description =
        find_child(node, "description").and_then(|d| d.text()).unwrap_or("").trim().to_string();
    let image = find_child(node, "image").and_then(|i| i.attribute("path")).map(norm_path);
    let condition_flags = find_child(node, "conditionFlags")
        .map(|cf| {
            elements(cf, "flag")
                .into_iter()
                .map(|f| (f.attribute("name").unwrap_or("").to_string(), f.text().unwrap_or("").trim().to_string()))
                .collect()
        })
        .unwrap_or_default();
    let files = find_child(node, "files").map(|fs| parse_file_list(fs, seq)).unwrap_or_default();
    let type_descriptor = find_child(node, "typeDescriptor").map(parse_type_descriptor).unwrap_or(
        TypeDescriptor { default_type: PluginType::Optional, patterns: Vec::new() },
    );
    Plugin { name, description, image, type_descriptor, condition_flags, files }
}

fn parse_type_descriptor(node: Node) -> TypeDescriptor {
    if let Some(t) = find_child(node, "type") {
        return TypeDescriptor {
            default_type: PluginType::parse(t.attribute("name").unwrap_or("")),
            patterns: Vec::new(),
        };
    }
    if let Some(dt) = find_child(node, "dependencyType") {
        let default_type = find_child(dt, "defaultType")
            .map(|d| PluginType::parse(d.attribute("name").unwrap_or("")))
            .unwrap_or(PluginType::Optional);
        let mut patterns = Vec::new();
        if let Some(ps) = find_child(dt, "patterns") {
            for p in elements(ps, "pattern") {
                let plugin_type = find_child(p, "type")
                    .map(|t| PluginType::parse(t.attribute("name").unwrap_or("")))
                    .unwrap_or(PluginType::Optional);
                let condition = find_child(p, "dependencies")
                    .map(parse_composite)
                    .unwrap_or(Condition::Sub { op: Operator::And, conditions: Vec::new() });
                patterns.push(DependencyPattern { plugin_type, condition });
            }
        }
        return TypeDescriptor { default_type, patterns };
    }
    TypeDescriptor { default_type: PluginType::Optional, patterns: Vec::new() }
}

fn parse_conditional_installs(node: Node, seq: &mut u32) -> Vec<ConditionalInstall> {
    let mut out = Vec::new();
    if let Some(patterns) = find_child(node, "patterns") {
        for p in elements(patterns, "pattern") {
            let condition = find_child(p, "dependencies")
                .map(parse_composite)
                .unwrap_or(Condition::Sub { op: Operator::And, conditions: Vec::new() });
            let files = find_child(p, "files").map(|fs| parse_file_list(fs, seq)).unwrap_or_default();
            out.push(ConditionalInstall { condition, files });
        }
    }
    out
}

/// Parse a composite-dependency node (`<dependencies>` or `<visible>`): its element
/// children are conditions, combined by the node's `operator` (default `And`).
fn parse_composite(node: Node) -> Condition {
    let op = match node.attribute("operator") {
        Some("Or") => Operator::Or,
        _ => Operator::And,
    };
    let conditions = node.children().filter(|n| n.is_element()).filter_map(parse_condition).collect();
    Condition::Sub { op, conditions }
}

fn parse_condition(node: Node) -> Option<Condition> {
    match node.tag_name().name() {
        "fileDependency" => Some(Condition::File {
            file: norm_path(node.attribute("file").unwrap_or("")),
            state: node.attribute("state").unwrap_or("").to_string(),
        }),
        "flagDependency" => Some(Condition::Flag {
            flag: node.attribute("flag").unwrap_or("").to_string(),
            value: node.attribute("value").unwrap_or("").to_string(),
        }),
        "gameDependency" => Some(Condition::Version {
            kind: "Game".to_string(),
            version: node.attribute("version").unwrap_or("").to_string(),
        }),
        "fommDependency" => Some(Condition::Version {
            kind: "Fomm".to_string(),
            version: node.attribute("version").unwrap_or("").to_string(),
        }),
        "foseDependency" => Some(Condition::Version {
            kind: "Fose".to_string(),
            version: node.attribute("version").unwrap_or("").to_string(),
        }),
        "dependencies" => Some(parse_composite(node)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A synthetic ModuleConfig.xml exercising the schema (no third-party mod file in-tree).
    // Modelled on real FOMODs: a required set, one step with a SelectExactlyOne group, a
    // plugin with a conditional typeDescriptor, and a conditionalFileInstalls block.
    const XML: &str = r#"<config>
  <moduleName>Example Mod</moduleName>
  <moduleImage path="fomod\images\logo.jpg" />
  <requiredInstallFiles>
    <file source="Core\common.esp" destination="common.esp" />
    <folder source="Core\meshes" destination="meshes" />
  </requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Main">
      <optionalFileGroups order="Explicit">
        <group name="Variant" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="Standard">
              <description>The standard version.</description>
              <image path="fomod\images\std.jpg" />
              <conditionFlags><flag name="variant">standard</flag></conditionFlags>
              <files><file source="Standard\main.esp" destination="main.esp" priority="0" /></files>
              <typeDescriptor><type name="Recommended"/></typeDescriptor>
            </plugin>
            <plugin name="Lite">
              <description>The lite version.</description>
              <files><file source="Lite\main.esp" destination="main.esp" priority="0" /></files>
              <typeDescriptor>
                <dependencyType>
                  <defaultType name="Optional"/>
                  <patterns><pattern>
                    <dependencies operator="And">
                      <fileDependency file="SomeMaster.esm" state="Active"/>
                      <flagDependency flag="variant" value="standard"/>
                    </dependencies>
                    <type name="NotUsable"/>
                  </pattern></patterns>
                </dependencyType>
              </typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls>
    <patterns><pattern>
      <dependencies operator="And"><flagDependency flag="variant" value="standard"/></dependencies>
      <files><file source="Extra\patch.esp" destination="patch.esp" /></files>
    </pattern></patterns>
  </conditionalFileInstalls>
</config>"#;

    #[test]
    fn parses_the_schema() {
        let mc = ModuleConfig::parse(XML).expect("parse");

        assert_eq!(mc.module_name, "Example Mod");
        assert_eq!(mc.module_image.as_deref(), Some("fomod/images/logo.jpg"));
        assert_eq!(mc.required_files.len(), 2);
        assert!(mc.required_files[1].is_folder);
        assert_eq!(mc.required_files[1].destination, "meshes");

        assert_eq!(mc.steps.len(), 1);
        let g = &mc.steps[0].groups[0];
        assert_eq!(g.group_type, GroupType::SelectExactlyOne);
        assert_eq!(g.plugins.len(), 2);

        let std = &g.plugins[0];
        assert_eq!(std.name, "Standard");
        assert_eq!(std.type_descriptor.default_type, PluginType::Recommended);
        assert_eq!(std.condition_flags, vec![("variant".to_string(), "standard".to_string())]);
        assert_eq!(std.files[0].destination, "main.esp");

        // The "Lite" plugin becomes NotUsable when SomeMaster.esm is active AND variant=standard.
        let lite = &g.plugins[1];
        assert_eq!(lite.type_descriptor.default_type, PluginType::Optional);
        assert_eq!(lite.type_descriptor.patterns.len(), 1);
        assert_eq!(lite.type_descriptor.patterns[0].plugin_type, PluginType::NotUsable);
        assert_eq!(
            lite.type_descriptor.patterns[0].condition,
            Condition::Sub {
                op: Operator::And,
                conditions: vec![
                    Condition::File { file: "SomeMaster.esm".to_string(), state: "Active".to_string() },
                    Condition::Flag { flag: "variant".to_string(), value: "standard".to_string() },
                ],
            }
        );

        assert_eq!(mc.conditional_installs.len(), 1);
        assert_eq!(mc.conditional_installs[0].files[0].destination, "patch.esp");
    }

    #[test]
    fn decodes_utf16le_bom() {
        // "<a/>" in UTF-16LE with a BOM.
        let bytes = [0xFF, 0xFE, b'<', 0, b'a', 0, b'/', 0, b'>', 0];
        assert_eq!(decode_xml(&bytes), "<a/>");
    }
}
