//! Replay: the replay start picker and the floating replay control across the chart slots.

use super::core::Direction;
use super::layout::Region;
use super::{Action, Terminal};
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::floating_replay_control::{FloatingReplayControl, ReplayControlAction};
use egui_charts::ui::replay::{
    ReplayAction, ReplayController, ReplayStartAction, ReplayStartPicker,
};
use egui_charts::ui::replay::{ReplayToolbarAction, ReplayToolbarState};
use egui_charts::ui::top_toolbar::TopToolbarAction;
use egui_charts::widget::SeriesId;

pub(super) struct State {
    picker: ReplayStartPicker,
    control: FloatingReplayControl,
    controller: Option<ReplayController>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            picker: ReplayStartPicker::default(),
            control: FloatingReplayControl::new(),
            controller: None,
        }
    }
}

impl Terminal {
    pub(super) fn replay_init(&mut self) {
        // Replay is always available; no external service is required.
        self.set_control_visible(Control::Replay, true);
    }

    pub(super) fn replay_poll(&mut self, ctx: &egui::Context) {
        // Advance the controller and collect state before releasing the borrow.
        let (advanced, active, playing, speed, curr, total) = {
            let Some(controller) = self.replay.controller.as_mut() else {
                return;
            };
            let advanced = controller.update();
            (
                advanced,
                controller.state.is_active(),
                controller.state.is_playing(),
                controller.state.speed.multiplier(),
                controller.state.curr_bar,
                controller.state.total_bars,
            )
        };
        // Sync the floating control's display from the controller's state.
        self.replay.control.sync_from(active, playing, speed, curr, total);
        // Sync the trading dock's replay toolbar state.
        self.trading.replay.sync_from(active, playing, speed);
        if advanced > 0 {
            ctx.request_repaint();
            self.push_replay_to_chart();
        }
    }

    pub(super) fn replay_show(&mut self, ui: &mut egui::Ui, region: Region) {
        match region {
            Region::Chart => {
                // Keep the draggable bar docked inside the chart area.
                self.replay.control.set_dock_rect(self.chart_area);
            }
            Region::Windows => {
                let ctx = ui.ctx().clone();
                // Floating replay control – navigation and speed actions.
                let control_action = self.replay.control.show(&ctx);
                if control_action != ReplayControlAction::None {
                    ctx.request_repaint();
                    self.emit(Action::Replay(control_action));
                }
                // Start-date picker – where replay begins.
                let start_action = self.replay.picker.show(&ctx, false);
                if start_action != ReplayStartAction::None {
                    ctx.request_repaint();
                    self.emit(Action::ReplayStart(start_action));
                }
            }
            _ => {}
        }
    }

    pub(super) fn replay_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            // Top toolbar toggle: open picker when idle, stop when active.
            Action::TopToolbar(TopToolbarAction::ToggleReplay) => {
                self.toggle_replay();
                None
            }
            // Start picker choice.
            Action::ReplayStart(start_action) => {
                self.handle_replay_start(start_action);
                None
            }
            // Floating control actions.
            Action::Replay(control_action) => {
                self.handle_replay_control(control_action);
                None
            }
            // Replay toolbar in the trading dock.
            Action::ReplayToolbar(toolbar_action) => {
                self.handle_replay_toolbar(toolbar_action);
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn replay_settings(
        &mut self,
        _settings: &mut UserSettings,
        _direction: Direction,
    ) {}

    pub(super) fn replay_modal(&self) -> bool {
        self.replay.picker.is_open
    }

    /// Toggles replay on or off.
    fn toggle_replay(&mut self) {
        if self.replay_active {
            self.stop_replay();
        } else {
            let start_date = self
                .chart()
                .bars(SeriesId::MAIN)
                .first()
                .map(|bar| bar.time);
            self.replay.picker.open(start_date);
        }
    }

    /// Handles a choice from the start picker.
    fn handle_replay_start(&mut self, action: ReplayStartAction) {
        let bars = self.chart().bars(SeriesId::MAIN);
        let index = match action {
            ReplayStartAction::None => return,
            ReplayStartAction::Resume => return, // no session persistence yet
            ReplayStartAction::SelectBar => return, // requires a click handler wired elsewhere
            ReplayStartAction::First => 0,
            ReplayStartAction::Random => {
                use std::hash::BuildHasher;
                let count = bars.len().max(1);
                std::collections::hash_map::RandomState::new()
                    .hash_one(chrono::Utc::now()) as usize
                    % count
            }
            ReplayStartAction::Date(time) => bars.partition_point(|bar| bar.time < time),
            _ => return,
        };
        if index < bars.len() {
            self.start_replay_at(index);
        }
    }

    /// Starts replay from bar `index`, entering replay mode.
    fn start_replay_at(&mut self, index: usize) {
        let bars = self.chart().bars(SeriesId::MAIN).to_vec();
        if bars.is_empty() {
            return;
        }
        let symbol = self.chart().symbol().to_owned();
        let mut controller =
            ReplayController::new(bars, vec![]).with_symbol(symbol.clone());
        controller.handle_action(ReplayAction::EnterReplayMode);
        controller.handle_action(ReplayAction::JumpToBar(index));
        self.replay.controller = Some(controller);
        self.replay_active = true;
        self.replay.control.set_symbol(&symbol);
        self.replay.control.show_control();
        // Pause the chart's live feed while replaying.
        self.chart_mut().set_feed_paused(true);
        self.push_replay_to_chart();
    }

    /// Stops the active replay and restores the live feed.
    pub(super) fn stop_replay(&mut self) {
        if self.replay.controller.take().is_some() {
            self.chart_mut().set_replay_time(None);
            self.chart_mut().set_feed_paused(false);
            self.replay_active = false;
            self.replay.control.hide_control();
            self.replay.picker.is_open = false;
        }
    }

    /// Applies the current controller cursor to the chart's replay window.
    fn push_replay_to_chart(&mut self) {
        let Some(controller) = &self.replay.controller else {
            return;
        };
        let clock = self.chart().bar_clock();
        if let Some(cutoff) = controller.cutoff(&clock) {
            self.chart_mut().set_replay_time(Some(cutoff));
        }
    }

    /// Handles an action from the floating replay control.
    fn handle_replay_control(&mut self, action: ReplayControlAction) {
        match &action {
            ReplayControlAction::Stop => {
                self.stop_replay();
                return;
            }
            ReplayControlAction::Close => {
                self.replay.control.dismiss();
                return;
            }
            ReplayControlAction::None => return,
            _ => {}
        }
        // Map control actions to replay actions and forward to the controller.
        let replay_action = match action {
            ReplayControlAction::Play => Some(ReplayAction::Play),
            ReplayControlAction::Pause => Some(ReplayAction::Pause),
            ReplayControlAction::StepForward | ReplayControlAction::NextBar => {
                Some(ReplayAction::StepForward)
            }
            ReplayControlAction::StepBackward | ReplayControlAction::PreviousBar => {
                Some(ReplayAction::StepBackward)
            }
            ReplayControlAction::GoToStart => Some(ReplayAction::JumpToStart),
            ReplayControlAction::GoToEnd => Some(ReplayAction::JumpToEnd),
            ReplayControlAction::SetSpeed(speed) => Some(ReplayAction::SetSpeed(speed)),
            _ => None,
        };
        if let Some(replay_action) = replay_action {
            if let Some(controller) = self.replay.controller.as_mut() {
                controller.handle_action(replay_action);
            }
            self.push_replay_to_chart();
        }
    }

    /// Handles an action from the trading dock's replay toolbar.
    fn handle_replay_toolbar(&mut self, action: ReplayToolbarAction) {
        let control_action = match action {
            ReplayToolbarAction::Play => ReplayControlAction::Play,
            ReplayToolbarAction::Pause => ReplayControlAction::Pause,
            ReplayToolbarAction::Stop => ReplayControlAction::Stop,
            ReplayToolbarAction::StepForward => ReplayControlAction::StepForward,
            ReplayToolbarAction::StepBackward => ReplayControlAction::StepBackward,
            ReplayToolbarAction::SetSpeed(speed) => {
                ReplayControlAction::SetSpeed(speed.multiplier() as f32)
            }
            _ => return,
        };
        self.handle_replay_control(control_action);
    }
}

// Silence the dead-code lint: `ReplayToolbarState` sync is done inside `replay_poll`.
const _: fn() = || {
    let _: ReplayToolbarState;
};
