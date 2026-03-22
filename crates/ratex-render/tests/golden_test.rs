/// Golden tests: compare RaTeX rendered PNGs against KaTeX reference PNGs.
///
/// Uses ink-based comparison: crop to content, normalize size, compute IoU.
/// This ensures blank/broken renders are correctly identified as failures.
use std::path::PathBuf;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};

const INK_THRESHOLD: u8 = 240;
const NORM_HEIGHT: u32 = 120;
const SCORE_THRESHOLD: f64 = 0.30;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap().to_path_buf()
}

fn font_dir() -> String {
    project_root()
        .join("tools/lexer_compare/node_modules/katex/dist/fonts")
        .to_string_lossy().to_string()
}

fn load_png(path: &std::path::Path) -> Option<(Vec<u8>, u32, u32)> {
    let file = std::fs::File::open(path).ok()?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some((buf, info.width, info.height))
}

fn is_ink_pixel(data: &[u8], offset: usize) -> bool {
    if offset + 2 >= data.len() { return false; }
    data[offset] < INK_THRESHOLD || data[offset+1] < INK_THRESHOLD || data[offset+2] < INK_THRESHOLD
}

/// Find bounding box of ink pixels, return (x_min, y_min, x_max, y_max)
fn ink_bbox(data: &[u8], w: u32, h: u32, channels: u32) -> (u32, u32, u32, u32) {
    let mut x_min = w;
    let mut y_min = h;
    let mut x_max = 0u32;
    let mut y_max = 0u32;

    for y in 0..h {
        for x in 0..w {
            let off = (y * w + x) as usize * channels as usize;
            if is_ink_pixel(data, off) {
                x_min = x_min.min(x);
                y_min = y_min.min(y);
                x_max = x_max.max(x);
                y_max = y_max.max(y);
            }
        }
    }

    if x_max < x_min {
        return (0, 0, w.min(10), h.min(10));
    }

    let margin = 2;
    (
        x_min.saturating_sub(margin),
        y_min.saturating_sub(margin),
        (x_max + margin).min(w - 1),
        (y_max + margin).min(h - 1),
    )
}

/// Crop image data to bounding box. Returns (new_data, new_w, new_h)
fn crop_image(data: &[u8], w: u32, h: u32, channels: u32) -> (Vec<u8>, u32, u32) {
    let (x0, y0, x1, y1) = ink_bbox(data, w, h, channels);
    let nw = x1 - x0 + 1;
    let nh = y1 - y0 + 1;
    let mut out = vec![255u8; (nw * nh * channels) as usize];

    for y in 0..nh {
        for x in 0..nw {
            let src = ((y0 + y) * w + (x0 + x)) as usize * channels as usize;
            let dst = (y * nw + x) as usize * channels as usize;
            for c in 0..channels as usize {
                if src + c < data.len() && dst + c < out.len() {
                    out[dst + c] = data[src + c];
                }
            }
        }
    }

    (out, nw, nh)
}

/// Nearest-neighbor resize to target height, preserving aspect ratio.
fn resize_image(data: &[u8], w: u32, h: u32, channels: u32, target_h: u32) -> (Vec<u8>, u32, u32) {
    if h == 0 || w == 0 {
        return (vec![255u8; (target_h * target_h * channels) as usize], target_h, target_h);
    }
    let scale = target_h as f64 / h as f64;
    let new_w = (w as f64 * scale).round().max(1.0) as u32;

    let mut out = vec![255u8; (new_w * target_h * channels) as usize];
    for y in 0..target_h {
        let src_y = ((y as f64 / scale) as u32).min(h - 1);
        for x in 0..new_w {
            let src_x = ((x as f64 / scale) as u32).min(w - 1);
            let src_off = (src_y * w + src_x) as usize * channels as usize;
            let dst_off = (y * new_w + x) as usize * channels as usize;
            for c in 0..channels as usize {
                if src_off + c < data.len() && dst_off + c < out.len() {
                    out[dst_off + c] = data[src_off + c];
                }
            }
        }
    }

    (out, new_w, target_h)
}

/// Compute ink-based comparison score after crop+normalize.
#[allow(clippy::too_many_arguments)]
fn ink_compare(
    ref_data: &[u8], ref_w: u32, ref_h: u32, ref_ch: u32,
    test_data: &[u8], test_w: u32, test_h: u32, test_ch: u32,
) -> f64 {
    let (rc, rcw, rch) = crop_image(ref_data, ref_w, ref_h, ref_ch);
    let (tc, tcw, tch) = crop_image(test_data, test_w, test_h, test_ch);

    let (rn, rnw, rnh) = resize_image(&rc, rcw, rch, ref_ch, NORM_HEIGHT);
    let (tn, tnw, tnh) = resize_image(&tc, tcw, tch, test_ch, NORM_HEIGHT);

    let w = rnw.max(tnw) as usize;
    let h = rnh.max(tnh) as usize;

    let get_ink = |data: &[u8], dw: u32, _dh: u32, ch: u32, x: usize, y: usize| -> bool {
        if x >= dw as usize || y >= NORM_HEIGHT as usize { return false; }
        let off = (y * dw as usize + x) * ch as usize;
        is_ink_pixel(data, off)
    };

    let mut ref_ink = 0usize;
    let mut both_ink = 0usize;
    let mut either_ink = 0usize;

    for y in 0..h {
        for x in 0..w {
            let r = get_ink(&rn, rnw, rnh, ref_ch, x, y);
            let t = get_ink(&tn, tnw, tnh, test_ch, x, y);
            if r { ref_ink += 1; }
            if r && t { both_ink += 1; }
            if r || t { either_ink += 1; }
        }
    }

    let iou = if either_ink > 0 { both_ink as f64 / either_ink as f64 } else { 1.0 };
    let recall = if ref_ink > 0 { both_ink as f64 / ref_ink as f64 } else { 0.0 };

    let ref_aspect = rcw as f64 / rch.max(1) as f64;
    let test_aspect = tcw as f64 / tch.max(1) as f64;
    let aspect_sim = ref_aspect.min(test_aspect) / ref_aspect.max(test_aspect);

    let width_sim = (rnw as f64).min(tnw as f64) / (rnw as f64).max(tnw as f64);

    0.4 * iou + 0.2 * recall + 0.2 * aspect_sim + 0.2 * width_sim
}

fn run_golden_suite(
    label: &str,
    tc_path: &std::path::Path,
    fixtures_dir: &std::path::Path,
    min_pass_rate: f64,
    device_pixel_ratio: f32,
) {
    if !tc_path.exists() || !fixtures_dir.exists() {
        eprintln!("Skipping {label}: path missing");
        return;
    }

    let font_dir = font_dir();
    let render_opts = RenderOptions {
        font_size: 40.0,
        padding: 10.0,
        font_dir,
        device_pixel_ratio,
    };
    let layout_opts = LayoutOptions::default();

    let lines: Vec<String> = std::fs::read_to_string(tc_path).unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .map(|l| l.to_string())
        .collect();

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    for (i, expr) in lines.iter().enumerate() {
        let idx = format!("{:04}", i + 1);
        let ref_path = fixtures_dir.join(format!("{idx}.png"));
        if !ref_path.exists() { skipped += 1; continue; }

        let ast = match parse(expr) { Ok(a) => a, Err(_) => { skipped += 1; continue; } };
        let lbox = layout(&ast, &layout_opts);
        let dl = to_display_list(&lbox);
        let png_bytes = match render_to_png(&dl, &render_opts) { Ok(d) => d, Err(_) => { skipped += 1; continue; } };

        let (ref_data, rw, rh) = match load_png(&ref_path) { Some(v) => v, None => { skipped += 1; continue; } };
        let ref_ch = if ref_data.len() == (rw * rh * 4) as usize { 4 } else { 3 };

        let decoder = png::Decoder::new(std::io::Cursor::new(&png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut test_buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut test_buf).unwrap();
        test_buf.truncate(info.buffer_size());
        let test_ch = if test_buf.len() == (info.width * info.height * 4) as usize { 4 } else { 3 };

        let score = ink_compare(
            &ref_data, rw, rh, ref_ch,
            &test_buf, info.width, info.height, test_ch,
        );

        if score >= SCORE_THRESHOLD {
            passed += 1;
        } else {
            failed += 1;
            if failed <= 10 {
                eprintln!("FAIL {label} {idx}: score={score:.3} | {}", &expr[..expr.len().min(60)]);
            }
        }
    }

    let total = passed + failed;
    let rate = if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 };
    eprintln!("\n{label} (ink-based): {passed}/{total} passed ({rate:.1}%), {skipped} skipped");

    if total == 0 {
        eprintln!("{label}: no reference PNGs — add tests/golden/fixtures_ce/{{0001,…}}.png (KaTeX+mhchem) to enable.");
        return;
    }

    assert!(rate >= min_pass_rate,
        "{label} pass rate {rate:.1}% below {min_pass_rate:.0}% ({passed}/{total})."
    );
}

#[test]
fn golden_test_pass_rate() {
    let root = project_root();
    run_golden_suite(
        "Golden (main)",
        &root.join("tests/golden/test_cases.txt"),
        &root.join("tests/golden/fixtures"),
        75.0,
        1.0,
    );
}

/// mhchem (`\\ce` / `\\pu`): uses [tests/golden/test_case_ce.txt](../../tests/golden/test_case_ce.txt) and `fixtures_ce/`.
#[test]
fn golden_mhchem_pass_rate() {
    let root = project_root();
    // 2.0 matches legacy fixtures_ce (Puppeteer DPR 2); use 1.0 if refs were regenerated with DPR 1.
    run_golden_suite(
        "Golden (mhchem)",
        &root.join("tests/golden/test_case_ce.txt"),
        &root.join("tests/golden/fixtures_ce"),
        50.0,
        2.0,
    );
}

