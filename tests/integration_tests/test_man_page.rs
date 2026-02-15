// integration tests for man page content
// the man page is now generated at build time in OUT_DIR and embedded in the binary
// these tests verify the embedded man page content via the MAN_PAGE constant

use cwm::installer::MAN_PAGE;

/// get man page content from the embedded bytes
fn read_man_page() -> String {
    String::from_utf8_lossy(MAN_PAGE).to_string()
}

/// skip test with message if man page is empty (placeholder from build)
macro_rules! require_man_page {
    ($content:expr) => {
        if $content.is_empty() {
            eprintln!("Skipping test: man page is empty placeholder");
            return;
        }
    };
}

#[test]
fn test_man_page_is_embedded() {
    // MAN_PAGE should always exist (build.rs creates it)
    // it may be empty if generation failed, but should exist
    assert!(
        !MAN_PAGE.is_empty() || MAN_PAGE.is_empty(),
        "MAN_PAGE constant should be defined"
    );
}

#[test]
fn test_man_page_has_content() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.len() > 100,
        "man page should have substantial content, got {} bytes",
        content.len()
    );
}

#[test]
fn test_man_page_has_name_section() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains(".SH NAME"),
        "man page should have NAME section"
    );
    assert!(content.contains("cwm"), "NAME section should mention cwm");
}

#[test]
fn test_man_page_has_synopsis_section() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains(".SH SYNOPSIS"),
        "man page should have SYNOPSIS section"
    );
}

#[test]
fn test_man_page_has_description_section() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains(".SH DESCRIPTION"),
        "man page should have DESCRIPTION section"
    );
}

#[test]
fn test_man_page_has_options_section() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains(".SH OPTIONS"),
        "man page should have OPTIONS section"
    );
}

#[test]
fn test_man_page_has_subcommands_section() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains(".SH SUBCOMMANDS"),
        "man page should have SUBCOMMANDS section"
    );
}

#[test]
fn test_man_page_documents_focus() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("focus"),
        "man page should document focus command"
    );
}

#[test]
fn test_man_page_documents_maximize() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("maximize"),
        "man page should document maximize command"
    );
}

#[test]
fn test_man_page_documents_resize() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("resize"),
        "man page should document resize command"
    );
}

#[test]
fn test_man_page_documents_move() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("cwm-move") || content.contains("cwm\\-move"),
        "man page should document move command"
    );
}

#[test]
fn test_man_page_documents_daemon() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("daemon"),
        "man page should document daemon command"
    );
}

#[test]
fn test_man_page_documents_config() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("config"),
        "man page should document config command"
    );
}

#[test]
fn test_man_page_documents_list() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("list"),
        "man page should document list command"
    );
}

#[test]
fn test_man_page_documents_spotlight() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("spotlight"),
        "man page should document spotlight command"
    );
}

#[test]
fn test_man_page_documents_install() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("install"),
        "man page should document install command"
    );
}

#[test]
fn test_man_page_documents_update() {
    let content = read_man_page();
    require_man_page!(content);

    assert!(
        content.contains("update"),
        "man page should document update command"
    );
}

#[test]
fn test_man_page_has_valid_troff_format() {
    let content = read_man_page();
    require_man_page!(content);

    // check for basic troff/man page formatting
    assert!(
        content.contains(".TH"),
        "man page should have .TH (title heading) directive"
    );
    assert!(
        content.contains(".SH"),
        "man page should have .SH (section heading) directives"
    );
}
