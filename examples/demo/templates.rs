//! Study and drawing templates, kept in the demo's storage.

use super::DemoApp;
use egui_charts::studies::{InputValue, StudySource, StudyTemplate};
use egui_charts::ui::dialogs::templates::DrawingTemplate;

impl DemoApp {
    pub(super) fn current_study_template(&self) -> Vec<StudyTemplate> {
        let studies: Vec<_> = self.chart.indicators().collect();
        StudyTemplate::capture_set(studies.iter().map(|(id, study)| (id.0, *study)))
    }

    pub(super) fn load_study_template(&mut self, template: &[StudyTemplate]) {
        match StudyTemplate::create_set_with(template, &egui_charts::studies::IndicatorFactory::new())
        {
            Ok(studies) => self.user_action(|app| {
                app.remove_all_indicators();
                let mut ids = Vec::new();
                for study in studies {
                    let inputs = super::study_input_summary(study.as_ref());
                    let id = app.chart.add_indicator(study);
                    app.chart.set_indicator_inputs(id, inputs);
                    ids.push(id.0);
                }
                for (id, mut stored) in ids.iter().copied().zip(template.iter().cloned()) {
                    stored.remap_sources(&ids);
                    for (input, value) in &stored.inputs {
                        if matches!(value, InputValue::Source(StudySource::Study { .. })) {
                            let study = app
                                .chart
                                .indicator_mut(egui_charts::widget::SeriesId(id))
                                .expect("added study");
                            study
                                .set_input(input, value.clone())
                                .expect("validated study source");
                            let inputs = super::study_input_summary(study);
                            app.chart
                                .set_indicator_inputs(egui_charts::widget::SeriesId(id), inputs);
                        }
                    }
                }
            }),
            Err(error) => self.notify(error),
        }
    }

    pub(super) fn current_drawing_template(&self) -> Option<DrawingTemplate> {
        let manager = self.chart.drawings();
        let id = manager.sel_drawing()?;
        manager
            .drawings
            .iter()
            .find(|drawing| drawing.id == id)
            .map(DrawingTemplate::from_drawing)
    }

    pub(super) fn load_drawing_template(&mut self, template: DrawingTemplate) {
        let manager = self.chart.drawings();
        let Some(id) = manager.sel_drawing() else {
            self.notify("Select a drawing first");
            return;
        };
        let Some(drawing) = manager.drawings.iter().find(|drawing| drawing.id == id) else {
            return;
        };
        let mut edited = drawing.clone();
        if let Err(error) = template.apply_to(&mut edited) {
            self.notify(error);
            return;
        }
        self.chart.user_action(|chart| {
            chart
                .drawings_mut()
                .edit_drawing(id, |drawing| *drawing = edited)
        });
    }

    pub(super) fn show_templates(&mut self, ctx: &egui::Context) {
        if let Some(studies) = self.study_templates.show(ctx, "Study templates") {
            self.load_study_template(&studies);
        }
        if let Some(drawing) = self.drawing_templates.show(ctx, "Drawing templates") {
            self.load_drawing_template(drawing);
        }
        self.top_toolbar.indicator_templates = self
            .study_templates
            .templates
            .entries()
            .keys()
            .map(|name| (name.clone(), name.clone()))
            .collect();
        let dialog = &self.indicator_dialog;
        self.top_toolbar.favorite_indicators = dialog
            .indicators()
            .iter()
            .filter(|info| dialog.favorites.contains(&info.id))
            .map(|info| (info.id.clone(), info.name.clone()))
            .collect();
        self.drawing_toolbar.state.templates = self
            .drawing_templates
            .templates
            .entries()
            .iter()
            .map(
                |(name, template)| egui_charts::ui::drawing_toolbar::DrawingTemplate {
                    id: name.clone(),
                    name: name.clone(),
                    drawing_type: template.settings.tool_type.as_str().into(),
                    is_default: false,
                },
            )
            .collect();
    }
}
