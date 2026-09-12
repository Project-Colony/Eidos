use crate::{Addon, Context};
use std::io::{self, Read};
use std::os::{fd::AsFd, unix::process::CommandExt};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const OUTPUT_LIMIT: usize = 1024 * 1024;

/// Run a trusted extension with bounded output, cancellation and a deadline.
/// The deadline includes inherited pipes. This is process isolation, not a
/// filesystem sandbox: extensions have the current user's access rights.
pub fn capture(
    addon: &Addon,
    context: &Context,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Output, String> {
    let mut context = context.clone();
    if let Some(parent) = addon.source.parent() {
        context
            .values
            .insert("addon_dir".into(), parent.display().to_string());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("extension cancelled".into());
    }
    for value in addon.args.iter().chain(std::iter::once(&addon.workdir)) {
        let missing = context.missing(value);
        if !missing.is_empty() {
            return Err(format!(
                "unknown extension placeholders: {}",
                missing.join(", ")
            ));
        }
    }
    let mut command = Command::new(&addon.exec);
    command.args(addon.args.iter().map(|arg| context.expand(arg)));
    if !addon.workdir.is_empty() {
        command.current_dir(context.expand(&addon.workdir));
    } else if let Some(instance) = context.values.get("instance") {
        command.current_dir(instance);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.process_group(0);
    let child = command.spawn().map_err(|e| e.to_string())?;
    let mut guard = ChildGuard {
        child,
        complete: false,
    };
    let mut stdout = guard
        .child
        .stdout
        .take()
        .ok_or("missing extension stdout")?;
    let mut stderr = guard
        .child
        .stderr
        .take()
        .ok_or("missing extension stderr")?;
    nonblocking(&stdout)?;
    nonblocking(&stderr)?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut out_closed = false;
    let mut err_closed = false;
    let mut status = None;
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("extension cancelled".into());
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "extension took longer than {timeout:?} and was stopped"
            ));
        }
        // One bounded read per pipe per iteration prevents a noisy stream from
        // starving cancellation or the other stream. No reader threads survive.
        let progress = drain(&mut stdout, &mut out, &mut out_closed)?
            | drain(&mut stderr, &mut err, &mut err_closed)?;
        if status.is_none() {
            status = guard.child.try_wait().map_err(|e| e.to_string())?;
        }
        if let Some(status) = status.filter(|_| out_closed && err_closed) {
            guard.complete = true;
            return Ok(Output {
                status,
                stdout: out,
                stderr: err,
            });
        }
        if !progress {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn nonblocking(fd: &impl AsFd) -> Result<(), String> {
    let flags = rustix::fs::fcntl_getfl(fd).map_err(|e| e.to_string())?;
    rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).map_err(|e| e.to_string())
}

fn drain(pipe: &mut impl Read, bytes: &mut Vec<u8>, closed: &mut bool) -> Result<bool, String> {
    if *closed {
        return Ok(false);
    }
    let mut buffer = [0; 8192];
    match pipe.read(&mut buffer) {
        Ok(0) => {
            *closed = true;
            Ok(true)
        }
        Ok(count) => {
            if bytes.len() + count > OUTPUT_LIMIT {
                return Err("extension output exceeded 1 MiB per stream".into());
            }
            bytes.extend_from_slice(&buffer[..count]);
            Ok(true)
        }
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(e.to_string()),
    }
}

struct ChildGuard {
    child: Child,
    complete: bool,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.complete {
            if let Some(pid) = rustix::process::Pid::from_raw(self.child.id() as i32) {
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shell(script: &str) -> Addon {
        let mut addon = crate::parse_addon(
            "id='capture'\nkind='diagnose'\nexec='/bin/sh'",
            Path::new("/tmp/capture.toml"),
        )
        .unwrap();
        addon.args = vec!["-c".into(), script.into()];
        addon
    }

    #[test]
    fn bounded_capture_handles_both_pipes_errors_and_cancellation() {
        let ctx = Context::default();
        let cancel = AtomicBool::new(false);
        let timeout = Duration::from_secs(2);
        let output = capture(
            &shell("printf output; printf error >&2; exit 7"),
            &ctx,
            timeout,
            &cancel,
        )
        .unwrap();
        assert_eq!(output.stdout, b"output");
        assert_eq!(output.stderr, b"error");
        assert_eq!(output.status.code(), Some(7));
        for pipe in ["", " >&2"] {
            let result = capture(
                &shell(&format!("head -c 1048577 /dev/zero{pipe}")),
                &ctx,
                timeout,
                &cancel,
            );
            assert!(result.unwrap_err().contains("exceeded"));
        }
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(40));
                cancel.store(true, Ordering::Relaxed);
            });
            let started = Instant::now();
            assert!(capture(&shell("sleep 3"), &ctx, timeout, &cancel)
                .unwrap_err()
                .contains("cancelled"));
            assert!(started.elapsed() < Duration::from_secs(1));
        });
        assert!(capture(&shell("exit 0"), &ctx, timeout, &cancel)
            .unwrap_err()
            .contains("cancelled"));
    }
}
