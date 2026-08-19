use std::sync::Arc;

#[test]
fn font_bytes_remains_a_public_arc_vec_alias() {
    let bytes: ratex_font_loader::FontBytes = Arc::new(vec![0, 1, 2, 3]);
    assert_eq!(bytes.as_slice(), [0, 1, 2, 3]);
}
