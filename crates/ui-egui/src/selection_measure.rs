//! Passive selection measurements, sharing the Inspect engine and document units.
use crate::SolveApp;
use serde_json::Value;
use solidcraft_engine::{Sel, doc::Document};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct Cache {
    doc: Option<Arc<Document>>,
    revision: u64,
    selection: Vec<Sel>,
    sketch: Option<u64>,
    result: Value,
}

fn is_curve(sel: &Sel) -> bool {
    matches!(sel, Sel::Edge { .. } | Sel::SketchCurve { .. } | Sel::ExternalSketchCurve { .. })
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v >= 0.0)
}

fn length(item: &Value) -> Option<&Value> {
    item.get("arc_length").or_else(|| item.get("circumference")).or_else(|| item.get("length_mm"))
}

fn text(app: &mut SolveApp) -> Option<String> {
    if !app.preferences.selection_measurements {
        return None;
    }
    let selections = &app.session.selection;
    if selections.is_empty() || selections.len() > 100 {
        return None;
    }
    let curves_only = selections.iter().all(is_curve);
    if !(curves_only || selections.len() == 1 && matches!(selections.first(), Some(Sel::Face { .. } | Sel::Profile { .. }))) {
        return None;
    }
    let cache = &mut app.selection_measure;
    if cache.doc.as_ref().is_none_or(|d| !Arc::ptr_eq(d, &app.session.doc))
        || cache.revision != app.session.revision
        || cache.selection != app.session.selection
        || cache.sketch != app.session.active_sketch
    {
        // Measure each curve independently: two selections mean summed length here,
        // while Inspect retains its existing distance-between-items semantics.
        let items: Vec<Value> = app
            .session
            .selection
            .iter()
            .enumerate()
            .filter(|(i, item)| !app.session.selection.iter().take(*i).any(|other| crate::selection::same_item(other, item)))
            .map(|(_, item)| {
                let measured = app.session.measure_items(std::slice::from_ref(item));
                measured.get("items").and_then(Value::as_array).and_then(|items| items.first()).cloned().unwrap_or(Value::Null)
            })
            .collect();
        cache.result = Value::Array(items);
        cache.doc = Some(app.session.doc.clone());
        cache.revision = app.session.revision;
        cache.selection = app.session.selection.clone();
        cache.sketch = app.session.active_sketch;
    }
    let items = cache.result.as_array()?.clone();
    if items.len() > 1 {
        let lengths: Option<Vec<f64>> = items.iter().map(|item| length(item).and_then(number)).collect();
        let total: f64 = lengths?.iter().sum();
        return total.is_finite().then(|| format!("{}: {} ({})", crate::i18n::tr("Full Length"), crate::prefs::show_mm(app, total, 1), items.len()));
    }
    let item = items.first()?.clone();
    let mut parts = Vec::new();
    // Round items expose their analytic arc length, avoiding tessellation chord error.
    for (label, value) in [
        (crate::i18n::tr("Length"), if curves_only { length(&item) } else { None }),
        (crate::i18n::tr("ui.radius"), item.get("radius")),
        (crate::i18n::tr("ui.diameter"), item.get("diameter")),
    ] {
        if let Some(value) = value.and_then(number) {
            parts.push(format!("{label}: {}", crate::prefs::show_mm(app, value, 1)));
        }
    }
    if let Some(area) = item.get("area_mm2").and_then(number) {
        parts.insert(0, format!("{}: {}", crate::i18n::tr("Area"), crate::prefs::show_mm(app, area, 2)));
    }
    (!parts.is_empty()).then(|| parts.join("  ·  "))
}

/// Reserve the right end of the history strip; return the remaining strip's right edge.
pub(crate) fn show(app: &mut SolveApp, ui: &mut egui::Ui, rect: egui::Rect) -> f32 {
    let Some(text) = text(app) else { return rect.right() };
    let t = crate::theme::Tokens::get();
    let width = (rect.width() - 260.0).clamp(0.0, 640.0);
    if width < 90.0 {
        return rect.right();
    }
    let galley = ui.painter().layout_no_wrap(text.clone(), egui::FontId::proportional(t.font_label), t.text);
    let width = width.min(galley.size().x + 24.0);
    let area = egui::Rect::from_min_max(egui::pos2(rect.right() - width, rect.top()), rect.right_bottom());
    ui.painter().line_segment([area.left_top(), area.left_bottom()], egui::Stroke::new(1.0, t.border));
    let mut child =
        ui.new_child(egui::UiBuilder::new().max_rect(area.shrink2(egui::vec2(10.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.set_clip_rect(area);
    child.add(egui::Label::new(egui::RichText::new(text.clone()).size(t.font_label).color(t.text)).truncate()).on_hover_text(text);
    crate::scenario::publish_rect("selection:measure", area);
    area.left()
}
