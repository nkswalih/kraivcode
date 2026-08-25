// Issue #544: stray Enter after a bracketed paste must not submit.
#[test]
fn bare_enter_immediately_after_paste_does_not_submit() {
    // Windows Terminal / conhost sends a separate bare Enter key event after
    // a bracketed paste ending with \n; it must not submit the chat (#544).
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = create_test_app();
    crate::tui::app::input::handle_paste(&mut app, "hello world\n".to_string());
    assert_eq!(app.input, "[Pasted ~2 lines]");

    app.handle_key_press_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(
        !app.is_processing,
        "Enter right after paste must not submit"
    );
    assert!(
        !app.input.is_empty(),
        "input should be preserved after paste"
    );

    // A later, human-timed Enter still submits.
    crate::tui::app::input::paste_guard_expire_for_test();
    app.handle_key_press_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.input.is_empty(), "later Enter should submit normally");
}

// Keymap-independent multi-line input: a trailing backslash makes Enter insert
// a newline instead of submitting.
//
// Motivation: both newline chords (Shift+Enter, Option/Alt+Enter) require the
// terminal to disambiguate modified Enter via the kitty keyboard protocol.
// macOS Terminal.app and default-profile iTerm2 send a bare CR instead, so
// those users previously had no way to type a multi-line prompt.

#[test]
fn trailing_backslash_enter_inserts_newline_instead_of_submitting() {
    let mut app = create_test_app();
    app.set_input_for_test("first\\");
    app.cursor_pos = app.input.len();

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter should be handled");

    assert_eq!(app.input, "first\n");
    assert_eq!(app.cursor_pos, app.input.len());
    assert!(
        app.display_messages().is_empty(),
        "the draft must not be submitted: {:?}",
        app.display_messages()
    );
}

#[test]
fn plain_enter_still_submits_without_a_trailing_backslash() {
    let mut app = create_test_app();
    app.set_input_for_test("first");
    app.cursor_pos = app.input.len();

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter should be handled");

    assert!(app.input.is_empty(), "input should be consumed by submit");
}

#[test]
fn escaped_double_backslash_is_literal_text_and_still_submits() {
    let mut app = create_test_app();
    app.set_input_for_test("path\\\\");
    app.cursor_pos = app.input.len();

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter should be handled");

    assert!(
        app.input.is_empty(),
        "an escaped backslash is literal, so Enter submits: {:?}",
        app.input
    );
}

#[test]
fn backslash_continuation_only_applies_at_the_end_of_the_draft() {
    let mut app = create_test_app();
    app.set_input_for_test("a\\b");
    app.cursor_pos = 2; // just after the backslash, not at end

    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter should be handled");

    assert!(
        app.input.is_empty(),
        "mid-draft backslash must not become a continuation: {:?}",
        app.input
    );
}

#[test]
fn repeated_continuations_build_a_multi_line_draft() {
    let mut app = create_test_app();
    for line in ["one\\", "two\\"] {
        for c in line.chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
                .expect("char should be handled");
        }
        // Pace the Enter like a human: in-process calls land microseconds
        // apart, which the paste-burst detector classifies as terminal
        // injection rather than an intentional newline.
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.handle_key(KeyCode::Enter, KeyModifiers::empty())
            .expect("enter should be handled");
    }
    for c in "three".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .expect("char should be handled");
    }

    assert_eq!(app.input, "one\ntwo\nthree");
}

// Terminals without bracketed-paste support inject multiline clipboard
// content as raw key events whose \r bytes arrive as an Enter storm. Those
// injected Enters must reconstruct newlines in the composer, never submit
// the draft mid-paste.
#[test]
fn injected_enter_storm_inserts_newlines_instead_of_submitting() {
    let mut app = create_test_app();
    crate::tui::app::input::paste_burst_reset_for_test();

    // Simulate conhost-style injection: dense printable keys interleaved
    // with Enters, all landing with sub-millisecond gaps.
    for line in ["alpha", "beta", "gamma", "delta"] {
        for c in line.chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
                .expect("char should be handled");
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::empty())
            .expect("enter should be handled");
    }

    assert!(
        !app.is_processing,
        "injected Enters must never submit the draft"
    );
    assert_eq!(app.input, "alpha\nbeta\ngamma\ndelta");
}

#[test]
fn human_paced_enter_after_typing_still_submits() {
    let mut app = create_test_app();
    crate::tui::app::input::paste_burst_reset_for_test();

    for c in "hi there".chars() {
        app.handle_key(KeyCode::Char(c), KeyModifiers::empty())
            .expect("char should be handled");
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    // Human pause before committing: far above the 12 ms injection gap.
    std::thread::sleep(std::time::Duration::from_millis(40));
    app.handle_key(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter should be handled");

    assert!(
        app.input.is_empty(),
        "a human-paced Enter must submit normally"
    );
}

#[test]
fn raw_multiline_paste_recovery_collapses_and_preserves_full_text() {
    let mut app = create_test_app();
    crate::tui::app::input::paste_burst_reset_for_test();
    app.set_input_for_test("line 1");
    app.cursor_pos = app.input.len();

    assert!(crate::tui::app::input::recover_raw_multiline_paste_with_text(
        &mut app,
        "line 1\n\nline 2\nline 3".to_string(),
    ));
    assert_eq!(app.input, "[Pasted ~4 lines]");
    assert_eq!(app.pasted_contents, vec!["line 1\n\nline 2\nline 3"]);
    let visible = app.input.clone();
    assert_eq!(
        crate::tui::app::input::expand_paste_placeholders(&mut app, &visible),
        "line 1\n\nline 2\nline 3"
    );
    assert!(!app.is_processing);
    crate::tui::app::input::paste_burst_reset_for_test();
}
