// Integration tests for the interactive `/skills` dialog panel. This file is
// `include!`d into `mod tests`, so it must not carry an inner doc comment or
// re-import names (`KeyCode`, `KeyModifiers`) that the including module
// already brings into scope.
#[test]
fn slash_skills_opens_interactive_panel_and_skills_text_keeps_plain_report() {
    let mut app = create_test_app();

    // `/skills` opens the picker and does not push a report message.
    assert!(super::state_ui::handle_info_command(&mut app, "/skills"));
    assert!(
        app.skills_picker_overlay.is_some(),
        "/skills must open the interactive panel"
    );
    assert_eq!(
        app.display_messages()
            .iter()
            .filter(|message| message.title.as_deref() == Some("Skills"))
            .count(),
        0,
        "/skills should not push a plain-text report anymore"
    );

    // Dismiss and confirm the hidden `/skills-text` fallback still reports.
    app.skills_picker_overlay = None;
    assert!(super::state_ui::handle_info_command(&mut app, "/skills-text"));
    let content = app.display_messages().last().unwrap().content.clone();
    assert!(content.contains("Loaded skills"), "{content}");
    assert!(
        content.contains("Endorsed skills (recommended by jcode)"),
        "{content}"
    );
}

#[test]
fn slash_skills_panel_enter_activates_loaded_skill() {
    let mut app = create_test_app();
    let temp = tempfile::tempdir().expect("tempdir");
    let skill_dir = temp.path().join(".jcode/skills/panel-skill");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: panel-skill\ndescription: Panel activation regression skill\n---\nUse it.\n",
    )
    .expect("write SKILL.md");
    app.session.working_dir = Some(temp.path().to_string_lossy().to_string());

    assert!(super::state_ui::handle_info_command(&mut app, "/skills"));
    assert!(app.skills_picker_overlay.is_some());

    // Type the filter until the slot is the only match.
    for ch in "panel-skill".chars() {
        let command = app
            .next_skills_picker_action(KeyCode::Char(ch), KeyModifiers::empty())
            .expect("filter key handled");
        assert!(command.is_none(), "typing should not execute anything");
    }

    let command = app
        .next_skills_picker_action(KeyCode::Enter, KeyModifiers::empty())
        .expect("enter handled");
    assert!(
        app.skills_picker_overlay.is_none(),
        "executing a command must close the panel"
    );
    let command = command.expect("enter should activate the loaded skill");
    assert!(matches!(
        &command,
        crate::tui::skill_picker::SkillPickerCommand::Activate { name } if name == "panel-skill"
    ));

    app.handle_skills_picker_command(command);
    assert_eq!(app.active_skill.as_deref(), Some("panel-skill"));
    let last = app.display_messages().last().expect("activation message");
    assert_eq!(last.role, "system");
    assert!(last.content.contains("Activated skill: panel-skill"), "{:?}", last.content);
}

#[test]
fn skills_panel_copy_command_writes_install_text_to_clipboard() {
    const INSTALL: &str = "npx skills add nvidia/skills --skill cuopt-developer --yes";
    let mut app = create_test_app();

    crate::tui::app::helpers::capture_clipboard_for_tests();
    app.handle_skills_picker_command(crate::tui::skill_picker::SkillPickerCommand::Copy {
        text: INSTALL.to_string(),
    });
    assert_eq!(
        crate::tui::app::helpers::captured_clipboard_for_tests().as_deref(),
        Some(INSTALL)
    );
    let notice = app.status_notice.as_ref().expect("status notice set");
    assert!(notice.0.contains("copied to clipboard"), "{:?}", notice.0);
    crate::tui::app::helpers::stop_capturing_clipboard_for_tests();
}

#[test]
fn skills_panel_esc_closes_without_executing() {
    let mut app = create_test_app();
    assert!(super::state_ui::handle_info_command(&mut app, "/skills"));
    assert!(app.skills_picker_overlay.is_some());

    let command = app
        .next_skills_picker_action(KeyCode::Esc, KeyModifiers::empty())
        .expect("esc handled");
    assert!(command.is_none());
    assert!(
        app.skills_picker_overlay.is_none(),
        "Esc should close the panel"
    );
    assert!(!app.is_processing, "closing the panel runs nothing");
}