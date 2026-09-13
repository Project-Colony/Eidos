use super::super::{OmodFileKind, OmodMember};
use super::*;
// Match the launch SDP composer limit before accepting a replacement request.
const MAX_SHADER_BYTES: u64 = 8 * 1024 * 1024;

fn member<'a>(
    session: &'a OmodSession,
    kind: OmodFileKind,
    name: &str,
) -> Result<&'a OmodMember, InstallError> {
    session
        .members
        .iter()
        .find(|m| m.kind == kind && m.path.eq_ignore_ascii_case(name))
        .ok_or_else(|| bad(format!("missing decoded OMOD member: {name}")))
}
fn input(session: &OmodSession, spec: &OmodMember) -> Result<fs::File, InstallError> {
    let root = if spec.kind == OmodFileKind::Data {
        session.data_root()
    } else {
        session.plugins_root()
    };
    let name = super::super::member_path(&spec.path)?;
    let path = checked_destination(session.tree.path(), &root.join(name))?;
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(bad("decoded source is not a regular file"));
    }
    context::Source::capture(path)?.open()
}
fn copy_member(
    session: &OmodSession,
    spec: &OmodMember,
    mut output: impl Write,
    max: u64,
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    session.verify_archive(cancel)?;
    if spec.size > max {
        return Err(bad("decoded member exceeds the byte bound"));
    }
    let mut file = input(session, spec)?;
    let mut count = 0u64;
    let mut crc = crc32fast::Hasher::new();
    let mut buffer = [0; 65536];
    loop {
        check_cancel(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > spec.size {
            return Err(bad("decoded OMOD member grew before publication"));
        }
        crc.update(&buffer[..n]);
        output.write_all(&buffer[..n])?;
    }
    if count != spec.size || crc.finalize() != spec.crc32 {
        return Err(bad(
            "decoded OMOD member checksum changed before publication",
        ));
    }
    session.verify_archive(cancel)?;
    Ok(())
}
fn read_member(
    session: &OmodSession,
    kind: OmodFileKind,
    name: &str,
    max: u64,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, InstallError> {
    let mut bytes = Vec::new();
    copy_member(
        session,
        member(session, kind, name)?,
        &mut bytes,
        max,
        cancel,
    )?;
    Ok(bytes)
}
fn target(stage: &Path, name: &str) -> Result<PathBuf, InstallError> {
    let name = super::super::member_path(name)?;
    let path = checked_destination(stage, &stage.join(name))?;
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    Ok(path)
}
fn stage_source(
    stage: &Path,
    name: &str,
    context: &ScriptedContext,
    cancel: &AtomicBool,
) -> Result<PathBuf, InstallError> {
    let path = target(stage, name)?;
    if !path.exists() {
        let source = context
            .sources
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| bad(format!("missing effective edit target: {name}")))?;
        let bytes = source.read(MAX_EFFECT_FILE, cancel)?;
        fs::write(&path, bytes)?;
    }
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(bad("effect target is not a regular staged file"));
    }
    Ok(path)
}
fn number<T: std::str::FromStr>(s: &str) -> Result<T, InstallError> {
    s.parse()
        .map_err(|_| bad("invalid evaluated effect number"))
}
fn shader_name(args: &[String]) -> Result<String, InstallError> {
    let package = number::<u8>(&args[0])?;
    let name = &args[1];
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return Err(bad("shader name requires .pso or .vso"));
    };
    if !name.is_ascii()
        || name.len() >= 256
        || stem.is_empty()
        || !matches!(ext.to_ascii_lowercase().as_str(), "pso" | "vso")
        || name.contains(['/', '\\'])
    {
        return Err(bad(
            "shader name must be one ASCII .pso/.vso component shorter than 256 bytes",
        ));
    }
    super::super::member_path(&format!(
        "Shaders/OMOD/{package}/{}.{}",
        stem.to_ascii_uppercase(),
        ext.to_ascii_lowercase()
    ))
}
pub(super) fn materialize(
    session: &OmodSession,
    context: &ScriptedContext,
    review: &ScriptedReview,
    stage: &Path,
    cancel: &AtomicBool,
) -> Result<(), InstallError> {
    let mut total = 0u64;
    let mut paths = std::collections::BTreeSet::new();
    for selected in &review.plan.files {
        check_cancel(cancel)?;
        let source = member(session, selected.kind, &selected.source)?;
        total = total
            .checked_add(source.size)
            .ok_or_else(|| bad("OMOD output size overflow"))?;
        if total > super::super::MAX_OUTPUT_BYTES {
            return Err(bad("scripted OMOD output exceeds 8 GiB"));
        }
        let path = target(stage, &selected.destination)?;
        if !paths.insert(selected.destination.to_ascii_lowercase()) {
            return Err(bad("duplicate selected OMOD destination"));
        }
        let file = fs::File::options()
            .write(true)
            .create_new(true)
            .open(path)?;
        copy_member(session, source, file, super::super::MAX_FILE_BYTES, cancel)?;
    }
    if let Some(known) = &review.known {
        let generated = known
            .generate(
                |name, max| {
                    read_member(session, OmodFileKind::Data, name, max, cancel).map_err(obmm_error)
                },
                |archive, name, max| {
                    let source = context
                        .sources
                        .get(&archive.to_ascii_lowercase())
                        .ok_or_else(|| ObmmError {
                            line: 0,
                            message: format!("missing effective BSA: {archive}"),
                        })?;
                    source.verify().map_err(obmm_error)?;
                    let identity =
                        eidos_conflicts::archive_identity(&source.path).map_err(|e| ObmmError {
                            line: 0,
                            message: e.to_string(),
                        })?;
                    let bytes = eidos_conflicts::read_archive_member_checked(
                        &source.path,
                        name,
                        max,
                        Some(&identity),
                    )
                    .map_err(|e| ObmmError {
                        line: 0,
                        message: e.to_string(),
                    })?;
                    source.verify().map_err(obmm_error)?;
                    Ok(bytes)
                },
                cancel,
            )
            .map_err(|e| bad(e.to_string()))?;
        if !generated
            .iter()
            .map(|f| &f.destination)
            .eq(review.generated_files.iter())
        {
            return Err(bad("native generated destinations changed after review"));
        }
        for generated in generated {
            total += generated.bytes.len() as u64;
            if total > super::super::MAX_OUTPUT_BYTES {
                return Err(bad("generated OMOD output exceeds 8 GiB"));
            }
            let path = target(stage, &generated.destination)?;
            fs::write(path, generated.bytes)?;
        }
    }
    for effect in &review.plan.effects {
        check_cancel(cancel)?;
        let a = &effect.arguments;
        match effect.command.as_str() {
            "EditXMLReplace" | "EditXMLLine" => {
                let path = stage_source(stage, &a[0], context, cancel)?;
                let source = context::Source::capture(path.clone())?;
                let bytes = source.read(MAX_EFFECT_FILE, cancel)?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| bad(format!("Line {}: XML edit requires UTF-8", effect.line)))?;
                let output = if effect.command == "EditXMLReplace" {
                    if a[1].is_empty() {
                        return Err(bad("empty XML replacement"));
                    }
                    let count = text.matches(&a[1]).count();
                    let size = text
                        .len()
                        .checked_add(count.saturating_mul(a[2].len().saturating_sub(a[1].len())))
                        .ok_or_else(|| bad("XML replacement size overflow"))?;
                    if size as u64 > MAX_EFFECT_FILE {
                        return Err(bad("XML replacement exceeds byte bound"));
                    }
                    text.replace(&a[1], &a[2])
                } else {
                    let index = number::<usize>(&a[1])?;
                    let mut lines: Vec<_> = text.lines().map(str::to_string).collect();
                    if index >= lines.len() {
                        return Err(bad("XML line index is outside the staged file"));
                    }
                    lines[index] = a[2].clone();
                    let nl = eidos_ini::newline_style(&text);
                    let mut output = lines.join(nl);
                    if text.ends_with('\n') {
                        output.push_str(nl);
                    }
                    output
                };
                if output.len() as u64 > MAX_EFFECT_FILE {
                    return Err(bad("XML edit exceeds byte bound"));
                }
                fs::write(path, output)?;
            }
            "SetPluginByte" | "SetPluginShort" | "SetPluginInt" | "SetPluginLong"
            | "SetPluginFloat" => {
                use std::io::{Seek, SeekFrom};
                let path = stage_source(stage, &a[0], context, cancel)?;
                let offset = number::<u64>(&a[1])?;
                let bytes = match effect.command.as_str() {
                    "SetPluginByte" => vec![number::<u8>(&a[2])?],
                    "SetPluginShort" => number::<i16>(&a[2])?.to_le_bytes().to_vec(),
                    "SetPluginInt" => number::<i32>(&a[2])?.to_le_bytes().to_vec(),
                    "SetPluginLong" => number::<i64>(&a[2])?.to_le_bytes().to_vec(),
                    _ => number::<f32>(&a[2])?.to_le_bytes().to_vec(),
                };
                let end = offset
                    .checked_add(bytes.len() as u64)
                    .ok_or_else(|| bad("plugin offset overflow"))?;
                if end > super::super::MAX_FILE_BYTES {
                    return Err(bad("plugin effect exceeds 2 GiB bound"));
                }
                let mut f = fs::File::options().write(true).open(path)?;
                f.seek(SeekFrom::Start(offset))?;
                f.write_all(&bytes)?;
            }
            "PatchDataFile" | "PatchPlugin" => {
                if a[2] != "True" && !context.sources.contains_key(&a[1].to_ascii_lowercase()) {
                    return Err(bad("patch target disappeared"));
                }
                let kind = if effect.command == "PatchPlugin" {
                    OmodFileKind::Plugin
                } else {
                    OmodFileKind::Data
                };
                let path = target(stage, &a[1])?;
                let f = fs::File::create(path)?;
                copy_member(
                    session,
                    member(session, kind, &a[0])?,
                    f,
                    super::super::MAX_FILE_BYTES,
                    cancel,
                )?;
            }
            "EditSDP" | "EditShader" => {
                let name = shader_name(a)?;
                let selected =
                    checked_destination(stage, &stage.join(super::super::member_path(&a[2])?))?;
                let bytes = if selected.is_file() {
                    context::Source::capture(selected)?.read(MAX_SHADER_BYTES, cancel)?
                } else if session
                    .members
                    .iter()
                    .any(|m| m.kind == OmodFileKind::Data && m.path.eq_ignore_ascii_case(&a[2]))
                {
                    read_member(session, OmodFileKind::Data, &a[2], MAX_SHADER_BYTES, cancel)?
                } else {
                    let source = stage_source(stage, &a[2], context, cancel)?;
                    context::Source::capture(source)?.read(MAX_SHADER_BYTES, cancel)?
                };
                fs::write(target(stage, &name)?, bytes)?;
            }
            _ => {} // Profile requests and explicitly reviewed unsupported proposals.
        }
    }
    // Count and validate the FINAL tree too: effects may add files or grow them.
    validate_tree(stage, cancel)?;
    Ok(())
}
fn obmm_error(e: InstallError) -> ObmmError {
    ObmmError {
        line: 0,
        message: e.to_string(),
    }
}
pub(super) fn validate_tree(root: &Path, cancel: &AtomicBool) -> Result<(), InstallError> {
    let mut dirs = vec![(root.to_path_buf(), 0usize)];
    let mut total = 0u64;
    let mut count = 0usize;
    while let Some((dir, depth)) = dirs.pop() {
        check_cancel(cancel)?;
        if depth > 128 {
            return Err(bad("stage exceeds directory depth bound"));
        }
        let mut names = std::collections::BTreeSet::new();
        for e in fs::read_dir(&dir)? {
            let e = e?;
            let name = e
                .file_name()
                .into_string()
                .map_err(|_| bad("non-UTF8 staged name"))?;
            if !names.insert(name.to_ascii_lowercase()) {
                return Err(bad("ambiguous staged case variants"));
            }
            let p = checked_destination(root, &e.path())?;
            let m = fs::symlink_metadata(&p)?;
            if m.is_dir() {
                dirs.push((p, depth + 1));
            } else if m.is_file() {
                count += 1;
                total = total
                    .checked_add(m.len())
                    .ok_or_else(|| bad("stage size overflow"))?;
                if m.len() > super::super::MAX_FILE_BYTES
                    || total > super::super::MAX_OUTPUT_BYTES
                    || count > super::super::MAX_FILES
                {
                    return Err(bad("stage exceeds OMOD output bounds"));
                }
            } else {
                return Err(bad("stage contains a link or special file"));
            }
        }
    }
    Ok(())
}
