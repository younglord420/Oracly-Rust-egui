//! Commands: keyboard requests, quick search, export and the clipboard.

use super::core::Direction;
use super::layout::Region;
use super::services::ExportContents;
use super::{Action, ExportFile, Terminal};
use egui_charts::config::KeyboardAction;
use egui_charts::drawings::DrawingToolType;
use egui_charts::localization as l10n;
use egui_charts::scales::PriceScaleMode;
use egui_charts::services::UserSettings;
use egui_charts::studies::IndicatorFactory;
use egui_charts::ui::Control;
use egui_charts::ui::context_menu::{ChartContextAction, ContextMenuAction};
use egui_charts::ui::drawing_toolbar::DrawingToolbarAction;
use egui_charts::ui::export::{ExportConfig, ExportDialog, ExportFormat, ExportResponse};
use egui_charts::ui::timeframe_toolbar::TimeframeToolbarAction;
use egui_charts::ui::top_toolbar::TopToolbarAction;
use egui_charts::ui::widget_bar::{RightBarIcon, WidgetBarAction};
use egui_charts::ui_kit::command_palette::{Command, CommandPaletteHandler};
use egui_charts::widget::{ImageExportOptions, ShortcutRequest};

pub(super) struct State {
    export: ExportDialog,
    quick_search: CommandPaletteHandler,
}

impl Default for State {
    fn default() -> Self {
        Self {
            export: ExportDialog::new().with_formats(&[ExportFormat::Clipboard]),
            quick_search: CommandPaletteHandler::new(quick_search_commands()),
        }
    }
}

fn quick_search_commands() -> Vec<Command> {
    let mut commands: Vec<_> = [
        ("symbols", "Symbol search", "Actions"),
        ("indicators", "Indicators", "Actions"),
        ("settings", "Chart settings", "Settings"),
        ("export", "Export chart", "Actions"),
        ("date", "Go to date", "Actions"),
        ("replay", "Toggle replay", "Actions"),
        ("help", "Keyboard shortcuts", "Actions"),
        ("reset", "Reset chart view", "Actions"),
        ("log", "Toggle logarithmic scale", "Settings"),
        ("invert", "Invert price scale", "Settings"),
        ("grid", "Toggle grid", "Settings"),
    ]
    .into_iter()
    .map(|(id, label, category)| Command::new(id, label).category(category))
    .collect();
    for category in DrawingToolType::categories() {
        for tool in DrawingToolType::by_category(category) {
            if !tool.is_cursor() {
                commands.push(
                    Command::new(format!("tool:{}", tool.data_name()), tool.as_str())
                        .category("Drawing tools")
                        .keywords([*category]),
                );
            }
        }
    }
    let factory = IndicatorFactory::new();
    let mut names = factory.list();
    names.sort();
    commands.extend(names.into_iter().map(|name| {
        Command::new(format!("study:{name}"), format!("Add {name}"))
            .category("Studies")
            .keywords([factory.category(&name).unwrap_or_default()])
    }));
    commands
}

impl Terminal {
    pub(super) fn commands_init(&mut self) {
        self.set_control_visible(Control::Interval, true);
        self.set_control_visible(Control::QuickSearch, true);
        if self.services.save.is_some() {
            let formats = if self.services.export_formats.is_empty() {
                &[
                    ExportFormat::Png,
                    ExportFormat::Svg,
                    ExportFormat::Pdf,
                    ExportFormat::Csv,
                    ExportFormat::Clipboard,
                ][..]
            } else {
                self.services.export_formats
            };
            self.commands.export = ExportDialog::new().with_formats(formats);
            self.set_control_visible(Control::Snapshot, true);
            self.set_control_visible(Control::ExportData, true);
            // File export is the terminal's share path when the host has no publish service.
            self.top_toolbar.state.can_publish_image = true;
            self.set_control_visible(Control::Publish, true);
        } else {
            // Clipboard-only until the host supplies `with_file_export`; keep Snapshot hidden so
            // the default toolbar layout matches terminals that have not wired exports yet.
            self.commands.export =
                ExportDialog::new().with_formats(&[ExportFormat::Clipboard]);
        }
    }

    pub(super) fn commands_poll(&mut self, _ctx: &egui::Context) {}

    pub(super) fn commands_show(&mut self, ui: &mut egui::Ui, region: Region) {
        if region != Region::Windows {
            return;
        }
        match self.commands.export.show(ui.ctx()) {
            ExportResponse::Export { config, save_path } => {
                self.run_export(ui.ctx(), config, save_path);
            }
            ExportResponse::Closed | ExportResponse::None => {}
        }
        // The chart already emits OpenQuickSearch for Ctrl/Cmd+K; show without a second opener.
        if let Some(id) = self.commands.quick_search.show(ui.ctx()) {
            self.execute_quick_search(&id);
        }
    }

    pub(super) fn commands_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Shortcut(request) => self.perform_shortcut(request),
            Action::TopToolbar(TopToolbarAction::OpenQuickSearch) => {
                if self.shown(Control::QuickSearch) {
                    self.commands.quick_search.open();
                    None
                } else {
                    Some(Action::TopToolbar(TopToolbarAction::OpenQuickSearch))
                }
            }
            Action::TopToolbar(TopToolbarAction::TakeScreenshot)
            | Action::TopToolbar(TopToolbarAction::OpenImage) => {
                self.commands.export.open();
                None
            }
            Action::TopToolbar(TopToolbarAction::CopyImage) => {
                let mut options = ImageExportOptions::default();
                options.include_crosshair = true;
                if let Some(ctx) = self.ctx.clone() {
                    self.copy_chart_image(&ctx, options);
                }
                None
            }
            Action::TopToolbar(TopToolbarAction::Publish) => {
                if self.services.save.is_some() {
                    if let Some(ctx) = self.ctx.clone() {
                        self.save_png(&ctx, ImageExportOptions::default(), None);
                    }
                    None
                } else {
                    Some(Action::TopToolbar(TopToolbarAction::Publish))
                }
            }
            // Shareable link URLs need a host publishing service.
            Action::TopToolbar(TopToolbarAction::CopyImageLink) => {
                Some(Action::TopToolbar(TopToolbarAction::CopyImageLink))
            }
            Action::ChartMenu(ChartContextAction::Chart(ContextMenuAction::ExportCsv)) => {
                self.save_csv(None);
                None
            }
            Action::Export(response) => match response {
                ExportResponse::Export { config, save_path } => {
                    if let Some(ctx) = self.ctx.clone() {
                        self.run_export(&ctx, config, save_path);
                    }
                    None
                }
                ExportResponse::Closed | ExportResponse::None => None,
            },
            other => Some(other),
        }
    }

    fn perform_shortcut(&mut self, request: ShortcutRequest) -> Option<Action> {
        match request {
            ShortcutRequest::IntervalEntry(text) => {
                if self.shown(Control::Interval) {
                    self.top_toolbar
                        .timeframe_selector
                        .open_interval_entry(text);
                    None
                } else {
                    Some(Action::Shortcut(ShortcutRequest::IntervalEntry(text)))
                }
            }
            ShortcutRequest::Action(KeyboardAction::OpenIntervalSearch) => {
                if self.shown(Control::Interval) {
                    self.top_toolbar
                        .timeframe_selector
                        .open_interval_entry(String::new());
                    None
                } else {
                    Some(Action::Shortcut(ShortcutRequest::Action(
                        KeyboardAction::OpenIntervalSearch,
                    )))
                }
            }
            ShortcutRequest::Action(KeyboardAction::OpenQuickSearch) => {
                if self.shown(Control::QuickSearch) {
                    self.commands.quick_search.open();
                    None
                } else {
                    Some(Action::Shortcut(ShortcutRequest::Action(
                        KeyboardAction::OpenQuickSearch,
                    )))
                }
            }
            ShortcutRequest::Action(KeyboardAction::MaximizeChart) => {
                // Multi-chart grids toggle the active tile in ChartLayout; a lone chart goes
                // fullscreen through the top toolbar.
                if self.charts.len() > 1 {
                    None
                } else {
                    self.core_perform(Action::TopToolbar(TopToolbarAction::ToggleFullscreen))
                }
            }
            ShortcutRequest::SymbolSearch(text) => {
                Some(Action::Shortcut(ShortcutRequest::SymbolSearch(text)))
            }
            other => Some(Action::Shortcut(other)),
        }
    }

    fn execute_quick_search(&mut self, id: &str) {
        if let Some(name) = id.strip_prefix("tool:") {
            if let Some(tool) = DrawingToolType::all()
                .iter()
                .find(|tool| tool.data_name() == name)
            {
                self.emit(Action::DrawingToolbar(DrawingToolbarAction::SelectTool(
                    *tool,
                )));
            }
            return;
        }
        if let Some(name) = id.strip_prefix("study:") {
            self.emit(Action::TopToolbar(TopToolbarAction::AddIndicator(
                name.to_owned(),
            )));
            return;
        }
        match id {
            "symbols" => self.emit(Action::TopToolbar(TopToolbarAction::OpenSymbolSearch)),
            "indicators" => self.emit(Action::TopToolbar(TopToolbarAction::OpenIndicators)),
            "settings" => self.emit(Action::TopToolbar(TopToolbarAction::OpenSettings)),
            "export" => self.commands.export.open(),
            "date" => self.emit(Action::BottomBar(TimeframeToolbarAction::OpenDatePicker)),
            "replay" => self.emit(Action::TopToolbar(TopToolbarAction::ToggleReplay)),
            "help" => self.emit(Action::WidgetBar(WidgetBarAction::Clicked(
                RightBarIcon::Help,
            ))),
            "reset" => self.emit(Action::ChartMenu(ChartContextAction::Chart(
                ContextMenuAction::ResetChart,
            ))),
            "log" => {
                self.user_action(|terminal| {
                    let mode = &mut terminal.chart_mut().price_scale_mut().options_mut().mode;
                    *mode = if *mode == PriceScaleMode::Logarithmic {
                        PriceScaleMode::Normal
                    } else {
                        PriceScaleMode::Logarithmic
                    };
                });
            }
            "invert" => {
                self.user_action(|terminal| {
                    let options = terminal.chart_mut().price_scale_mut().options_mut();
                    options.invert_scale = !options.invert_scale;
                });
            }
            "grid" => {
                self.user_action(|terminal| {
                    let config = terminal.chart_mut().config_mut();
                    config.show_grid = !config.show_grid;
                });
            }
            _ => {}
        }
    }

    pub(super) fn commands_settings(
        &mut self,
        _settings: &mut UserSettings,
        _direction: Direction,
    ) {
    }

    pub(super) fn commands_modal(&self) -> bool {
        self.commands.export.is_open() || self.commands.quick_search.is_open()
    }

    fn run_export(
        &mut self,
        ctx: &egui::Context,
        config: ExportConfig,
        save_path: Option<std::path::PathBuf>,
    ) {
        match config.format {
            ExportFormat::Csv => self.save_csv(save_path),
            ExportFormat::Clipboard => self.copy_chart_image(ctx, config.image),
            ExportFormat::Png => self.save_png(ctx, config.image, save_path),
            ExportFormat::Svg => self.save_svg(ctx, &config.image, save_path),
            ExportFormat::Pdf => self.save_pdf(ctx, &config.image, save_path),
        }
    }

    fn save_csv(&mut self, save_path: Option<std::path::PathBuf>) {
        match self.chart().export_csv() {
            Ok(csv) => {
                let name = format!("{}_export.csv", self.chart().symbol());
                self.finish_export(name, save_path, ExportContents::Text(csv));
            }
            Err(error) => self.export_failed(save_path.is_some(), error.to_string()),
        }
    }

    fn save_png(
        &mut self,
        ctx: &egui::Context,
        options: ImageExportOptions,
        save_path: Option<std::path::PathBuf>,
    ) {
        match self.export_png(ctx, &options) {
            Ok(png) => {
                let name = format!("{}_export.png", self.chart().symbol());
                self.finish_export(name, save_path, ExportContents::Png(png));
            }
            Err(error) => self.export_failed(save_path.is_some(), error),
        }
    }

    fn save_svg(
        &mut self,
        ctx: &egui::Context,
        options: &ImageExportOptions,
        save_path: Option<std::path::PathBuf>,
    ) {
        match Terminal::export_svg(self, ctx, options) {
            Ok(svg) => {
                let name = format!("{}_export.svg", self.chart().symbol());
                self.finish_export(name, save_path, ExportContents::Text(svg));
            }
            Err(error) => self.export_failed(save_path.is_some(), error),
        }
    }

    fn save_pdf(
        &mut self,
        ctx: &egui::Context,
        options: &ImageExportOptions,
        save_path: Option<std::path::PathBuf>,
    ) {
        match Terminal::export_pdf(self, ctx, options) {
            Ok(pdf) => {
                let name = format!("{}_export.pdf", self.chart().symbol());
                self.finish_export(name, save_path, ExportContents::Pdf(pdf));
            }
            Err(error) => self.export_failed(save_path.is_some(), error),
        }
    }

    fn copy_chart_image(&mut self, ctx: &egui::Context, options: ImageExportOptions) {
        match self.export_png(ctx, &options) {
            Ok(png) => match copy_png(ctx, &png) {
                Ok(()) => {
                    if self.commands.export.is_open() {
                        self.commands.export.mark_success();
                    }
                    self.toast(l10n::text(ctx, "Image sent to clipboard"));
                }
                Err(error) => self.export_failed(self.commands.export.is_open(), error),
            },
            Err(error) => self.export_failed(self.commands.export.is_open(), error),
        }
    }

    fn finish_export(
        &mut self,
        name: String,
        path: Option<std::path::PathBuf>,
        contents: ExportContents,
    ) {
        let from_dialog = self.commands.export.is_open();
        if let Some(save) = &mut self.services.save {
            save(ExportFile {
                name: name.clone(),
                path: path.clone(),
                contents,
            });
            if from_dialog {
                self.commands.export.mark_success();
            }
            let detail = path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or(name);
            self.toast(l10n::format_text(
                &self.ctx(),
                "Export successful: {0}",
                &[("0", detail)],
            ));
        } else {
            self.export_failed(from_dialog, "No file export handler".into());
        }
    }

    fn export_failed(&mut self, from_dialog: bool, error: String) {
        if from_dialog {
            self.commands.export.mark_error(error.clone());
        }
        self.toast(l10n::format_text(
            &self.ctx(),
            "Export failed: {0}",
            &[("0", error)],
        ));
    }
}

fn copy_png(ctx: &egui::Context, png: &[u8]) -> Result<(), String> {
    let decoder = png::Decoder::new(png);
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).map_err(|error| error.to_string())?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        other => return Err(format!("unsupported PNG colour type {other:?}")),
    };
    ctx.copy_image(egui::ColorImage::from_rgba_unmultiplied(
        [info.width as usize, info.height as usize],
        &rgba,
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{SIZE, TestFeed, frame};
    use super::*;
    use egui_charts::config::TimezoneMode;
    use egui_charts::services::UserSettings;
    use std::sync::{Arc, Mutex};

    #[test]
    fn quick_search_opens_and_runs_settings() {
        let mut terminal = Terminal::new(Arc::new(TestFeed)).with_symbol("AAPL");
        frame(&mut terminal, SIZE, Vec::new());
        assert!(!terminal.commands.quick_search.is_open());
        terminal.run(Action::TopToolbar(TopToolbarAction::OpenQuickSearch));
        assert!(terminal.commands.quick_search.is_open());
        assert!(terminal.commands_modal());
        terminal.execute_quick_search("settings");
        frame(&mut terminal, SIZE, Vec::new());
        assert!(terminal.studies_modal());
    }

    #[test]
    fn publish_saves_png_when_file_export_is_wired() {
        let saved = Arc::new(Mutex::new(None));
        let sink = saved.clone();
        let mut terminal = Terminal::new(Arc::new(TestFeed))
            .with_symbol("AAPL")
            .with_file_export(&[ExportFormat::Png], move |file| {
                *sink.lock().unwrap() = Some(file.name);
            });
        frame(&mut terminal, SIZE, Vec::new());
        assert!(terminal.top_toolbar.state.can_publish_image);
        terminal.run(Action::TopToolbar(TopToolbarAction::Publish));
        let name = saved.lock().unwrap().clone();
        assert_eq!(name.as_deref(), Some("AAPL_export.png"));
    }

    #[test]
    fn publish_returns_to_host_without_file_export() {
        let mut terminal = Terminal::new(Arc::new(TestFeed)).with_symbol("AAPL");
        frame(&mut terminal, SIZE, Vec::new());
        assert!(!terminal.top_toolbar.state.can_publish_image);
        let left = terminal.run(Action::TopToolbar(TopToolbarAction::Publish));
        assert!(matches!(
            left,
            Some(Action::TopToolbar(TopToolbarAction::Publish))
        ));
    }

    #[test]
    fn time_preferences_round_trip_through_user_settings() {
        let mut terminal = Terminal::new(Arc::new(TestFeed)).with_symbol("AAPL");
        frame(&mut terminal, SIZE, Vec::new());
        let mut settings = UserSettings::default();
        settings.timezone = Some("UTC".into());
        settings.extended_hours = Some(true);
        settings.adjust_for_dividends = Some(true);
        terminal.time_settings(&mut settings, Direction::Load);
        assert!(matches!(
            terminal.chart().chart_options().time_scale.timezone,
            TimezoneMode::Utc
        ));
        assert!(terminal.chart().extended_hours());
        assert!(terminal.studies.settings.adjust_for_dividends);

        let mut saved = UserSettings::default();
        terminal.time_settings(&mut saved, Direction::Save);
        assert_eq!(saved.timezone.as_deref(), Some("UTC"));
        assert_eq!(saved.extended_hours, Some(true));
        assert_eq!(saved.adjust_for_dividends, Some(true));
    }
}
