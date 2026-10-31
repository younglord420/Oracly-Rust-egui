//! Serializable inputs and presentation of one study.

use crate::studies::{
    ConfiguredStudy, Indicator, IndicatorFactory, InputValue, StudySettings, StudySource,
};

/// Serializable inputs and presentation of one study. Hosts may resolve `name` through their
/// own study factory when they supply custom studies.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StudyTemplate {
    /// Study factory name.
    pub name: String,
    /// Typed inputs keyed by their stable study-local identifiers.
    pub inputs: Vec<(String, InputValue)>,
    /// Plot styles, precision and interval visibility.
    pub settings: StudySettings,
    /// Whether the study is visible.
    pub visible: bool,
}

impl StudyTemplate {
    /// Capture the current calculation inputs and presentation, without computed values.
    /// Chart-local study source ids are left as-is; use [`capture_set`](Self::capture_set) when
    /// saving a related group so sources become indices into that group.
    pub fn from_study(study: &dyn Indicator) -> Self {
        Self {
            name: study.name().into(),
            inputs: study
                .inputs()
                .into_iter()
                .map(|input| (input.id, input.value))
                .collect(),
            settings: study.settings(),
            visible: study.is_visible(),
        }
    }

    /// Capture every study with [`StudySource::Study`] ids rewritten to positions in the returned
    /// vector. Unknown parent ids become `usize::MAX`.
    pub fn capture_set<'a, I>(studies: I) -> Vec<Self>
    where
        I: IntoIterator<Item = (usize, &'a dyn Indicator)>,
    {
        let studies: Vec<_> = studies.into_iter().collect();
        studies
            .iter()
            .map(|(_, study)| {
                let mut template = Self::from_study(*study);
                for (_, value) in &mut template.inputs {
                    if let InputValue::Source(StudySource::Study { id: parent, .. }) = value {
                        *parent = studies
                            .iter()
                            .position(|(source, _)| *source == *parent)
                            .unwrap_or(usize::MAX);
                    }
                }
                template
            })
            .collect()
    }

    /// Rewrite [`StudySource::Study`] ids from template-set positions through `ids`. A position
    /// outside `ids`, or `usize::MAX`, becomes `usize::MAX`.
    pub fn remap_sources(&mut self, ids: &[usize]) {
        for (_, value) in &mut self.inputs {
            if let InputValue::Source(StudySource::Study { id, .. }) = value {
                *id = ids.get(*id).copied().unwrap_or(usize::MAX);
            }
        }
    }

    /// Reconstruct a built-in study. Unknown names or invalid inputs leave the host chart intact.
    /// Hosts reconstruct the entire set successfully before replacing their current studies.
    pub fn create(&self) -> Result<Box<dyn Indicator>, String> {
        self.create_with(&IndicatorFactory::new())
    }

    /// Reconstruct a study through `factory`, which resolves [`name`](Self::name) to a built-in
    /// or host-registered study. Fails like [`create`](Self::create). Custom studies the host
    /// registered on `factory` restore the same way as built-ins.
    pub fn create_with(&self, factory: &IndicatorFactory) -> Result<Box<dyn Indicator>, String> {
        let study = factory
            .create(&self.name)
            .ok_or_else(|| format!("Unknown study: {}", self.name))?;
        let mut study = ConfiguredStudy::new(study);
        study
            .apply_settings(&self.inputs, self.settings.clone())
            .map_err(str::to_owned)?;
        study.set_visible(self.visible);
        Ok(Box::new(study))
    }

    /// Reconstruct every template through `factory`. Fails before returning if any name is
    /// unknown or any input is invalid, so the host can keep its current studies.
    pub fn create_set_with(
        templates: &[Self],
        factory: &IndicatorFactory,
    ) -> Result<Vec<Box<dyn Indicator>>, String> {
        templates
            .iter()
            .map(|template| template.create_with(factory))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::{ConfiguredStudy, RSI, SMA};

    #[test]
    fn capture_set_rewrites_sources_to_template_positions() {
        let sma = ConfiguredStudy::new(Box::new(SMA::new(5)));
        let mut rsi = ConfiguredStudy::new(Box::new(RSI::new(5)));
        rsi.set_input(
            "source",
            InputValue::Source(StudySource::Study { id: 42, plot: 0 }),
        )
        .unwrap();
        let templates =
            StudyTemplate::capture_set([(42, &sma as &dyn Indicator), (7, &rsi as &dyn Indicator)]);
        assert_eq!(templates[0].name, "SMA");
        let InputValue::Source(StudySource::Study { id, plot }) = templates[1]
            .inputs
            .iter()
            .find(|(name, _)| name == "source")
            .unwrap()
            .1
            .clone()
        else {
            panic!("expected study source");
        };
        assert_eq!((id, plot), (0, 0));
        let mut restored = templates[1].clone();
        restored.remap_sources(&[100, 200]);
        let InputValue::Source(StudySource::Study { id, .. }) = restored
            .inputs
            .iter()
            .find(|(name, _)| name == "source")
            .unwrap()
            .1
            .clone()
        else {
            panic!("expected study source");
        };
        assert_eq!(id, 100);
    }
}
