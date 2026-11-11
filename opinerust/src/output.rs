//! An output sink that keeps the drawings a script creates.
//!
//! Pinecone's [`DefaultPineOutput`] stores labels, boxes, lines, linefills,
//! polylines and tables behind private maps with no way to enumerate them, so a
//! host that only sees `plots()` cannot draw Smart-Money-Concepts order blocks or
//! fair-value gaps. The traits themselves are public, so this sink keeps the
//! handles in maps the bridge can read while delegating every non-drawing output
//! (plots, logs, inputs, metadata, fills, globals) to a `DefaultPineOutput`.
//!
//! Handle lifetime matches Pine: drawings are **persistent** across bars and are
//! not cleared by [`PineOutput::clear`], because a `var`-held id created on an
//! early bar is mutated on a later one.

use pine_core::{
    AlertCondition, AlertConditionOutput, BoxOutput, Color, DefaultPineOutput, DrawingOutput,
    FillObject, FillOutput, GlobalContext, GlobalOutput, Indicator, Input, InputOutput, Label,
    LabelOutput, Library, LineObject, LineOutput, LinefillObject, LogEntry, LogLevel, LogOutput,
    MetadataOutput, PineBox, PineOutput, Plot, PlotOutput, Plotarrow, Plotbar, Plotcandle,
    Plotchar, Plotshape, PolylineObject, Table, TableOutput,
};
use std::collections::BTreeMap;

/// Every handle a script created, in creation order.
#[derive(Clone, Debug, Default)]
pub struct Drawings {
    /// `label.new` handles, keyed by the id the script received.
    pub labels: BTreeMap<usize, Label>,
    /// `box.new` handles, keyed by the id the script received.
    pub boxes: BTreeMap<usize, PineBox>,
    /// `line.new`/`hline` handles, keyed by the id the script received.
    pub lines: BTreeMap<usize, LineObject>,
    /// `table.new` handles, keyed by the id the script received.
    pub tables: BTreeMap<usize, Table>,
    /// `linefill.new` handles.
    pub linefills: BTreeMap<usize, LinefillObject>,
    /// `polyline.new` handles.
    pub polylines: BTreeMap<usize, PolylineObject>,
}

/// Output sink for one run: plain outputs plus the drawings the run created.
#[derive(Clone, Debug, Default)]
pub struct DrawingOutputSink {
    /// Plots, logs, inputs, metadata, fills and globals.
    base: DefaultPineOutput,
    /// Alignment to the default output before drawings are added.
    drawings: Drawings,
    next_label: usize,
    next_box: usize,
    next_line: usize,
    next_table: usize,
    next_linefill: usize,
    next_polyline: usize,
}

impl DrawingOutputSink {
    /// The drawings created so far.
    pub fn drawings(&self) -> &Drawings {
        &self.drawings
    }

    /// The non-drawing output of this run.
    #[cfg(test)]
    pub fn base(&self) -> &DefaultPineOutput {
        &self.base
    }
}

impl PineOutput for DrawingOutputSink {
    fn clear(&mut self) {
        // Drawings are persistent handles, so they outlive the bar. Only the
        // per-bar streams reset.
        self.base.clear();
    }
}

impl LogOutput for DrawingOutputSink {
    fn add_log(&mut self, level: LogLevel, message: String) {
        self.base.add_log(level, message);
    }

    fn get_logs(&self) -> &[LogEntry] {
        self.base.get_logs()
    }
}

impl PlotOutput for DrawingOutputSink {
    fn add_plot(&mut self, plot: Plot) {
        self.base.add_plot(plot);
    }

    fn plots(&self) -> &[Plot] {
        self.base.plots()
    }

    fn add_plotarrow(&mut self, arrow: Plotarrow) {
        self.base.add_plotarrow(arrow);
    }

    fn plotarrows(&self) -> &[Plotarrow] {
        self.base.plotarrows()
    }

    fn add_plotbar(&mut self, bar: Plotbar) {
        self.base.add_plotbar(bar);
    }

    fn plotbars(&self) -> &[Plotbar] {
        self.base.plotbars()
    }

    fn add_plotcandle(&mut self, candle: Plotcandle) {
        self.base.add_plotcandle(candle);
    }

    fn plotcandles(&self) -> &[Plotcandle] {
        self.base.plotcandles()
    }

    fn add_plotchar(&mut self, char: Plotchar) {
        self.base.add_plotchar(char);
    }

    fn plotchars(&self) -> &[Plotchar] {
        self.base.plotchars()
    }

    fn add_plotshape(&mut self, shape: Plotshape) {
        self.base.add_plotshape(shape);
    }

    fn plotshapes(&self) -> &[Plotshape] {
        self.base.plotshapes()
    }
}

impl LabelOutput for DrawingOutputSink {
    fn add_label(&mut self, label: Label) -> usize {
        let id = self.next_label;
        self.next_label += 1;
        self.drawings.labels.insert(id, label);
        id
    }

    fn get_label(&self, id: usize) -> Option<&Label> {
        self.drawings.labels.get(&id)
    }

    fn get_label_mut(&mut self, id: usize) -> Option<&mut Label> {
        self.drawings.labels.get_mut(&id)
    }

    fn delete_label(&mut self, id: usize) -> bool {
        self.drawings.labels.remove(&id).is_some()
    }
}

impl BoxOutput for DrawingOutputSink {
    fn add_box(&mut self, box_obj: PineBox) -> usize {
        let id = self.next_box;
        self.next_box += 1;
        self.drawings.boxes.insert(id, box_obj);
        id
    }

    fn get_box(&self, id: usize) -> Option<&PineBox> {
        self.drawings.boxes.get(&id)
    }

    fn get_box_mut(&mut self, id: usize) -> Option<&mut PineBox> {
        self.drawings.boxes.get_mut(&id)
    }

    fn delete_box(&mut self, id: usize) -> bool {
        self.drawings.boxes.remove(&id).is_some()
    }
}

impl LineOutput for DrawingOutputSink {
    fn add_line(&mut self, line: LineObject) -> usize {
        let id = self.next_line;
        self.next_line += 1;
        self.drawings.lines.insert(id, line);
        id
    }

    fn get_line(&self, id: usize) -> Option<&LineObject> {
        self.drawings.lines.get(&id)
    }

    fn get_line_mut(&mut self, id: usize) -> Option<&mut LineObject> {
        self.drawings.lines.get_mut(&id)
    }

    fn delete_line(&mut self, id: usize) -> bool {
        self.drawings.lines.remove(&id).is_some()
    }
}

impl TableOutput for DrawingOutputSink {
    fn add_table(&mut self, table: Table) -> usize {
        let id = self.next_table;
        self.next_table += 1;
        self.drawings.tables.insert(id, table);
        id
    }

    fn get_table(&self, id: usize) -> Option<&Table> {
        self.drawings.tables.get(&id)
    }

    fn get_table_mut(&mut self, id: usize) -> Option<&mut Table> {
        self.drawings.tables.get_mut(&id)
    }

    fn delete_table(&mut self, id: usize) -> bool {
        self.drawings.tables.remove(&id).is_some()
    }
}

impl DrawingOutput for DrawingOutputSink {
    fn add_linefill(&mut self, linefill: LinefillObject) -> usize {
        let id = self.next_linefill;
        self.next_linefill += 1;
        self.drawings.linefills.insert(id, linefill);
        id
    }

    fn get_linefill(&self, id: usize) -> Option<&LinefillObject> {
        self.drawings.linefills.get(&id)
    }

    fn get_linefill_mut(&mut self, id: usize) -> Option<&mut LinefillObject> {
        self.drawings.linefills.get_mut(&id)
    }

    fn delete_linefill(&mut self, id: usize) -> bool {
        self.drawings.linefills.remove(&id).is_some()
    }

    fn add_polyline(&mut self, polyline: PolylineObject) -> usize {
        let id = self.next_polyline;
        self.next_polyline += 1;
        self.drawings.polylines.insert(id, polyline);
        id
    }

    fn delete_polyline(&mut self, id: usize) -> bool {
        self.drawings.polylines.remove(&id).is_some()
    }
}

impl FillOutput for DrawingOutputSink {
    fn add_fill(&mut self, fill: FillObject) {
        self.base.add_fill(fill);
    }

    fn fills(&self) -> &[FillObject] {
        self.base.fills()
    }
}

impl GlobalOutput for DrawingOutputSink {
    fn set_bgcolor(&mut self, color: Option<Color>) {
        self.base.set_bgcolor(color);
    }

    fn set_barcolor(&mut self, color: Option<Color>) {
        self.base.set_barcolor(color);
    }

    fn global_context(&self) -> &GlobalContext {
        self.base.global_context()
    }
}

impl AlertConditionOutput for DrawingOutputSink {
    fn add_alertcondition(&mut self, alert: AlertCondition) {
        self.base.add_alertcondition(alert);
    }

    fn alertconditions(&self) -> &[AlertCondition] {
        self.base.alertconditions()
    }
}

impl MetadataOutput for DrawingOutputSink {
    fn set_indicator(&mut self, indicator: Indicator) {
        self.base.set_indicator(indicator);
    }

    fn indicator(&self) -> Option<&Indicator> {
        self.base.indicator()
    }

    fn set_library(&mut self, library: Library) {
        self.base.set_library(library);
    }

    fn library(&self) -> Option<&Library> {
        self.base.library()
    }
}

impl InputOutput for DrawingOutputSink {
    fn add_input(&mut self, input: Input) {
        self.base.add_input(input);
    }

    fn inputs(&self) -> &[Input] {
        self.base.inputs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pine_core::{Extend, LineStyle as PineLineStyle, XLoc};

    fn box_obj(left: f64, right: f64) -> PineBox {
        PineBox {
            left,
            top: 10.0,
            right,
            bottom: 5.0,
            border_color: None,
            border_width: 1.0,
            border_style: "solid".into(),
            extend: "none".into(),
            xloc: "bar_index".into(),
            bgcolor: None,
            text: String::new(),
            text_size: 0.0,
            text_color: None,
            text_halign: String::new(),
            text_valign: String::new(),
            text_wrap: String::new(),
            text_font_family: String::new(),
        }
    }

    #[test]
    fn boxes_keep_their_handles_across_clears() {
        let mut sink = DrawingOutputSink::default();
        let first = sink.add_box(box_obj(0.0, 1.0));
        sink.clear();
        let second = sink.add_box(box_obj(2.0, 3.0));
        assert_eq!((first, second), (0, 1), "ids stay unique and rising");
        assert_eq!(sink.drawings().boxes.len(), 2, "a var-held box survives");
    }

    #[test]
    fn deleted_boxes_release_their_handle() {
        let mut sink = DrawingOutputSink::default();
        let id = sink.add_box(box_obj(0.0, 1.0));
        assert!(sink.delete_box(id));
        assert!(sink.get_box(id).is_none());
        assert!(!sink.delete_box(id), "a second delete reports nothing");
    }

    #[test]
    fn plots_reset_but_drawings_do_not() {
        let mut sink = DrawingOutputSink::default();
        sink.add_line(LineObject {
            x1: 0.0,
            y1: 1.0,
            x2: 1.0,
            y2: 2.0,
            xloc: XLoc::BarIndex,
            extend: Extend::None,
            color: None,
            style: PineLineStyle::StyleSolid,
            width: 1.0,
        });
        sink.add_plot(Plot {
            series: 1.0,
            title: "p".into(),
            ..Plot::default()
        });
        sink.clear();
        assert!(sink.plots().is_empty(), "plots are per bar");
        assert_eq!(sink.drawings().lines.len(), 1, "lines persist");
    }
}
