from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one replacement target, found {count}")
    p.write_text(text.replace(old, new, 1))


path = "crates/ratex-pdf/src/fonts.rs"
replace_once(
    path,
    "pub(crate) fn embed_emoji_rasters(\n",
    "#[allow(clippy::chunks_exact_to_as_chunks)]\npub(crate) fn embed_emoji_rasters(\n",
)
replace_once(
    path,
    "fn decode_png_rgba8(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {",
    "#[allow(clippy::chunks_exact_to_as_chunks)]\nfn decode_png_rgba8(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {",
)
