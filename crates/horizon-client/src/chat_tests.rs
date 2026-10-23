use super::*;
use crate::{font::Font, hud::Rect};
use std::{collections::HashMap, path::Path};

fn font() -> Font {
    // The missing-glyph metrics let these tests run without game assets.
    Font::load(
        Path::new("/nonexistent-horizon-chat-test-resources"),
        &HashMap::new(),
        &[],
        (1, 1),
        (0, 0),
        Rect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            w: 1,
            h: 1,
        },
    )
}

fn add(chat: &mut Chat, font: &Font, content: Text) {
    for content in font.wrap_components(&content, chat.max_width()) {
        chat.lines.insert(
            0,
            Line {
                added: 0,
                content,
                indicator: None,
                head: None,
                picture: None,
                sender: None,
                indent: 0,
            },
        );
    }
}

#[test]
fn clicking_seed_copies_the_raw_value_only_inside_the_link() {
    let font = font();
    for seed in [1, -42, i64::MIN, i64::MAX] {
        let mut chat = Chat::default();
        let seed_text = Text::copyable(seed.to_string()).with_color(text::GOLD);
        assert_eq!(seed_text.plain(), format!("[{seed}]"));
        assert_eq!(seed_text.spans[0].style.color, Some(text::GOLD));
        assert_eq!(seed_text.spans[1].style.color, Some(text::GREEN));
        assert_eq!(seed_text.spans[2].style.color, Some(text::GOLD));
        add(&mut chat, &font, Text::literal("Seed: ").append(seed_text));
        let expected = Some(ClickEvent::CopyToClipboard(seed.to_string()));
        let start = 4.0 + f64::from(font.width("Seed: ["));
        let end = start + f64::from(font.width(&seed.to_string()));
        assert_eq!(chat.click_at(&font, 360, start, 315.0), expected);
        assert_eq!(chat.click_at(&font, 360, end - 0.5, 315.0), expected);
        for (x, y) in [
            (start - 0.5, 315.0),
            (end, 315.0),
            (3.0, 315.0),
            (start, 320.0),
            (start, 310.0),
        ] {
            assert_eq!(chat.click_at(&font, 360, x, y), None, "{x}, {y}");
        }
    }
}

#[test]
fn click_targets_match_rendered_spans_when_scaled_wrapped_and_scrolled() {
    let font = font();
    let screenshot = ClickEvent::OpenFile("/tmp/My screenshots/世界.png".into());
    for scale in [0.5, 0.75, 1.0] {
        for line_spacing in [0.0, 0.5, 1.0] {
            let mut chat = Chat::default();
            chat.options.scale = scale;
            chat.options.width = 0.0;
            chat.options.line_spacing = line_spacing;
            for _ in 0..20 {
                add(
                    &mut chat,
                    &font,
                    Text::literal("Saved screenshot as ").append(Text::styled(
                        "2026-10-14_18.55.14.png",
                        Style {
                            click: Some(screenshot.clone()),
                            bold: Some(true),
                            underlined: Some(true),
                            ..Style::default()
                        },
                    )),
                );
            }
            for scroll in [0, 3, 12] {
                chat.scroll = scroll;
                let mut links = 0;
                for draw in chat.layout(361, 0, true) {
                    if let GuiDraw::Text { text, x, y, .. } = draw {
                        let mut at = x as f64;
                        for (c, style) in text.chars() {
                            let width = f64::from(font.advance(c, style));
                            let cursor_x = (4.0 + at + width / 2.0) * scale;
                            let cursor_y = (y as f64 + 3.0) * scale;
                            assert_eq!(
                                chat.click_at(&font, 361, cursor_x, cursor_y),
                                style.click,
                                "scale={scale}, spacing={line_spacing}, scroll={scroll}, char={c}"
                            );
                            links += usize::from(style.click.is_some());
                            at += width;
                        }
                    }
                }
                assert!(links > 0);
                // The history above the focused viewport is not clickable.
                let top = ((361 - 40) as f64 / scale).floor()
                    - chat.lines_per_page(true) as f64 * chat.options.line_height() as f64;
                assert_eq!(
                    chat.click_at(&font, 361, 10.0 * scale, (top - 1.0) * scale),
                    None
                );
            }
        }
    }
}

#[test]
fn locate_coordinates_fill_chat_with_teleport_without_submitting() {
    let font = font();
    let m = Measure(&font);
    let registries = Registries::default();
    let commands = Commands {
        dispatcher: crate::command::vanilla::game_dispatcher(),
        registries: &registries,
    };
    // Structures and POIs use relative Y; biome results include absolute Y.
    for (y, expected) in [
        (None, "/tp @s -1136 ~ 848"),
        (Some(73), "/tp @s -1136 73 848"),
    ] {
        let link = Text::teleport_link(-1136, y, 848);
        assert_eq!(link.component()["click_event"]["action"], "suggest_command");
        assert_eq!(link.component()["click_event"]["command"], expected);
        let mut chat = Chat::default();
        add(&mut chat, &font, Text::literal("Found at ").append(link));
        let mut screen = ChatScreen::new(&m, &commands, "draft", 0, 640, 360);
        let x = 4.0 + f64::from(font.width("Found at "));
        assert!(!screen.click(&m, &commands, x, 315.0));
        let Some(ClickEvent::SuggestCommand(command)) = chat.click_at(&font, 360, x, 315.0) else {
            panic!("locate coordinates must suggest a command");
        };
        screen.suggest_command(&m, &commands, &command);
        assert_eq!(screen.value(), expected);
        assert_eq!(screen.input.cursor, expected.chars().count());
        assert_eq!(screen.input.highlight, screen.input.cursor);
        // Submission is a separate, explicit Enter press.
        assert_eq!(
            screen.key(
                &m,
                &commands,
                &[],
                winit::keyboard::KeyCode::Enter,
                false,
                false
            ),
            ChatAction::Submit(expected.to_owned())
        );
    }
}

#[test]
fn history_arrows_recall_messages_and_restore_the_draft() {
    use winit::keyboard::KeyCode::{ArrowDown, ArrowUp};

    let font = font();
    let m = Measure(&font);
    let registries = Registries::default();
    let commands = Commands {
        dispatcher: crate::command::vanilla::game_dispatcher(),
        registries: &registries,
    };
    let history = vec!["first message".to_owned(), "latest message".to_owned()];
    let mut screen = ChatScreen::new(&m, &commands, "unfinished draft", history.len(), 640, 360);
    for (key, expected) in [
        (ArrowUp, "latest message"),
        (ArrowUp, "first message"),
        (ArrowUp, "first message"),
        (ArrowDown, "latest message"),
        (ArrowDown, "unfinished draft"),
        (ArrowDown, "unfinished draft"),
    ] {
        assert_eq!(
            screen.key(&m, &commands, &history, key, false, false),
            ChatAction::None
        );
        assert_eq!(screen.value(), expected);
    }
}

#[test]
fn mod_commands_are_coloured_and_complete_like_vanilla_ones() {
    let font = font();
    let m = Measure(&font);
    // Allow Commands off: a mod's command needs no level.
    let registries = Registries::default();
    let vanilla = crate::command::vanilla::game_dispatcher();
    let modded = vanilla.with_mod_commands(["where", "coords"]);
    let red = |screen: &ChatScreen| {
        screen
            .format(screen.value(), 0)
            .spans
            .iter()
            .any(|s| s.style.color == Some(text::RED))
    };
    let typed = |dispatcher: &crate::command::Dispatcher, line: &str| {
        let c = Commands {
            dispatcher,
            registries: &registries,
        };
        let mut screen = ChatScreen::new(&m, &c, "", 0, 640, 360);
        screen.type_text(&m, &c, line);
        screen
    };
    // Unknown without the mod's tree (#456).
    assert!(red(&typed(vanilla, "/coords")));
    for line in ["/coords", "/where", "/where me and you"] {
        assert!(!red(&typed(&modded, line)), "{line}");
    }
    let c = Commands {
        dispatcher: &modded,
        registries: &registries,
    };
    let mut screen = typed(&modded, "/coo");
    screen.key(&m, &c, &[], winit::keyboard::KeyCode::Tab, false, false);
    assert_eq!(screen.value(), "/coords");
}

#[test]
fn ask_server_nodes_wait_for_the_servers_completions() {
    // ClientSuggestionProvider.customSuggestion: an `ask_server` node at the
    // cursor sends the command so far, and the list is done with the reply.
    let font = font();
    let m = Measure(&font);
    let registries = Registries {
        permission: 4,
        ..Registries::default()
    };
    let mut d = crate::command::vanilla::game_dispatcher().clone();
    let child = |d: &crate::command::Dispatcher, n: usize, name: &str| {
        d.nodes[n]
            .children
            .iter()
            .copied()
            .find(|&c| d.nodes[c].name() == name)
            .expect(name)
    };
    let id = child(&d, child(&d, child(&d, 0, "bossbar"), "get"), "id");
    d.nodes[id].suggests = Some(crate::command::Suggests::AskServer);
    let c = Commands {
        dispatcher: &d,
        registries: &registries,
    };
    let tab = |screen: &mut ChatScreen| {
        screen.key(&m, &c, &[], winit::keyboard::KeyCode::Tab, false, false);
    };
    let mut screen = ChatScreen::new(&m, &c, "", 0, 640, 360);
    screen.type_text(&m, &c, "/bossbar get pt");
    let (request, sent) = screen.take_request().expect("the server is asked");
    assert_eq!(sent, "/bossbar get pt");
    assert_eq!(screen.take_request(), None);
    // Nothing to complete until the reply.
    tab(&mut screen);
    assert_eq!(screen.value(), "/bossbar get pt");
    // A reply to an older request is dropped; the range is in UTF-16 units.
    let reply = |text: &str| vec![(text.to_owned(), None)];
    screen.server_suggestions(&m, &c, request - 1, (13, 2), reply("pt6:old"));
    tab(&mut screen);
    assert_eq!(screen.value(), "/bossbar get pt");
    screen.server_suggestions(&m, &c, request, (13, 2), reply("pt6:check"));
    tab(&mut screen);
    assert_eq!(screen.value(), "/bossbar get pt6:check");
    // Without the server's provider the local list is done at once.
    let local = crate::command::vanilla::game_dispatcher();
    let c = Commands {
        dispatcher: local,
        registries: &registries,
    };
    let mut screen = ChatScreen::new(&m, &c, "", 0, 640, 360);
    screen.type_text(&m, &c, "/bossbar get pt");
    assert_eq!(screen.take_request(), None);
}

/// A message from `sender` as `Chat::add_from` adds it, without an atlas.
fn add_from(chat: &mut Chat, font: &Font, content: Text, sender: Option<u128>) {
    let message = Message {
        added: 0,
        content,
        indicator: None,
        sender,
        picture: None,
    };
    chat.add_lines(font, &message, false);
    chat.messages.insert(0, message);
}

fn heads_and_texts(chat: &Chat) -> (Vec<String>, Vec<(i32, String)>) {
    let mut heads = Vec::new();
    let mut texts = Vec::new();
    for draw in chat.layout(361, 0, true) {
        match draw {
            GuiDraw::Blit { texture, .. } => heads.push(texture.into_owned()),
            GuiDraw::Text { text, x, .. } => texts.push((x, text.plain())),
            _ => {}
        }
    }
    (heads, texts)
}

#[test]
fn a_mods_picture_starts_the_first_line_and_indents_every_line() {
    use super::{Picture, PictureSource};
    let font = font();
    let mut chat = Chat::default();
    chat.head_skins.insert(7, "skins/abc".into());
    let long = "word ".repeat(80);
    let message = Message {
        added: 0,
        content: Text::literal(long.clone()),
        indicator: None,
        sender: Some(7),
        picture: Some(std::sync::Arc::new(Picture {
            source: PictureSource::Sender,
            regions: vec![[8, 8, 8, 8], [40, 8, 8, 8]],
            width: 8,
            height: 8,
        })),
    };
    chat.add_lines(&font, &message, false);
    let (heads, texts) = heads_and_texts(&chat);
    // A face and a hat, once, before the first line.
    assert_eq!(heads, ["skins/abc", "skins/abc"]);
    assert!(texts.len() > 1);
    assert!(texts.iter().all(|(x, _)| *x == 10));
    // Without the picture the text starts at 0.
    let mut bare = Chat::default();
    add_from(&mut bare, &font, Text::literal(long), Some(7));
    assert!(heads_and_texts(&bare).1.iter().all(|(x, _)| *x == 0));
}

#[test]
fn without_the_extra_a_players_message_draws_as_vanilla() {
    let font = font();
    let mut plain = Chat::default();
    let mut from = Chat::default();
    add(&mut plain, &font, Text::literal("<Steve> hello"));
    add_from(&mut from, &font, Text::literal("<Steve> hello"), Some(7));
    assert_eq!(
        format!("{:?}", plain.layout(361, 0, true)),
        format!("{:?}", from.layout(361, 0, true))
    );
    assert!(heads_and_texts(&from).0.is_empty());
}

#[test]
fn chat_heads_draw_the_senders_face_and_hat_before_the_first_line() {
    let font = font();
    let mut chat = Chat {
        heads: true,
        ..Chat::default()
    };
    chat.head_skins.insert(7, "skins/abc".into());
    add_from(&mut chat, &font, Text::literal("<Steve> hello"), Some(7));
    add_from(&mut chat, &font, Text::literal("[Server] hi"), None);
    // An unknown sender gets the default skin for their UUID.
    add_from(&mut chat, &font, Text::literal("<Alex> hey"), Some(0));
    let (heads, texts) = heads_and_texts(&chat);
    assert_eq!(
        heads,
        [
            "skins/abc",
            "skins/abc",
            tab_list_default(0),
            tab_list_default(0),
        ]
    );
    assert_eq!(
        texts,
        [
            (HEAD_WIDTH, "<Steve> hello".to_owned()),
            (0, "[Server] hi".to_owned()),
            (HEAD_WIDTH, "<Alex> hey".to_owned()),
        ]
    );
}

fn tab_list_default(uuid: u128) -> &'static str {
    crate::tab_list::default_skin(uuid)
}

#[test]
fn a_wrapped_message_with_a_head_indents_every_line_and_heads_only_the_first() {
    let font = font();
    let mut chat = Chat {
        heads: true,
        ..Chat::default()
    };
    let long = "word ".repeat(80);
    add_from(&mut chat, &font, Text::literal(long.clone()), Some(1));
    assert!(chat.lines.len() > 1);
    assert!(chat.lines.iter().all(|l| l.indent == HEAD_WIDTH));
    assert_eq!(chat.lines.iter().filter(|l| l.head.is_some()).count(), 1);
    // The first line (oldest, last in the list) carries the head.
    assert_eq!(chat.lines.last().unwrap().head, Some(1));
    // The head's width comes off the wrapping width.
    let max = chat.max_width() - HEAD_WIDTH;
    assert!(
        chat.lines
            .iter()
            .all(|l| font.width_text(&l.content) <= max as f32)
    );
    // Clicks left of the indent hit nothing.
    assert_eq!(chat.click_at(&font, 360, 4.0 + 5.0, 315.0), None);
}
