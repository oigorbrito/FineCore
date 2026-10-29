    #[test]
    fn bang_composer_keeps_the_original_unfilled_style() {
        let mut app = test_app();
        app.composer = "!echo hello".into();
        app.cursor = app.composer.chars().count();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(2, 22)].bg, ratatui::style::Color::Reset);
        assert_ne!(buffer[(0, 20)].symbol(), "┌");
    }

    #[test]
    fn empty_session_has_a_useful_static_welcome() {
        let mut app = App::new(TuiConfig {
            model_name: "gpt-oss:20b".into(),
            workspace_name: "/workspace/orca-harness".into(),
            workspace_root: "/test-ws".into(),
            provider: Provider::Local,
            subagent_depth: orca_harness_tools::SubagentDepth::new(1),
            stats: orca_harness_tools::BackgroundStats::new(),
            session_id: None,
            mcp: Default::default(),
            skills: Default::default(),
            mode: Default::default(),
            plan: Default::default(),
        });
        let screen = rendered_rows(&mut app, 90, 30).join("\n");

        assert!(screen.contains("▀▄ FINECORE"), "logo missing: {screen}");
        assert!(
            screen.contains(concat!("v", env!("CARGO_PKG_VERSION"))),
            "version missing: {screen}"
        );
        assert!(screen.contains("gpt-oss:20b"), "model missing: {screen}");
        assert!(
            screen.contains("/workspace/orca-harness"),
            "workspace missing: {screen}"
        );
        assert!(
            screen.contains("Describe a task to begin"),
            "welcome hint missing: {screen}"
        );
        let rows = rendered_rows(&mut app, 90, 30);
        let models = rows
            .iter()
            .position(|row| row.contains("/models    switch model"))
            .expect("hint row");
        assert!(
            rows[models + 1].contains("/mode      plan"),
            "hints share the label column: {:?}",
            rows[models + 1]
        );
        // Centred in the terminal, not pushed against the composer.
        let first = rows.iter().position(|row| row.contains("FINECORE")).unwrap();
        let last = rows.iter().rposition(|row| row.contains("/mode")).unwrap();
        assert!(first >= 8 && 30 - last >= 8, "card rows {first}..{last} of 30");
    }

    #[test]
    fn welcome_centres_the_visible_card_not_its_maximum_width() {
        let mut app = test_app();
        let rows = rendered_rows(&mut app, 90, 30);
        assert!(rows.iter().any(|row| row.contains("FineCore — your terminal coding agent")));
        let first = rows.iter().position(|row| row.contains("FINECORE")).unwrap();
        let last = rows.iter().rposition(|row| row.contains("/mode")).unwrap();
        let visible_width = rows[first..=last]
            .iter()
            .map(|row| row.trim_start().chars().count())
            .max()
            .unwrap();
        let visible_indent = rows[first].chars().take_while(|ch| *ch == ' ').count();

        assert_eq!(
            visible_indent,
            (90 - visible_width) / 2,
            "the longest visible row defines the card centre: {:?}",
            &rows[first..=last]
        );
    }

    #[test]
    fn startup_notices_stay_behind_the_welcome_until_the_first_turn() {
        let mut app = test_app();
        push_notice(&mut app, "MCP docs connected · 4 tools");
        app.absorb_pending();

        let screen = rendered_rows(&mut app, 90, 30).join("\n");

        assert!(screen.contains("▀▄ FINECORE"), "welcome missing: {screen}");
        assert!(
            !screen.contains("MCP docs connected"),
            "startup notice should remain in the background: {screen}"
        );
        assert!(
            flat_lines(&app.transcript).contains("MCP docs connected"),
            "startup notice should remain recorded"
        );
    }

    #[test]
    fn mcp_connecting_status_preserves_welcome_and_hides_startup_notices() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();
        assert!(!app.mcp_connecting);
        handle_ui_msg(&mut app, UiMsg::Notice("startup notice".into()), &tx, 90);
        app.absorb_pending();

        for connecting in [true, false] {
            handle_ui_msg(&mut app, UiMsg::McpConnecting(connecting), &tx, 90);
            assert_eq!(app.mcp_connecting, connecting);
            assert!(!app.welcome_dismissed);
            for width in [40, 90] {
                let screen = rendered_rows(&mut app, width, 30).join("\n");
                assert_eq!(screen.contains("MCP connecting · tools pending"), connecting, "{screen}");
                assert!(screen.contains("FINECORE"), "{screen}");
                assert!(!screen.contains("startup notice"), "{screen}");
            }
        }
        assert!(flat_lines(&app.transcript).contains("startup notice"));
        assert!(!flat_lines(&app.transcript).contains("tools pending"));
    }

    #[test]
    fn mcp_connecting_status_is_visible_in_normal_view_until_completion() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();
        app.welcome_dismissed = true;
        for connecting in [true, false] {
            handle_ui_msg(&mut app, UiMsg::McpConnecting(connecting), &tx, 90);
            let screen = rendered_rows(&mut app, 90, 30).join("\n");
            assert_eq!(screen.contains("MCP connecting · tools pending"), connecting, "{screen}");
            assert!(!screen.contains("FINECORE"), "{screen}");
        }
    }

    #[test]
    fn help_as_the_first_command_opens_the_picker_without_polluting_history() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();
        let transcript_before = flat_lines(&app.transcript);

        slash_command(&mut app, "help", &tx, 90);
        let screen = rendered_rows(&mut app, 90, 40).join("\n");

        assert!(matches!(app.overlay, Some(Overlay::Help { .. })));
        assert!(
            screen.contains("Help") && screen.contains("show available slash commands"),
            "help missing: {screen}"
        );
        assert!(
            !screen.contains("▀▄ FINECORE"),
            "welcome remained: {screen}"
        );
        assert!(app.pending_history.is_empty());
        assert_eq!(flat_lines(&app.transcript), transcript_before);
        assert_eq!(app.turn_count, 0, "local help is not a model turn");
    }

    #[test]
    fn help_picker_places_the_selected_command_in_the_composer() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();

        slash_command(&mut app, "help", &tx, 90);
        press(&mut app, &tx, KeyCode::Down);
        press(&mut app, &tx, KeyCode::Enter);

        assert!(app.overlay.is_none());
        assert_eq!(app.composer, "/hotkeys");
        assert_eq!(app.cursor, app.composer.chars().count());
    }

    #[test]
    fn hotkeys_lists_registered_shortcuts_without_starting_a_turn() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();
        let transcript_before = flat_lines(&app.transcript);

        slash_command(&mut app, "hotkeys", &tx, 120);
        let shown = flat_lines(&app.pending_history);

        assert!(app.overlay.is_none(), "hotkeys writes to the transcript");
        for expected in [
            "hotkeys",
            "Shift+Tab",
            "cycle normal, plan, orchestrate, auto, and yolo modes",
            "Ctrl+O",
            "expand the latest work or tool output",
            "Ctrl+Y",
            "Backspace / Delete",
            "Tab / Shift+Tab",
            "d / t",
            "n / N / Esc / Ctrl+C",
        ] {
            assert!(shown.contains(expected), "missing {expected:?}: {shown}");
        }
        assert_eq!(flat_lines(&app.transcript), transcript_before);
        assert!(!app.pending_history.is_empty());
        assert_eq!(app.turn_count, 0, "local hotkeys is not a model turn");
    }

    #[test]
    fn help_picker_filters_without_writing_to_the_transcript() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = test_app();
        let transcript_before = flat_lines(&app.transcript);

        slash_command(&mut app, "help", &tx, 90);
        for character in "mode".chars() {
            press(&mut app, &tx, KeyCode::Char(character));
        }
        let shown = flat_lines(&live_lines(&app, 100));

        assert!(shown.contains("filter: mode"), "{shown}");
        assert!(shown.contains("/mode"), "{shown}");
        assert!(!shown.contains("/help"), "{shown}");
        assert_eq!(flat_lines(&app.transcript), transcript_before);
    }
