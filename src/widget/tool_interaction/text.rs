//! Inline editing for text annotations.

use crate::drawings::{DrawingManager, DrawingToolType};
use crate::localization as l10n;
use crate::theme::ThemeExt;
use egui::{Id, Key, Modifiers, Rect, Ui};

pub(super) fn begin_edit(manager: &mut DrawingManager, id: usize) {
    manager.edit_text(id);
}

/// Where the editor sits, and which plate it borrows.
#[derive(Clone, Copy)]
enum Layout {
    /// Top-left on the anchor.
    Plain,
    /// Centred on the box anchor. A balloon corner follows the box size.
    Centered { balloon: bool },
    /// Top-left on the anchor, in the note plate.
    Plate,
    /// The pin's text card, right of the head.
    Pin,
    /// Centred under the flag glyph.
    Below,
    /// On the line or shape label.
    LineLabel,
    /// Table, image, post and idea keep their popup.
    Popup,
}

fn layout_of(tool: DrawingToolType) -> Layout {
    use DrawingToolType::*;
    match tool {
        TextLabel | AnchoredText => Layout::Plain,
        Callout | Signpost => Layout::Centered { balloon: false },
        Comment => Layout::Centered { balloon: true },
        Note => Layout::Plate,
        AnchoredNote => Layout::Pin,
        FlagNote => Layout::Below,
        _ if tool.supports_line_label() => Layout::LineLabel,
        _ => Layout::Popup,
    }
}

fn draft_is_empty(tool: DrawingToolType, draft: &str) -> bool {
    if tool == DrawingToolType::Table {
        draft.split(['\t', '\n']).all(|cell| cell.trim().is_empty())
    } else {
        draft.trim().is_empty()
    }
}

pub(super) fn show_editor(ui: &mut Ui, manager: &mut DrawingManager, chart_id: Id, pane: Rect) {
    let Some(mut editor) = manager.text_editor.take() else {
        return;
    };
    let Some(drawing) = manager.drawings.iter().find(|d| {
        d.id == editor.id && !d.locked && d.is_visible_on_timeframe(&manager.curr_timeframe)
    }) else {
        if let Some(drawing) = manager.drawings.iter_mut().find(|d| d.id == editor.id) {
            drawing.text_editing = false;
        }
        return;
    };
    let tool = drawing.tool_type;
    let current = drawing.text.clone();
    let layout = layout_of(tool);
    let canvas = !matches!(layout, Layout::Popup);
    let theme = ui.ctx().active_theme();
    let font_size = if tool == DrawingToolType::FlagNote {
        crate::styles::typography::LG
    } else {
        crate::styles::typography::font_size(drawing.font_size)
    };
    let fill = drawing
        .annotation_box_fill(theme.chart.bg_axis)
        .unwrap_or(theme.chart.bg_axis);
    let contrast = matches!(
        tool,
        DrawingToolType::Note
            | DrawingToolType::Callout
            | DrawingToolType::Comment
            | DrawingToolType::Signpost
    );
    let ink = drawing.text_color.map_or_else(
        || {
            if contrast {
                crate::drawings::rendering::contrast_ink(fill)
            } else if matches!(layout, Layout::LineLabel | Layout::Pin | Layout::Below) {
                drawing.text_color32()
            } else {
                theme.ui.text
            }
        },
        |c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
    );
    let sample = if editor.draft.is_empty() {
        "Enter Text"
    } else {
        editor.draft.as_str()
    };
    let mut size = if canvas {
        let lines = sample.split('\n').count().max(1) as f32;
        let cols = sample
            .split('\n')
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        egui::vec2(
            (cols * font_size * 0.62 + 24.0).clamp(72.0, pane.width()),
            (lines * font_size * 1.4 + 14.0).clamp(28.0, pane.height()),
        )
    } else {
        egui::vec2(
            260.0_f32.min(pane.width()),
            (if tool == DrawingToolType::Table {
                240.0_f32
            } else {
                160.0_f32
            })
            .min(pane.height()),
        )
    };
    let anchor = match layout {
        Layout::Centered { .. } => drawing.box_anchor(),
        _ => drawing.points.first().copied(),
    };
    let Some(anchor) = anchor else {
        if let Some(drawing) = manager.drawings.iter_mut().find(|d| d.id == editor.id) {
            drawing.text_editing = false;
        }
        return;
    };
    let frame_margin = egui::vec2(5.0, 3.0);
    let mut pos = match layout {
        Layout::Plain => anchor + egui::vec2(2.0 - 5.0, 2.0 - 3.0),
        Layout::Centered { .. } => anchor - size * 0.5,
        Layout::Plate => anchor,
        Layout::Pin => {
            let ann = &crate::tokens::DESIGN_TOKENS.sizing.annotation;
            let scale = (drawing.stroke_width / crate::styles::stroke::THICK).clamp(0.5, 4.0);
            let head_r = ann.pin_head_radius * scale;
            let head = egui::pos2(anchor.x, anchor.y - ann.pin_needle * scale - head_r);
            egui::pos2(head.x + head_r + ann.label_padding_x, head.y - size.y * 0.5)
        }
        Layout::Below => {
            let text_w = (size.x - frame_margin.x * 2.0).max(1.0);
            egui::pos2(
                anchor.x - text_w * 0.5,
                anchor.y + drawing.font_size * 0.5 + crate::tokens::DESIGN_TOKENS.spacing.xs,
            ) - frame_margin
        }
        Layout::LineLabel => {
            if let Some(label) = drawing.line_label_rect(ui.ctx(), sample, pane) {
                size.x = size.x.max(label.width() + frame_margin.x * 2.0);
                size.y = size.y.max(label.height() + frame_margin.y * 2.0);
                label.min - frame_margin
            } else {
                anchor
            }
        }
        Layout::Popup => anchor,
    };
    let (width, height) = (size.x, size.y);
    pos.x = pos.x.min(pane.right() - width).max(pane.left());
    pos.y = pos.y.min(pane.bottom() - height).max(pane.top());
    let id = chart_id.with(("drawing_text", editor.id));
    let (cancel, commit) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::Escape),
            input.consume_key(Modifiers::COMMAND, Key::Enter)
                || input.consume_key(Modifiers::CTRL, Key::Enter),
        )
    });
    let frame = ui.ctx().cumulative_frame_nr();
    let opened_frame = *editor.opened_frame.get_or_insert(frame);
    let request_focus = std::mem::take(&mut editor.focus);
    let hint = match tool {
        DrawingToolType::Image => "Image address (https://… or file://…)",
        DrawingToolType::Tweet => "Author on the first line, then the post",
        DrawingToolType::Idea => "Title on the first line, then the idea",
        _ => "Enter Text",
    };
    let radius = match layout {
        Layout::Centered { balloon: true } => (width * 0.5).min(height * 0.5),
        Layout::Centered { balloon: false } | Layout::Plate => 5.0,
        Layout::Pin => crate::tokens::DESIGN_TOKENS.rounding.md,
        Layout::Popup => 0.0,
        Layout::Plain | Layout::Below | Layout::LineLabel => 4.0,
    };
    let border = drawing.annotation_border().unwrap_or(egui::Stroke::NONE);
    let mut outside = false;
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .constrain_to(pane)
        .movable(false)
        .show(ui.ctx(), |ui| {
            ui.set_clip_rect(pane);
            if ui.is_sizing_pass() {
                editor.focus = request_focus;
                ui.ctx().request_discard("inline text editor layout");
            }
            let popup = match layout {
                Layout::Plain | Layout::Below | Layout::LineLabel => egui::Frame::new()
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1.0, ink.gamma_multiply(0.3)))
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(5, 3)),
                Layout::Centered { .. } => egui::Frame::new()
                    .fill(fill)
                    .stroke(border)
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(9, 6)),
                Layout::Plate => egui::Frame::new()
                    .fill(fill)
                    .stroke(border)
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
                Layout::Pin => egui::Frame::new()
                    .fill(theme.chart.bg_axis.gamma_multiply(0.92))
                    .stroke(border)
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
                Layout::Popup => egui::Frame::popup(ui.style()),
            };
            let response = popup.show(ui, |ui| {
                ui.set_width((width - popup.total_margin().sum().x).max(1.0));
                let field_id = id.with("field");
                if request_focus {
                    ui.memory_mut(|memory| memory.request_focus(field_id));
                }
                let field = if canvas {
                    let lines = editor.draft.split('\n').count().max(1);
                    egui::ScrollArea::vertical()
                        .max_height((height - popup.total_margin().sum().y).max(1.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut editor.draft)
                                    .id(field_id)
                                    .frame(egui::Frame::NONE)
                                    .hint_text(l10n::text(ui.ctx(), hint))
                                    .text_color(ink)
                                    .font(egui::FontId::proportional(font_size))
                                    .desired_width(ui.available_width())
                                    .desired_rows(lines),
                            )
                        })
                        .inner
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((height - 48.0).max(1.0))
                        .show(ui, |ui| {
                            if tool == DrawingToolType::Table {
                                crate::drawings::Drawing::edit_table(
                                    ui,
                                    &mut editor.draft,
                                    field_id,
                                )
                            } else {
                                ui.add(
                                    egui::TextEdit::multiline(&mut editor.draft)
                                        .id(field_id)
                                        .hint_text(l10n::text(ui.ctx(), hint))
                                        .font(egui::FontId::proportional(font_size))
                                        .desired_width((width - 16.0).max(1.0))
                                        .desired_rows(3),
                                )
                            }
                        })
                        .inner
                };
                if request_focus {
                    field.request_focus();
                }
                if !canvas {
                    ui.small(l10n::text(
                        ui.ctx(),
                        "Ctrl/Cmd+Enter to save, Esc to cancel",
                    ));
                }
            });
            outside = frame > opened_frame && response.response.clicked_elsewhere();
        });
    if cancel || commit || outside {
        ui.memory_mut(|memory| memory.surrender_focus(id.with("field")));
        if let Some(drawing) = manager.drawings.iter_mut().find(|d| d.id == editor.id) {
            drawing.text_editing = false;
        }
        if !cancel
            && matches!(
                tool,
                DrawingToolType::TextLabel | DrawingToolType::AnchoredText
            )
            && draft_is_empty(tool, &editor.draft)
        {
            manager.delete_drawing(editor.id);
        } else if !cancel && draft_is_empty(tool, &editor.draft) {
            if current.is_some() {
                manager.edit_drawing(editor.id, |drawing| drawing.text = None);
            }
        } else if !cancel && current.as_deref() != Some(editor.draft.as_str()) {
            manager.edit_drawing(editor.id, |drawing| drawing.text = Some(editor.draft));
        }
    } else {
        manager.text_editor = Some(editor);
    }
}

#[cfg(test)]
mod tests {
    use crate::drawings::DrawingToolType;
    use crate::widget::test_frames::{chart, frame};
    use egui::{Context, Event, Key, Modifiers, PointerButton, pos2};

    /// Narrow editors stay within the pane, and clicks on their chrome keep the draft open.
    #[test]
    fn editor_bounds_and_inside_clicks() {
        use crate::drawings::{Drawing, DrawingManager};
        use egui::{Id, RawInput, Rect, vec2};
        for dark in [false, true] {
            for (tool, width) in [
                DrawingToolType::TextLabel,
                DrawingToolType::Table,
                DrawingToolType::Image,
                DrawingToolType::Tweet,
                DrawingToolType::Idea,
            ]
            .into_iter()
            .flat_map(|tool| [120.0, 320.0].map(|width| (tool, width)))
            {
                let ctx = Context::default();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let pane = Rect::from_min_size(pos2(20.0, 20.0), vec2(width, 180.0));
                let mut manager = DrawingManager::new();
                let mut drawing = Drawing::new(7, tool);
                drawing.points.push(pane.right_bottom() - vec2(10.0, 10.0));
                drawing.text = Some("Long draft\n".repeat(30));
                manager.drawings.push(drawing);
                super::begin_edit(&mut manager, 7);
                let chart_id = Id::NULL.with("editor_bounds");
                let mut time = 0.0;
                let mut step = |manager: &mut DrawingManager, events| {
                    time += 0.1;
                    ctx.run_ui(
                        RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                pos2(0.0, 0.0),
                                vec2(600.0, 400.0),
                            )),
                            time: Some(time),
                            events,
                            ..Default::default()
                        },
                        |ui| super::show_editor(ui, manager, chart_id, pane),
                    )
                    .drop_without_applying_deltas();
                };
                for _ in 0..3 {
                    step(&mut manager, vec![]);
                }
                let area =
                    egui::AreaState::load(&ctx, chart_id.with(("drawing_text", 7_usize))).unwrap();
                let rect = Rect::from_min_size(area.left_top_pos(), area.size.unwrap());
                assert!(
                    pane.expand(1.0).contains_rect(rect),
                    "{tool:?}, {width}, dark {dark}: {rect:?}"
                );
                let pos = rect.center_bottom() - vec2(0.0, 8.0);
                for pressed in [true, false] {
                    step(
                        &mut manager,
                        vec![
                            Event::PointerMoved(pos),
                            Event::PointerButton {
                                pos,
                                button: PointerButton::Primary,
                                pressed,
                                modifiers: Modifiers::NONE,
                            },
                        ],
                    );
                }
                assert!(
                    manager.text_editor.is_some(),
                    "inside click must keep editing"
                );
                assert!(!manager.can_undo());
            }
        }
    }

    /// Every tool whose text is typed opens an editor on the chart. Commit is one edit,
    /// Escape keeps the previous text, and an empty draft deletes only a text label.
    #[test]
    fn typed_text_commits_on_the_canvas() {
        use crate::drawings::{Drawing, DrawingManager};
        use egui::{Id, RawInput, Rect, vec2};
        let ctx = Context::default();
        let pane = Rect::from_min_size(pos2(0.0, 0.0), vec2(480.0, 360.0));
        let chart_id = Id::NULL.with("typed_text");
        let mut time = 0.0;
        let mut step = |manager: &mut DrawingManager, events| {
            time += 0.05;
            ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0))),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| super::show_editor(ui, manager, chart_id, pane),
            )
            .drop_without_applying_deltas();
        };
        let key = |key, modifiers| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        for tool in DrawingToolType::all()
            .iter()
            .copied()
            .filter(|tool| tool.supports_text() || *tool == DrawingToolType::Image)
        {
            let mut manager = DrawingManager::new();
            let mut drawing = Drawing::new(7, tool);
            drawing.completed = true;
            drawing.points = vec![pos2(90.0, 140.0), pos2(240.0, 80.0), pos2(300.0, 200.0)];
            drawing.text = Some("Old".into());
            manager.drawings.push(drawing);
            assert!(manager.edit_text(7), "{tool:?}");
            for _ in 0..2 {
                step(&mut manager, vec![]);
            }
            let field = chart_id.with(("drawing_text", 7_usize)).with("field");
            assert!(
                ctx.memory(|memory| memory.has_focus(field)),
                "{tool:?} editor is focused"
            );
            step(&mut manager, vec![Event::Text("Hello".into())]);
            step(&mut manager, vec![key(Key::Enter, Modifiers::COMMAND)]);
            assert!(!manager.is_editing_text(), "{tool:?}");
            assert!(!manager.drawings[0].text_editing, "{tool:?}");
            let text = manager.drawings[0].text.as_deref().unwrap_or("");
            assert!(text.contains("Hello"), "{tool:?}: {text:?}");
            assert!(manager.can_undo(), "{tool:?}");
        }

        let mut manager = DrawingManager::new();
        let mut drawing = Drawing::new(7, DrawingToolType::TrendLine);
        drawing.completed = true;
        drawing.points = vec![pos2(90.0, 140.0), pos2(240.0, 80.0)];
        drawing.text = Some("Keep".into());
        manager.drawings.push(drawing);
        assert!(manager.edit_text(7));
        step(&mut manager, vec![]);
        step(&mut manager, vec![Event::Text("Nope".into())]);
        step(&mut manager, vec![key(Key::Escape, Modifiers::NONE)]);
        assert_eq!(manager.drawings[0].text.as_deref(), Some("Keep"));
        assert!(!manager.can_undo());

        for tool in DrawingToolType::all()
            .iter()
            .copied()
            .filter(|tool| tool.supports_text() || *tool == DrawingToolType::Image)
        {
            let mut manager = DrawingManager::new();
            let mut drawing = Drawing::new(7, tool);
            drawing.completed = true;
            drawing.points = vec![pos2(90.0, 140.0), pos2(240.0, 80.0)];
            drawing.text = Some("Gone".into());
            manager.drawings.push(drawing);
            assert!(manager.edit_text(7), "{tool:?}");
            manager.text_editor.as_mut().unwrap().draft.clear();
            step(&mut manager, vec![key(Key::Enter, Modifiers::COMMAND)]);
            let plain = matches!(
                tool,
                DrawingToolType::TextLabel | DrawingToolType::AnchoredText
            );
            if plain {
                assert!(manager.drawings.is_empty(), "{tool:?} is removed");
            } else {
                assert_eq!(manager.drawings.len(), 1, "{tool:?}");
                assert_eq!(manager.drawings[0].text, None, "{tool:?}");
            }
        }
    }

    /// The editor sits on the text it edits, and a generated label never opens one.
    #[test]
    fn editor_sits_on_the_painted_text() {
        use crate::drawings::rendering::ChartView;
        use crate::drawings::{Drawing, DrawingManager};
        use egui::{Id, LayerId, Painter, RawInput, Rect, Shape, vec2};
        let ctx = Context::default();
        let pane = Rect::from_min_size(pos2(0.0, 0.0), vec2(480.0, 360.0));
        let chart_id = Id::NULL.with("editor_place");
        for tool in DrawingToolType::all()
            .iter()
            .copied()
            .filter(|tool| tool.supports_text())
        {
            let mut manager = DrawingManager::new();
            let mut drawing = Drawing::new(7, tool);
            drawing.completed = true;
            drawing.points = vec![pos2(90.0, 160.0), pos2(260.0, 90.0), pos2(320.0, 220.0)];
            drawing.text = Some("Hello".into());
            let placed = drawing.clone();
            manager.drawings.push(drawing);
            assert!(manager.edit_text(7), "{tool:?}");
            ctx.run_ui(RawInput::default(), |ui| {
                super::show_editor(ui, &mut manager, chart_id, pane);
            })
            .drop_without_applying_deltas();
            let area =
                egui::AreaState::load(&ctx, chart_id.with(("drawing_text", 7_usize))).unwrap();
            let editor =
                Rect::from_min_size(area.left_top_pos(), area.size.unwrap_or(vec2(72.0, 28.0)));
            let anchor = placed.points[0];
            match tool {
                DrawingToolType::Comment | DrawingToolType::Signpost => {
                    let center = placed.box_anchor().unwrap();
                    assert!(editor.contains(center), "{tool:?}: {editor:?} {center:?}");
                }
                DrawingToolType::Note => {
                    assert!(
                        editor.min.distance(anchor) < 4.0,
                        "{tool:?}: {editor:?} {anchor:?}"
                    );
                }
                DrawingToolType::FlagNote => {
                    assert!(editor.min.y > anchor.y, "{tool:?}: {editor:?}");
                }
                _ if tool.supports_line_label() => {
                    if let Some(label) = placed.line_label_rect(&ctx, "Hello", pane) {
                        assert!(
                            editor.expand(1.0).intersects(label),
                            "{tool:?}: editor {editor:?} label {label:?}"
                        );
                    }
                }
                _ => {}
            }

            let mut hidden = placed.clone();
            hidden.text_editing = true;
            let output = ctx.run_ui(RawInput::default(), |_| {
                let view = ChartView {
                    instrument: None,
                    bars: &[],
                    revision: 0,
                    bar_to_x: &|b| b,
                    price_to_y: &|p| p as f32,
                    y_to_price: &f64::from,
                    time_axis: true,
                    executions: None,
                };
                hidden.render_original(
                    &Painter::new(ctx.clone(), LayerId::background(), pane),
                    pane,
                    &view,
                );
            });
            let mut labels = Vec::new();
            fn walk(shape: &Shape, out: &mut Vec<String>) {
                match shape {
                    Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
                    Shape::Text(text) => out.push(text.galley.text().to_owned()),
                    _ => {}
                }
            }
            output
                .shapes
                .iter()
                .for_each(|s| walk(&s.shape, &mut labels));
            output.drop_without_applying_deltas();
            if !matches!(
                tool,
                DrawingToolType::Table | DrawingToolType::Tweet | DrawingToolType::Idea
            ) {
                assert!(
                    labels.iter().all(|text| !text.contains("Hello")),
                    "{tool:?} paints its text under the editor: {labels:?}"
                );
            }
        }

        for tool in [
            DrawingToolType::PriceLabel,
            DrawingToolType::PriceNote,
            DrawingToolType::FibonacciRetracement,
        ] {
            let mut manager = DrawingManager::new();
            let mut drawing = Drawing::new(7, tool);
            drawing.points.push(pos2(40.0, 40.0));
            manager.drawings.push(drawing);
            assert!(!manager.edit_text(7), "{tool:?}");
            assert!(!manager.is_editing_text(), "{tool:?}");
        }
    }

    /// Completing an annotation focuses its inline editor. Commit is one undoable
    /// edit; double-click reopens it, and Escape discards the draft without deselecting.
    #[test]
    fn placement_and_double_click_edit_annotation_text() {
        for (tool, pane) in [
            DrawingToolType::TextLabel,
            DrawingToolType::AnchoredText,
            DrawingToolType::Callout,
            DrawingToolType::Table,
            DrawingToolType::Image,
            DrawingToolType::Tweet,
            DrawingToolType::Idea,
        ]
        .into_iter()
        .flat_map(|tool| [(tool, false), (tool, true)])
        {
            let ctx = Context::default();
            let mut chart = chart(&[10.0], &[20.0]);
            if pane {
                chart.add_indicator(Box::new(
                    crate::studies::CustomIndicator::new("Pane", |bars| {
                        vec![crate::studies::IndicatorValue::Single(15.0); bars.len()]
                    })
                    .with_overlay(false),
                ));
            }
            frame(&ctx, &mut chart, 0.0, vec![]);
            chart.drawings_mut().set_active_tool(Some(tool));
            let rect = if pane {
                chart.get_rendered_indicator_panes()[0].chart_rect
            } else {
                chart.get_rendered_price_rect()
            };
            let from = rect.min + rect.size() * egui::vec2(0.4, 0.4);
            let to = rect.min + rect.size() * egui::vec2(0.6, 0.7);
            let button = |pos, pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            let key = |key, modifiers| Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            let mut time = 0.0;
            let mut step = |chart: &mut crate::widget::Chart, events| {
                time += 0.1;
                frame(&ctx, chart, time, events);
            };
            step(&mut chart, vec![Event::PointerMoved(from)]);
            step(&mut chart, vec![button(from, true)]);
            if matches!(tool, DrawingToolType::Callout | DrawingToolType::Image) {
                step(
                    &mut chart,
                    vec![Event::PointerMoved(from + egui::vec2(8.0, 0.0))],
                );
                step(&mut chart, vec![Event::PointerMoved(to)]);
                step(&mut chart, vec![button(to, false)]);
            } else {
                step(&mut chart, vec![button(from, false)]);
            }
            assert_eq!(chart.drawings().drawings.len(), 1, "{tool:?}");
            let expected = if tool == DrawingToolType::Table {
                step(&mut chart, vec![Event::Text("First".into())]);
                step(&mut chart, vec![key(Key::Tab, Modifiers::NONE)]);
                step(&mut chart, vec![Event::Text("Second".into())]);
                "First\tSecond\t\n\t\t\n\t\t\n\t\t"
            } else {
                step(
                    &mut chart,
                    vec![
                        Event::Text("First".into()),
                        key(Key::Enter, Modifiers::NONE),
                        Event::Text("Second".into()),
                    ],
                );
                "First\nSecond"
            };
            if tool == DrawingToolType::Note {
                let outside = pos2(700.0, 450.0);
                step(
                    &mut chart,
                    vec![Event::PointerMoved(outside), button(outside, true)],
                );
                step(&mut chart, vec![button(outside, false)]);
            } else {
                step(&mut chart, vec![key(Key::Enter, Modifiers::COMMAND)]);
            }
            assert_eq!(
                chart.drawings().drawings[0].text.as_deref(),
                Some(expected),
                "{tool:?}"
            );
            let saved = chart.save_drawings().unwrap().to_json().unwrap();
            let mut restored = crate::Chart::new(crate::model::BarData::default());
            restored.load_drawings(&saved).unwrap();
            assert_eq!(
                restored.drawings().drawings[0].text.as_deref(),
                Some(expected)
            );
            assert!(chart.undo());
            assert_eq!(chart.drawings().drawings[0].text, None);
            assert!(chart.redo());
            step(&mut chart, vec![]);
            let id = chart.drawings().drawings[0].id;
            // The pin's text sits on the head above the anchor; the needle tip is the anchor.
            let on_text = if tool == DrawingToolType::AnchoredNote {
                chart.drawings().drawings[0].points[0]
                    - egui::vec2(
                        0.0,
                        crate::tokens::DESIGN_TOKENS.sizing.annotation.pin_needle
                            + crate::tokens::DESIGN_TOKENS
                                .sizing
                                .annotation
                                .pin_head_radius,
                    )
            } else {
                chart.drawings().drawings[0].points[usize::from(tool == DrawingToolType::Callout)]
                    + egui::vec2(10.0, 10.0)
            };
            assert_eq!(
                chart.drawings().hit_test(on_text),
                Some(id),
                "text body for {tool:?}"
            );
            for _ in 0..2 {
                step(
                    &mut chart,
                    vec![Event::PointerMoved(on_text), button(on_text, true)],
                );
                step(&mut chart, vec![button(on_text, false)]);
            }
            assert!(
                chart.take_indicator_settings_request().is_none(),
                "drawing double click owns its pane"
            );
            step(
                &mut chart,
                vec![
                    key(Key::A, Modifiers::COMMAND),
                    Event::Text("Discard".into()),
                ],
            );
            step(&mut chart, vec![key(Key::Escape, Modifiers::NONE)]);
            assert_eq!(
                chart.drawings().drawings[0].text.as_deref(),
                Some(expected),
                "{tool:?}"
            );
            assert_eq!(chart.drawings().sel_drawing(), Some(id), "{tool:?}");
            chart
                .drawings_mut()
                .edit_drawing(id, |drawing| drawing.locked = true);
            for _ in 0..2 {
                step(
                    &mut chart,
                    vec![Event::PointerMoved(on_text), button(on_text, true)],
                );
                step(&mut chart, vec![button(on_text, false)]);
            }
            step(
                &mut chart,
                vec![
                    Event::Text("Locked".into()),
                    key(Key::Enter, Modifiers::COMMAND),
                ],
            );
            assert_eq!(chart.drawings().drawings[0].text.as_deref(), Some(expected));
        }
    }

    /// Placing a text annotation opens its editor. A double click edits a line label, and
    /// finishing a path does not.
    #[test]
    fn placement_opens_annotation_editors_only() {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let place = |tool, drag| {
            let ctx = Context::default();
            let mut chart = chart(&[10.0, 11.0, 12.0], &[20.0, 21.0, 22.0]);
            frame(&ctx, &mut chart, 0.0, vec![]);
            chart.drawings_mut().set_active_tool(Some(tool));
            let rect = chart.get_rendered_price_rect();
            let from = rect.min + rect.size() * egui::vec2(0.35, 0.4);
            let to = rect.min + rect.size() * egui::vec2(0.65, 0.7);
            let mut time = 0.0;
            let mut step = |chart: &mut crate::widget::Chart, events| {
                time += 0.1;
                frame(&ctx, chart, time, events);
            };
            step(&mut chart, vec![Event::PointerMoved(from)]);
            step(&mut chart, vec![button(from, true)]);
            if drag {
                step(
                    &mut chart,
                    vec![Event::PointerMoved(from + egui::vec2(12.0, 0.0))],
                );
                step(&mut chart, vec![Event::PointerMoved(to)]);
                step(&mut chart, vec![button(to, false)]);
            } else {
                step(&mut chart, vec![button(from, false)]);
            }
            assert_eq!(chart.drawings().drawings.len(), 1, "{tool:?}");
            assert!(
                chart.drawings().is_editing_text(),
                "{tool:?} opens an editor when placed"
            );
        };
        place(DrawingToolType::Note, false);
        place(DrawingToolType::AnchoredNote, false);
        place(DrawingToolType::FlagNote, false);
        place(DrawingToolType::Comment, true);
        place(DrawingToolType::Signpost, true);

        let ctx = Context::default();
        let mut chart = chart(&[10.0, 11.0, 12.0], &[20.0, 21.0, 22.0]);
        frame(&ctx, &mut chart, 0.0, vec![]);
        chart
            .drawings_mut()
            .set_active_tool(Some(DrawingToolType::TrendLine));
        let rect = chart.get_rendered_price_rect();
        let from = rect.min + rect.size() * egui::vec2(0.3, 0.45);
        let to = rect.min + rect.size() * egui::vec2(0.7, 0.6);
        let mut time = 1.0;
        let mut step = |chart: &mut crate::widget::Chart, events| {
            time += 1.0;
            frame(&ctx, chart, time, events);
        };
        step(
            &mut chart,
            vec![Event::PointerMoved(from), button(from, true)],
        );
        step(&mut chart, vec![Event::PointerMoved(to), button(to, false)]);
        assert!(
            !chart.drawings().is_editing_text(),
            "a line has no editor yet"
        );
        let mid = from + (to - from) * 0.5;
        let mut t = 10.0;
        for _ in 0..2 {
            t += 0.1;
            frame(
                &ctx,
                &mut chart,
                t,
                vec![Event::PointerMoved(mid), button(mid, true)],
            );
            t += 0.1;
            frame(&ctx, &mut chart, t, vec![button(mid, false)]);
        }
        assert!(
            chart.drawings().is_editing_text(),
            "double-click edits the line label"
        );

        let ctx = Context::default();
        let mut path = crate::widget::test_frames::chart(&[10.0, 11.0, 12.0], &[20.0, 21.0, 22.0]);
        frame(&ctx, &mut path, 0.0, vec![]);
        path.drawings_mut()
            .set_active_tool(Some(DrawingToolType::Path));
        let rect = path.get_rendered_price_rect();
        let a = rect.min + rect.size() * egui::vec2(0.3, 0.4);
        let b = rect.min + rect.size() * egui::vec2(0.5, 0.6);
        let c = rect.min + rect.size() * egui::vec2(0.7, 0.45);
        let click = |path: &mut crate::widget::Chart, pos, t: f64| {
            frame(
                &ctx,
                path,
                t,
                vec![Event::PointerMoved(pos), button(pos, true)],
            );
            frame(&ctx, path, t + 0.05, vec![button(pos, false)]);
        };
        click(&mut path, a, 1.0);
        click(&mut path, b, 2.0);
        click(&mut path, c, 3.0);
        click(&mut path, c, 3.12);
        assert_eq!(path.drawings().drawings.len(), 1, "the path was finished");
        assert!(
            !path.drawings().is_editing_text(),
            "finishing a path does not open an editor"
        );
    }

    /// Pane-relative anchors survive pan and zoom, and scale with pane size rather than data.
    #[test]
    fn anchored_annotations_keep_their_pane_position() {
        use crate::drawings::{ChartPoint, Drawing};
        for tool in [
            DrawingToolType::AnchoredText,
            DrawingToolType::AnchoredNote,
            DrawingToolType::TextLabel,
        ] {
            let ctx = Context::default();
            let mut chart = chart(&[10.0], &[20.0]);
            let mut drawing = Drawing::new(7, tool);
            drawing.chart_points.push(ChartPoint::new(50.0, 15.0));
            chart.drawings_mut().drawings.push(drawing);
            frame(&ctx, &mut chart, 0.0, vec![]);
            let original = chart.drawings().drawings[0].points[0];
            chart.set_visible_bars(70);
            chart.set_start_idx(20);
            frame(&ctx, &mut chart, 0.1, vec![]);
            let moved = chart.drawings().drawings[0].points[0];
            if tool == DrawingToolType::TextLabel {
                assert_ne!(moved, original);
            } else {
                assert_eq!(moved, original, "{tool:?}");
                let mut fixed = Drawing::new(9, tool);
                fixed.completed = true;
                fixed.points.push(pos2(100.0, 200.0));
                fixed.chart_points.push(ChartPoint::new(0.0, 0.0));
                fixed.update_pane_coords(egui::Rect::from_min_size(
                    pos2(40.0, 60.0),
                    egui::vec2(800.0, 400.0),
                ));
                // (100-40)/800 = .075, (200-60)/400 = .35; on the new pane: (130,360).
                fixed.update_pane_coords(egui::Rect::from_min_size(
                    pos2(100.0, 80.0),
                    egui::vec2(400.0, 800.0),
                ));
                assert_eq!(fixed.points[0], pos2(130.0, 360.0));
                let mut manager = crate::drawings::DrawingManager::new();
                manager.drawings.push(fixed);
                let pane = egui::Rect::from_min_size(pos2(100.0, 80.0), egui::vec2(400.0, 800.0));
                manager.update_pane_anchors(pane);
                manager.select(9);
                manager.start_drag_body(9, pos2(135.0, 365.0));
                manager.update_drag_handle(
                    pos2(175.0, 405.0),
                    |x| x,
                    f64::from,
                    |price, from, to| price + f64::from(to - from),
                );
                manager.end_drag_handle();
                manager.update_all_screen_coords(|x| x, |y| y as f32);
                manager.update_pane_anchors(pane);
                assert_eq!(manager.drawings[0].points[0], pos2(170.0, 400.0));
                assert!(manager.undo());
                manager.update_all_screen_coords(|x| x, |y| y as f32);
                manager.update_pane_anchors(pane);
                assert_eq!(manager.drawings[0].points[0], pos2(130.0, 360.0));
                let saved = chart.save_drawings().unwrap().to_json().unwrap();
                chart.load_drawings(&saved).unwrap();
                chart.set_visible_bars(50);
                frame(&ctx, &mut chart, 0.2, vec![]);
                assert_eq!(chart.drawings().drawings[0].points[0], original);
            }
        }
    }
}
