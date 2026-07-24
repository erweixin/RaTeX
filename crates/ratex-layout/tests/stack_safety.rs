use std::process::Command;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;

const CHILD_ENV: &str = "RATEX_STACK_SAFETY_CHILD";

fn nested(prefix: &str, suffix: &str, depth: usize) -> String {
    format!("{}x{}", prefix.repeat(depth), suffix.repeat(depth))
}

fn nested_fraction(depth: usize) -> String {
    format!(r"{}x{}", r"\frac{1}{".repeat(depth), "}".repeat(depth))
}

fn nested_superscript(depth: usize) -> String {
    format!(r"{}x{}", r"x^{".repeat(depth), "}".repeat(depth))
}

fn unbraced_command_chain(command: &str, depth: usize) -> String {
    format!("{}x", command.repeat(depth))
}

fn unary_prooftree(inferences: usize) -> String {
    format!(
        r"\begin{{prooftree}}\AxiomC{{P}}{}\end{{prooftree}}",
        r"\UnaryInfC{P}".repeat(inferences)
    )
}

fn assert_pipeline_ok(input: &str) {
    let ast = parse(input).unwrap_or_else(|error| panic!("failed to parse bounded input: {error}"));
    let layout = layout(&ast, &LayoutOptions::default());
    let _ = to_display_list(&layout);
}

fn assert_recursion_limit(input: &str) {
    let error = parse(input).expect_err("over-limit input unexpectedly parsed");
    assert!(
        error.to_string().contains("Recursion limit exceeded"),
        "unexpected error: {error}"
    );
}

fn assert_parse_error(input: &str) {
    parse(input).expect_err("adversarial input unexpectedly parsed");
}

fn run_small_stack_cases() {
    // Debug parser frames are intentionally much larger than production
    // frames. Verify the supported boundary on the constrained stack in the
    // release configuration used by platform bindings.
    #[cfg(not(debug_assertions))]
    run_boundary_cases();

    let over_limit_cases = [
        nested("{", "}", 33),
        nested(r"\sqrt{", "}", 33),
        nested_fraction(33),
        nested(r"\left(", r"\right)", 33),
        nested_superscript(33),
    ];
    for input in &over_limit_cases {
        assert_recursion_limit(input);
    }

    let adversarial_cases = [
        nested("{", "}", 300),
        nested(r"\sqrt{", "}", 300),
        nested_fraction(300),
        nested(r"\left(", r"\right)", 300),
        nested_superscript(300),
    ];
    for input in &adversarial_cases {
        assert_recursion_limit(input);
    }

    let accents_4200 = "\u{301}".repeat(4_200);
    assert_recursion_limit(&format!("x{accents_4200}"));
    assert_recursion_limit(&format!(r"\text{{x{accents_4200}}}"));
    assert_recursion_limit(&format!(r"\text{{x{}}}", "\u{301}".repeat(32)));
    assert_recursion_limit(&unary_prooftree(32));

    // Unbraced primitive/function arguments must also terminate with a regular
    // parse error. This guards against accidentally turning command chains such
    // as `\sqrt\sqrt...x` into an unbounded parse_group -> parse_function path
    // that bypasses the expression-depth counter.
    assert_parse_error(&unbraced_command_chain(r"\sqrt", 4_200));
}

fn run_boundary_cases() {
    let boundary_cases = [
        ("group", nested("{", "}", 32)),
        ("sqrt", nested(r"\sqrt{", "}", 32)),
        ("frac", nested_fraction(32)),
        ("left-right", nested(r"\left(", r"\right)", 32)),
        ("superscript", nested_superscript(32)),
    ];
    for (_name, input) in &boundary_cases {
        assert_pipeline_ok(input);
    }

    let accents_32 = "\u{301}".repeat(32);
    let accents_31 = "\u{301}".repeat(31);
    assert_pipeline_ok(&format!("x{accents_32}"));
    assert_pipeline_ok(&format!(r"\text{{x{accents_31}}}"));

    assert_pipeline_ok(&unary_prooftree(31));
}

#[test]
fn parser_driven_pipeline_is_safe_on_a_small_stack() {
    if std::env::var_os(CHILD_ENV).is_some() {
        std::thread::Builder::new()
            .name("ratex-512k-stack".into())
            .stack_size(512 * 1024)
            .spawn(run_small_stack_cases)
            .expect("failed to spawn small-stack thread")
            .join()
            .expect("small-stack test thread panicked");
        return;
    }

    run_boundary_cases();

    let status = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("parser_driven_pipeline_is_safe_on_a_small_stack")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .status()
        .expect("failed to start isolated stack-safety test process");

    assert!(status.success(), "isolated stack-safety process failed");
}
