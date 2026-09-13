//! Per-launch Oblivion shader composition. No source or persistent Data writes.
//!
//! Format and mappings: ModOrganizer2/installer_omod
//! 43caaa5e849c572791b79255e2331302135be208, src/installerOmod.cpp:127–203;
//! OMODFramework 05b3d5629124cfa44308b3b78c38959b960b3730,
//! OMODFramework/Oblivion/OblivionSDP.cs. The format has no checksum: SHA-256
//! receipts below validate source identity separately from its length fields.

use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use eidos_core::LayerStack;
use sha2::{Digest, Sha256};

const MAX_PACKAGE: usize = 64 * 1024 * 1024;
const MAX_SHADER: usize = 8 * 1024 * 1024;
const MAX_SHADERS: usize = 16_384;
const MAX_SESSION: usize = 256 * 1024 * 1024;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn cancelled(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Shader preparation cancelled",
        ))
    } else {
        Ok(())
    }
}
fn shader_name(name: &str) -> io::Result<String> {
    if name.is_empty()
        || name.len() >= 256
        || !name.is_ascii()
        || name
            .bytes()
            .any(|b| b < 32 || b == 127 || b"/\\:*?\"<>|".contains(&b))
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
    {
        return Err(invalid(format!("Invalid shader name: {name:?}")));
    }
    let (base, ext) = name
        .rsplit_once('.')
        .ok_or_else(|| invalid("Shader name has no extension"))?;
    if base.is_empty() || !["pso", "vso"].iter().any(|e| ext.eq_ignore_ascii_case(e)) {
        return Err(invalid(format!("Unsupported shader name: {name}")));
    }
    Ok(format!(
        "{}.{}",
        base.to_ascii_uppercase(),
        ext.to_ascii_lowercase()
    ))
}

/// Compose the actual SDP format: magic 100, count, body length, then a 256-byte
/// NUL-padded name, little-endian byte length and bytecode for each record.
/// MO2 permits adding a shader absent from the source package. Names compare
/// case-insensitively; ambiguous input records are refused, never overwritten
/// silently. Unedited names and bytecode are preserved; output is name-sorted.
pub fn compose_package(
    source: &[u8],
    edits: &BTreeMap<String, Vec<u8>>,
    cancel: &AtomicBool,
) -> io::Result<Vec<u8>> {
    cancelled(cancel)?;
    if source.len() < 12 || source.len() > MAX_PACKAGE {
        return Err(invalid("SDP package size outside bounds"));
    }
    if edits.len() > MAX_SHADERS
        || edits
            .values()
            .try_fold(0usize, |n, v| n.checked_add(v.len()))
            .is_none_or(|n| n > MAX_PACKAGE)
    {
        return Err(invalid("Shader edits exceed aggregate package bounds"));
    }
    let u32_at = |at: usize| u32::from_le_bytes(source[at..at + 4].try_into().unwrap()) as usize;
    let count = u32_at(4);
    if u32_at(0) != 100
        || u32_at(8) != source.len() - 12
        || count > MAX_SHADERS
        || count > (source.len() - 12) / 260
    {
        return Err(invalid("Invalid SDP header, record count or body length"));
    }
    let mut records = BTreeMap::new();
    let mut pos = 12usize;
    for _ in 0..count {
        cancelled(cancel)?;
        let header = source
            .get(pos..pos + 260)
            .ok_or_else(|| invalid("Truncated SDP record"))?;
        let field: [u8; 256] = header[..256].try_into().unwrap();
        let end = field
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| invalid("SDP name is not NUL-terminated"))?;
        if field[end..].iter().any(|b| *b != 0) {
            return Err(invalid("SDP name has nonzero padding"));
        }
        let name =
            std::str::from_utf8(&field[..end]).map_err(|_| invalid("SDP name is not ASCII"))?;
        let key = shader_name(name)?.to_ascii_lowercase();
        let len = u32::from_le_bytes(header[256..260].try_into().unwrap()) as usize;
        pos += 260;
        if len > MAX_SHADER {
            return Err(invalid("SDP shader exceeds bytecode limit"));
        }
        let bytes = source
            .get(pos..pos + len)
            .ok_or_else(|| invalid("Truncated SDP shader"))?;
        pos += len;
        if records.insert(key, (field, bytes.to_vec())).is_some() {
            return Err(invalid("Duplicate case-insensitive SDP shader"));
        }
    }
    if pos != source.len() {
        return Err(invalid("Trailing SDP data or incorrect record count"));
    }
    let mut applied = std::collections::BTreeSet::new();
    for (name, data) in edits {
        cancelled(cancel)?;
        let name = shader_name(name)?;
        let key = name.to_ascii_lowercase();
        if !applied.insert(key.clone()) {
            return Err(invalid("Duplicate case-insensitive shader edit"));
        }
        if data.len() > MAX_SHADER {
            return Err(invalid("Replacement shader exceeds bytecode limit"));
        }
        let mut field = [0; 256];
        field[..name.len()].copy_from_slice(name.as_bytes());
        records
            .entry(key)
            .and_modify(|r| r.1.clone_from(data))
            .or_insert((field, data.clone()));
    }
    let size = records
        .values()
        .try_fold(0usize, |size, (_, bytes)| {
            size.checked_add(260 + bytes.len())
                .filter(|n| *n <= MAX_PACKAGE - 12)
        })
        .ok_or_else(|| invalid("Composed SDP exceeds package limit"))?;
    if records.len() > MAX_SHADERS {
        return Err(invalid("Composed SDP exceeds record limit"));
    }
    let mut out = Vec::with_capacity(12 + size);
    out.extend(100u32.to_le_bytes());
    out.extend((records.len() as u32).to_le_bytes());
    out.extend((size as u32).to_le_bytes());
    let mut sorted: Vec<_> = records.into_values().collect();
    sorted.sort_by_key(|r| r.0);
    for (field, bytes) in sorted {
        cancelled(cancel)?;
        out.extend(field);
        out.extend((bytes.len() as u32).to_le_bytes());
        out.extend(bytes);
    }
    Ok(out)
}

fn c_name(name: &std::ffi::OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| invalid("Path contains NUL"))
}
fn open_at(dir: &File, name: &std::ffi::OsStr, flags: i32) -> io::Result<File> {
    let name = c_name(name)?;
    // SAFETY: live directory descriptor and NUL-terminated component; returned
    // descriptor is owned exactly once. O_NOFOLLOW also rejects final symlinks.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_nofollow(path: &Path, directory: bool) -> io::Result<File> {
    if !path.is_absolute() {
        return Err(invalid("Shader provider paths must be absolute"));
    }
    let components: Vec<_> = path
        .components()
        .filter(|c| *c != Component::RootDir)
        .collect();
    let mut fd = File::open("/")?;
    for (i, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(invalid("Non-normal shader provider path"));
        };
        let is_dir = directory || i + 1 < components.len();
        let flags = libc::O_RDONLY
            | if is_dir {
                libc::O_DIRECTORY
            } else {
                libc::O_NONBLOCK
            };
        fd = open_at(&fd, name, flags)?;
    }
    let metadata = fd.metadata()?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(invalid("Unexpected shader provider type"));
    }
    Ok(fd)
}

#[derive(Debug, PartialEq, Eq)]
struct Source {
    virtual_path: String,
    physical: PathBuf,
    size: u64,
    device: u64,
    inode: u64,
    digest: [u8; 32],
}
fn read_source(
    view: &LayerStack,
    virtual_path: &str,
    limit: usize,
    cancel: &AtomicBool,
) -> io::Result<(Source, Vec<u8>)> {
    cancelled(cancel)?;
    let physical = view
        .resolve_read(virtual_path)
        .ok_or_else(|| invalid(format!("Missing shader source: {virtual_path}")))?;
    // The union's exact-case resolution is intentionally permissive. A generated
    // package cannot represent two case-colliding physical winners, so reject
    // those source paths instead of choosing a filesystem enumeration order.
    for ancestor in physical.ancestors().take_while(|p| p.parent().is_some()) {
        let name = ancestor.file_name().unwrap().as_bytes();
        let matches = fs::read_dir(ancestor.parent().unwrap())?
            .filter_map(Result::ok)
            .filter(|e| e.file_name().as_bytes().eq_ignore_ascii_case(name))
            .count();
        if matches != 1 {
            return Err(invalid(format!(
                "Ambiguous shader provider: {}",
                physical.display()
            )));
        }
    }
    let mut file = open_nofollow(&physical, false)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > limit as u64 {
        return Err(invalid(format!(
            "Shader source outside size/type bounds: {virtual_path}"
        )));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file).take(limit as u64 + 1).read_to_end(&mut bytes)?;
    cancelled(cancel)?;
    let after = file.metadata()?;
    if bytes.len() > limit
        || bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || after.mtime() != before.mtime()
        || after.mtime_nsec() != before.mtime_nsec()
        || after.ctime() != before.ctime()
        || after.ctime_nsec() != before.ctime_nsec()
    {
        return Err(invalid(format!(
            "Shader source changed while reading: {virtual_path}"
        )));
    }
    Ok((
        Source {
            virtual_path: virtual_path.into(),
            physical,
            size: before.len(),
            device: before.dev(),
            inode: before.ino(),
            digest: Sha256::digest(&bytes).into(),
        },
        bytes,
    ))
}

fn requests(view: &LayerStack) -> io::Result<BTreeMap<u8, BTreeMap<String, String>>> {
    let mut packages = BTreeMap::new();
    let mut request_count = 0usize;
    for (package, _, kind) in view.list_dir_typed("Shaders/OMOD") {
        let id: u8 = package
            .parse()
            .map_err(|_| invalid(format!("Invalid OMOD shader package directory: {package}")))?;
        if package != id.to_string() || !kind.is_some_and(|k| k.is_dir()) {
            return Err(invalid("Noncanonical or non-directory OMOD shader package"));
        }
        let mut edits = BTreeMap::new();
        for (name, _, kind) in view.list_dir_typed(&format!("Shaders/OMOD/{package}")) {
            let canonical = shader_name(&name)?;
            if !kind.is_some_and(|k| k.is_file()) {
                return Err(invalid("Shader replacement must be a regular file"));
            }
            edits.insert(canonical, format!("Shaders/OMOD/{package}/{name}"));
            request_count += 1;
            if request_count > MAX_SHADERS {
                return Err(invalid("Too many shader replacements across packages"));
            }
        }
        if !edits.is_empty() {
            packages.insert(id, edits);
        }
    }
    Ok(packages)
}

/// Owned, per-profile temporary output. Retain this value until the launch has
/// detached its exact-file mappings. Explicit cleanup surfaces errors; Drop is
/// a fallback for cancellation/errors before launch. It deletes only filenames
/// this session created, using held directory descriptors, never a broad tree.
pub struct ShaderSession {
    layers: Vec<PathBuf>,
    overwrite: PathBuf,
    requests: BTreeMap<u8, BTreeMap<String, String>>,
    sources: Vec<Source>,
    mappings: Vec<(PathBuf, PathBuf)>,
    generated: Vec<[u8; 32]>,
    dir: File,
    parent: File,
    name: CString,
    cleaned: bool,
}
impl ShaderSession {
    /// `(staged file, strictly relative Data destination)` for read-only binds
    /// after the Data union mount, so even an Overwrite source is superseded.
    pub fn mappings(&self) -> &[(PathBuf, PathBuf)] {
        &self.mappings
    }

    /// Rebuild the effective view and reject changed, missing or newly shadowed
    /// inputs, changed request membership, and changed generated output bytes.
    pub fn validate_sources(&self, cancel: &AtomicBool) -> io::Result<()> {
        cancelled(cancel)?;
        let view = LayerStack::new(self.layers.clone(), self.overwrite.clone());
        if requests(&view)? != self.requests {
            return Err(invalid("Shader requests changed before launch"));
        }
        for source in &self.sources {
            let (now, _) = read_source(&view, &source.virtual_path, MAX_PACKAGE, cancel)?;
            if &now != source {
                return Err(invalid(format!(
                    "Shader source winner changed before launch: {}",
                    source.virtual_path
                )));
            }
        }
        for ((path, _), digest) in self.mappings.iter().zip(&self.generated) {
            let mut bytes = Vec::new();
            open_nofollow(path, false)?
                .take(MAX_PACKAGE as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_PACKAGE || <[u8; 32]>::from(Sha256::digest(&bytes)) != *digest {
                return Err(invalid("Generated shader package changed before launch"));
            }
        }
        cancelled(cancel)
    }
    pub fn cleanup(mut self) -> io::Result<()> {
        self.remove_owned()
    }
    fn remove_owned(&mut self) -> io::Result<()> {
        if self.cleaned {
            return Ok(());
        }
        let current = open_at(
            &self.parent,
            std::ffi::OsStr::from_bytes(self.name.as_bytes()),
            libc::O_RDONLY | libc::O_DIRECTORY,
        )?
        .metadata()?;
        let owned = self.dir.metadata()?;
        if current.dev() != owned.dev() || current.ino() != owned.ino() {
            return Err(invalid(
                "Shader session directory was replaced; cleanup refused",
            ));
        }
        for (path, _) in &self.mappings {
            let name = c_name(path.file_name().unwrap())?;
            // SAFETY: held stage descriptor and a single component. unlinkat
            // removes a replaced symlink itself; it cannot follow its target.
            if unsafe { libc::unlinkat(self.dir.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                let e = io::Error::last_os_error();
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }
        if unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        self.cleaned = true;
        Ok(())
    }
}
impl Drop for ShaderSession {
    fn drop(&mut self) {
        let _ = self.remove_owned();
    }
}

/// Read effective installed `Shaders/OMOD/<u8>/<shader>` requests and compose
/// complete packages into a new private child of `session_parent`. The parent
/// must already exist outside every mounted/source tree. Layers are highest
/// priority first and include the pristine game Data as the final provider.
pub fn prepare(
    layers: Vec<PathBuf>,
    overwrite: PathBuf,
    session_parent: &Path,
    cancel: &AtomicBool,
) -> io::Result<Option<ShaderSession>> {
    cancelled(cancel)?;
    let view = LayerStack::new(layers.clone(), overwrite.clone());
    let requests = requests(&view)?;
    if requests.is_empty() {
        return Ok(None);
    }
    if layers
        .iter()
        .chain([&overwrite])
        .any(|p| session_parent.starts_with(p))
    {
        return Err(invalid("Shader session must be outside all Data providers"));
    }
    let parent = open_nofollow(session_parent, true)?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (name, root) = loop {
        let suffix = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!("omod-shaders-{}-{suffix}", std::process::id())).unwrap();
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } == 0 {
            let root = session_parent.join(name.to_str().unwrap());
            break (name, root);
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
        if suffix > 10_000 {
            return Err(invalid("Too many stale shader sessions"));
        }
    };
    let dir = open_at(
        &parent,
        std::ffi::OsStr::from_bytes(name.as_bytes()),
        libc::O_RDONLY | libc::O_DIRECTORY,
    )?;
    let mut session = ShaderSession {
        layers,
        overwrite,
        requests,
        sources: Vec::new(),
        mappings: Vec::new(),
        generated: Vec::new(),
        dir,
        parent,
        name,
        cleaned: false,
    };
    let mut total = 0usize;
    for (package, edits) in &session.requests {
        cancelled(cancel)?;
        let target = format!("Shaders/shaderpackage{package:03}.sdp");
        let (source, bytes) = read_source(&view, &target, MAX_PACKAGE, cancel)?;
        session.sources.push(source);
        let mut replacements = BTreeMap::new();
        for (name, virtual_path) in edits {
            let (source, bytes) = read_source(&view, virtual_path, MAX_SHADER, cancel)?;
            total = total
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_SESSION)
                .ok_or_else(|| invalid("Shader session exceeds aggregate limit"))?;
            session.sources.push(source);
            replacements.insert(name.clone(), bytes);
        }
        let output = compose_package(&bytes, &replacements, cancel)?;
        total = total
            .checked_add(output.len())
            .filter(|n| *n <= MAX_SESSION)
            .ok_or_else(|| invalid("Shader session exceeds aggregate limit"))?;
        let filename = format!("shaderpackage{package:03}.sdp");
        let mut file = open_at(
            &session.dir,
            std::ffi::OsStr::new(&filename),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        session
            .mappings
            .push((root.join(filename), PathBuf::from(target)));
        file.write_all(&output)?;
        file.sync_all()?;
        session.generated.push(Sha256::digest(&output).into());
    }
    session.validate_sources(cancel)?;
    Ok(Some(session))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn package(records: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::from(100u32.to_le_bytes());
        out.extend((records.len() as u32).to_le_bytes());
        out.extend(0u32.to_le_bytes());
        for (name, bytes) in records {
            let mut field = [0; 256];
            field[..name.len()].copy_from_slice(name.as_bytes());
            out.extend(field);
            out.extend((bytes.len() as u32).to_le_bytes());
            out.extend(*bytes);
        }
        let size = (out.len() - 12) as u32;
        out[8..12].copy_from_slice(&size.to_le_bytes());
        out
    }

    #[test]
    fn compose_replaces_adds_and_preserves_unrelated_bytes() {
        let original = package(&[("A.pso", b"old"), ("KEEP.vso", b"untouched")]);
        let edits = BTreeMap::from([
            ("a.PSO".into(), b"new shader".to_vec()),
            ("NEW.vso".into(), vec![0, 1, 2]),
        ]);
        let result = compose_package(&original, &edits, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            result,
            package(&[
                ("A.pso", b"new shader"),
                ("KEEP.vso", b"untouched"),
                ("NEW.vso", &[0, 1, 2])
            ])
        );
        assert_eq!(
            original,
            package(&[("A.pso", b"old"), ("KEEP.vso", b"untouched")])
        );
    }

    #[test]
    fn malformed_packages_and_effect_names_fail_closed() {
        let valid = package(&[("A.pso", b"x")]);
        let empty = BTreeMap::new();
        for end in 0..valid.len() {
            assert!(
                compose_package(&valid[..end], &empty, &AtomicBool::new(false)).is_err(),
                "truncation {end}"
            );
        }
        for (offset, value) in [(0, 101u32), (4, u32::MAX), (8, 1), (268, u32::MAX)] {
            let mut bad = valid.clone();
            bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(compose_package(&bad, &empty, &AtomicBool::new(false)).is_err());
        }
        let mut no_nul = valid.clone();
        no_nul[12..268].fill(b'a');
        assert!(compose_package(&no_nul, &empty, &AtomicBool::new(false)).is_err());
        assert!(compose_package(
            &package(&[("A.pso", b"1"), ("a.PSO", b"2")]),
            &empty,
            &AtomicBool::new(false)
        )
        .is_err());
        for name in [
            "../A.pso", "a/b.pso", "a\\b.pso", "a.txt", "a\0.pso", "é.pso",
        ] {
            assert!(compose_package(
                &valid,
                &BTreeMap::from([(name.into(), vec![1])]),
                &AtomicBool::new(false)
            )
            .is_err());
        }
        assert!(compose_package(&valid, &empty, &AtomicBool::new(true)).is_err());
    }

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "eidos-omod-shader-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn put(&self, path: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            path
        }
        fn prepare(&self, profile: &str, mods: &[&str]) -> io::Result<Option<ShaderSession>> {
            let mut layers: Vec<_> = mods.iter().map(|p| self.0.join(p)).collect();
            layers.push(self.0.join("game"));
            fs::create_dir_all(self.0.join(profile)).unwrap();
            prepare(
                layers,
                self.0.join("overwrite"),
                &self.0.join(profile),
                &AtomicBool::new(false),
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn winner_order_revalidation_profile_switch_and_owned_cleanup() {
        let f = Fixture::new();
        let original = package(&[("A.pso", b"base"), ("B.vso", b"keep")]);
        let base = f.put("game/Shaders/shaderpackage001.sdp", &original);
        f.put("low/Shaders/OMOD/1/A.pso", b"low");
        f.put("high/shaders/omod/1/a.PSO", b"high");
        let upper = f.put(
            "overwrite/Shaders/shaderpackage001.sdp",
            &package(&[("A.pso", b"overwrite"), ("B.vso", b"upper keep")]),
        );
        let a = f.prepare("profile-a", &["high", "low"]).unwrap().unwrap();
        let staged = a.mappings()[0].0.clone();
        assert_eq!(
            a.mappings()[0].1,
            PathBuf::from("Shaders/shaderpackage001.sdp")
        );
        assert_eq!(
            fs::read(&staged).unwrap(),
            package(&[("A.pso", b"high"), ("B.vso", b"upper keep")])
        );
        a.validate_sources(&AtomicBool::new(false)).unwrap();
        let b = f.prepare("profile-b", &["low"]).unwrap().unwrap();
        assert_eq!(
            fs::read(&b.mappings()[0].0).unwrap(),
            package(&[("A.pso", b"low"), ("B.vso", b"upper keep")])
        );
        f.put("high/shaders/omod/1/a.PSO", b"changed");
        assert!(a.validate_sources(&AtomicBool::new(false)).is_err());
        a.cleanup().unwrap();
        assert!(!staged.exists());
        assert!(b.mappings()[0].0.exists());
        drop(b);
        assert_eq!(fs::read(base).unwrap(), original);
        assert_eq!(
            fs::read(upper).unwrap(),
            package(&[("A.pso", b"overwrite"), ("B.vso", b"upper keep")])
        );
        assert_eq!(fs::read_dir(f.0.join("profile-a")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(f.0.join("profile-b")).unwrap().count(), 0);
    }

    #[test]
    fn missing_sources_symlinks_aliases_cancel_and_no_effects_leave_no_stage() {
        let f = Fixture::new();
        assert!(f.prepare("profile", &[]).unwrap().is_none());
        f.put("mod/Shaders/OMOD/1/A.pso", b"new");
        assert!(f.prepare("profile", &["mod"]).is_err());
        f.put(
            "game/Shaders/shaderpackage001.sdp",
            &package(&[("A.pso", b"old")]),
        );
        f.put("mod/Shaders/OMOD/01/B.pso", b"alias");
        assert!(f.prepare("profile", &["mod"]).is_err());
        fs::remove_dir_all(f.0.join("mod/Shaders/OMOD/01")).unwrap();
        let shader = f.0.join("mod/Shaders/OMOD/1/A.pso");
        fs::remove_file(&shader).unwrap();
        let outside = f.put("outside", b"secret");
        std::os::unix::fs::symlink(&outside, &shader).unwrap();
        assert!(f.prepare("profile", &["mod"]).is_err());
        fs::remove_file(&shader).unwrap();
        fs::write(&shader, b"new").unwrap();
        assert!(prepare(
            vec![f.0.join("mod"), f.0.join("game")],
            f.0.join("overwrite"),
            &f.0.join("profile"),
            &AtomicBool::new(true)
        )
        .is_err());
        assert_eq!(fs::read_dir(f.0.join("profile")).unwrap().count(), 0);
        assert_eq!(fs::read(outside).unwrap(), b"secret");
    }
    #[test]
    fn inherited_deletions_merged_requests_and_changed_winners_are_checked() {
        let f = Fixture::new();
        f.put(
            "game/Shaders/shaderpackage002.sdp",
            &package(&[("A.pso", b"base"), ("B.vso", b"base b")]),
        );
        f.put("low/Shaders/OMOD/2/A.pso", b"low a");
        f.put("high/Shaders/OMOD/2/B.vso", b"high b");
        let session = f.prepare("profile", &["high", "low"]).unwrap().unwrap();
        assert_eq!(
            fs::read(&session.mappings()[0].0).unwrap(),
            package(&[("A.pso", b"low a"), ("B.vso", b"high b")])
        );
        f.put("overwrite/Shaders/OMOD/2/A.pso", b"overwrite a");
        assert!(session.validate_sources(&AtomicBool::new(false)).is_err());
        drop(session);
        let session = f.prepare("profile", &["high", "low"]).unwrap().unwrap();
        assert_eq!(
            fs::read(&session.mappings()[0].0).unwrap(),
            package(&[("A.pso", b"overwrite a"), ("B.vso", b"high b")])
        );
        fs::remove_file(f.0.join("overwrite/Shaders/OMOD/2/A.pso")).unwrap();
        f.put("overwrite/Shaders/OMOD/2/.eidoswh.a.pso", b"");
        assert!(session.validate_sources(&AtomicBool::new(false)).is_err());
        drop(session);
        let session = f.prepare("profile", &["high", "low"]).unwrap().unwrap();
        assert_eq!(
            fs::read(&session.mappings()[0].0).unwrap(),
            package(&[("A.pso", b"base"), ("B.vso", b"high b")])
        );
        drop(session);
        f.put("overwrite/Shaders/.eidoswh.shaderpackage002.sdp", b"");
        assert!(f.prepare("profile", &["high", "low"]).is_err());
        assert_eq!(fs::read_dir(f.0.join("profile")).unwrap().count(), 0);
    }

    #[test]
    fn collisions_nonregular_and_replaced_parent_cleanup_fail_closed() {
        let f = Fixture::new();
        f.put(
            "game/Shaders/shaderpackage001.sdp",
            &package(&[("A.pso", b"old")]),
        );
        f.put("mod/Shaders/OMOD/1/A.pso", b"new");
        f.put("mod/Shaders/OMOD/1/a.PSO", b"ambiguous");
        assert!(f.prepare("profile", &["mod"]).is_err());
        fs::remove_file(f.0.join("mod/Shaders/OMOD/1/a.PSO")).unwrap();
        let session = f.prepare("profile", &["mod"]).unwrap().unwrap();
        let stage = session.mappings()[0].0.parent().unwrap().to_path_buf();
        fs::write(stage.join("foreign"), b"preserve").unwrap();
        assert!(session.cleanup().is_err());
        assert_eq!(fs::read(stage.join("foreign")).unwrap(), b"preserve");
        fs::remove_dir_all(stage).unwrap();
        let session = f.prepare("profile", &["mod"]).unwrap().unwrap();
        fs::rename(f.0.join("profile"), f.0.join("old-profile")).unwrap();
        fs::create_dir(f.0.join("profile")).unwrap();
        let sentinel = f.put("profile/unrelated", b"keep");
        assert!(session.validate_sources(&AtomicBool::new(false)).is_err());
        session.cleanup().unwrap();
        assert_eq!(fs::read(sentinel).unwrap(), b"keep");
        assert_eq!(fs::read_dir(f.0.join("old-profile")).unwrap().count(), 0);
    }

    #[test]
    fn size_limits_and_output_tampering_are_rejected() {
        let original = package(&[("A.pso", b"old")]);
        assert!(compose_package(
            &original,
            &BTreeMap::from([("A.pso".into(), vec![0; MAX_SHADER + 1])]),
            &AtomicBool::new(false)
        )
        .is_err());
        assert!(compose_package(
            &original,
            &BTreeMap::from([("A.pso".into(), vec![1]), ("a.PSO".into(), vec![2])]),
            &AtomicBool::new(false)
        )
        .is_err());
        let f = Fixture::new();
        f.put("game/Shaders/shaderpackage001.sdp", &original);
        let shader = f.put("mod/Shaders/OMOD/1/A.pso", b"new");
        File::options()
            .write(true)
            .open(&shader)
            .unwrap()
            .set_len(MAX_SHADER as u64 + 1)
            .unwrap();
        assert!(f.prepare("profile", &["mod"]).is_err());
        fs::write(shader, b"new").unwrap();
        let session = f.prepare("profile", &["mod"]).unwrap().unwrap();
        fs::write(&session.mappings()[0].0, package(&[("A.pso", b"bad")])).unwrap();
        assert!(session.validate_sources(&AtomicBool::new(false)).is_err());
        session.cleanup().unwrap();
        assert_eq!(fs::read_dir(f.0.join("profile")).unwrap().count(), 0);
    }
    #[test]
    fn substituted_stage_directory_is_not_removed() {
        let f = Fixture::new();
        f.put(
            "game/Shaders/shaderpackage001.sdp",
            &package(&[("A.pso", b"old")]),
        );
        f.put("mod/Shaders/OMOD/1/A.pso", b"new");
        let session = f.prepare("profile", &["mod"]).unwrap().unwrap();
        let stage = session.mappings()[0].0.parent().unwrap().to_path_buf();
        let renamed = stage.with_extension("moved");
        fs::rename(&stage, &renamed).unwrap();
        fs::create_dir(&stage).unwrap();
        assert!(session.cleanup().is_err());
        assert!(
            stage.is_dir(),
            "cleanup must not remove an unowned replacement directory"
        );
        assert!(renamed.join("shaderpackage001.sdp").is_file());
    }
}
