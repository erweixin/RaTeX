use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ratex-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp output dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn parse_error_exits_one_and_reports_failed_pdf() {
    let output_dir = TempDir::new("render-pdf-cli-parse-error");
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-pdf"))
        .arg("--output-dir")
        .arg(output_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn render-pdf");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"$\\ce{Zn^2+  <=>[+ 2OH-][+ 2H+]  $\n")
        .expect("write formula");

    let output = child.wait_with_output().expect("wait for render-pdf");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Processed 1 formula(s), wrote 0 PDF(s), failed 1."));
    assert!(stderr.contains("ERR    1"));
    assert!(!output_dir.path().join("0001.pdf").exists());
}
