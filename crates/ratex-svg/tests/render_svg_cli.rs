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
fn parse_error_exits_one_and_reports_failed_svg() {
    let output_dir = TempDir::new("render-svg-cli-parse-error");
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-svg"))
        .arg("--output-dir")
        .arg(output_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn render-svg");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"$\\ce{Zn^2+  <=>[+ 2OH-][+ 2H+]  $\n")
        .expect("write formula");

    let output = child.wait_with_output().expect("wait for render-svg");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Processed 1 formula(s), wrote 0 SVG(s), failed 1."));
    assert!(stderr.contains("ERR    1"));
    assert!(!output_dir.path().join("0001.svg").exists());
}

#[test]
fn default_colors_stay_rgba_paint_values() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-svg"))
        .arg("--stdout")
        .arg("--color")
        .arg("red")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn render-svg");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"x\n")
        .expect("write formula");

    let output = child.wait_with_output().expect("wait for render-svg");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#"fill="rgba(255,0,0,1)""#), "{stdout}");
}

#[test]
fn office_compatible_colors_flag_emits_rgb_paint_values() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-svg"))
        .arg("--stdout")
        .arg("--office-compatible-colors")
        .arg("--color")
        .arg("red")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn render-svg");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"x\n")
        .expect("write formula");

    let output = child.wait_with_output().expect("wait for render-svg");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#"fill="rgb(255,0,0)""#), "{stdout}");
    assert!(!stdout.contains("rgba("), "{stdout}");
}

#[test]
fn office_compatible_colors_flag_preserves_alpha_with_opacity_attrs() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-svg"))
        .arg("--stdout")
        .arg("--office-compatible-colors")
        .arg("--color")
        .arg("#ff000080")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn render-svg");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"\\angl{x}\n")
        .expect("write formula");

    let output = child.wait_with_output().expect("wait for render-svg");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#"fill="rgb(255,0,0)" fill-opacity="0.501961""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"stroke="rgb(255,0,0)" stroke-opacity="0.501961""#),
        "{stdout}"
    );
    assert!(!stdout.contains("rgba("), "{stdout}");
}
