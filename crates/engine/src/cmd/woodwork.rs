#[path = "woodwork_advanced.rs"]
mod advanced;
// Native furniture commands; the outer command owns the undo transaction.
use super::CommandSpec;
use crate::{EngineError, Result, Session, woodwork::Cabinet};
use serde_json::{Value, json};
pub(super) const META: &str = "org.solidcraft.holz";
pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("woodwork.panelize", "Panelize Furniture Body", advanced::panelize)
        .at("UTILITIES", "ADD-INS")
        .icon("box")
        .enabled(super::not_in_sketch)
        .params("body; thickness: mm; opening?: face index (default face pointing -Y); material?, density? → mitred panels of a convex planar body"),
    CommandSpec::new("woodwork.miter", "Furniture Miter", advanced::miter)
        .at("UTILITIES", "ADD-INS")
        .icon("chamfer")
        .enabled(super::not_in_sketch)
        .params("a,b: body names; face_a,face_b: outside planar face indices → equal-angle bisector cuts in one undo step"),
    CommandSpec::new("woodwork.trim", "Trim Furniture Panel", advanced::trim)
        .at("UTILITIES", "ADD-INS")
        .icon("split_body")
        .enabled(super::not_in_sketch)
        .params("body; tool?: boundary body (subtract, retained); otherwise plane:{origin,normal}, keep_positive?:bool (default true)"),
    CommandSpec::new("woodwork.cabinet", "Furniture Cabinet", create)
        .icon("box")
        .at("UTILITIES", "ADD-INS")
        .enabled(super::not_in_sketch)
        .params("config?: cabinet settings; origin?: [x,y,z] in mm; appends parametric panels in one undo step"),
    CommandSpec::new("woodwork.bounds", "Furniture Reference Body", bounds)
        .icon("measure")
        .noundo()
        .params("body: axis-aligned box name → width, height, depth, origin"),
    CommandSpec::new("woodwork.bom", "Furniture Cut List", bom)
        .icon("list")
        .at("UTILITIES", "ADD-INS")
        .noundo()
        .params("instance?: number (default most recent); path?: CSV destination → current panel sizes and CSV"),
];
pub(super) fn error(s: impl Into<String>) -> EngineError {
    EngineError::Other(s.into())
}
pub(super) fn nested(s: &mut Session, id: &str, p: &Value) -> Result<Value> {
    let spec = crate::find_command(id).ok_or_else(|| error(format!("Unknown command {id}")))?;
    if spec.reads_model && s.model.is_stale() {
        s.refresh();
    }
    spec.run_nested(s, p)
}
fn bounds(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p.get("body").and_then(Value::as_str).ok_or_else(|| error("Einen Quader auswählen."))?;
    let state = s.world_state();
    let b = state.body(name).ok_or_else(|| error("Grundkörper nicht gefunden."))?;
    let m = solidcraft_kernel::measure(&b.body)?;
    let d = m.bbox.max - m.bbox.min;
    if d.x <= 0. || d.y <= 0. || d.z <= 0. || (m.volume - d.x * d.y * d.z).abs() > (m.volume.abs() * 1e-7).max(1e-5) {
        return Err(error("Diese Maßübernahme unterstützt achsenparallele Quader. Für schräge Körper „Grundkörper in Platten“ verwenden."));
    }
    Ok(json!({"width":d.x,"depth":d.y,"height":d.z,"origin":[m.bbox.min.x,m.bbox.min.y,m.bbox.min.z]}))
}
fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let c: Cabinet = serde_json::from_value(p.get("config").cloned().unwrap_or_else(|| json!({}))).map_err(|e| error(e.to_string()))?;
    c.validate().map_err(error)?;
    let mut records = s.doc.metadata.get(META).and_then(Value::as_object).cloned().unwrap_or_default();
    if records.len() >= 100 {
        return Err(error("Höchstens 100 Möbelkorpusse pro Dokument."));
    }
    let id = (1..=1000)
        .find(|i| !records.contains_key(&i.to_string()) && !s.doc.params.iter().any(|p| p.name.starts_with(&format!("Holz{i}_"))))
        .ok_or_else(|| error("Kein freier Korpusname verfügbar."))?;
    let prefix = format!("Holz{id}_");
    let body_prefix = format!("Holz {id} · ");
    let origin: [f64; 3] =
        serde_json::from_value(p.get("origin").cloned().unwrap_or_else(|| json!([0., 0., 0.]))).map_err(|e| error(e.to_string()))?;
    if origin.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(error("Ungültiger Ursprung."));
    }
    let source = c.script().map_err(error)?;
    let steps = source.get("commands").and_then(Value::as_array).ok_or_else(|| error("Ungültige Möbeldefinition."))?;
    for step in steps {
        let cmd = step.get("command").and_then(Value::as_str).unwrap_or("");
        if cmd == "file.new" {
            continue;
        }
        let mut args = step.get("params").cloned().unwrap_or_else(|| json!({}));
        if cmd == "parameters.add"
            && let Some(n) = args.get("name").and_then(Value::as_str)
        {
            args["name"] = json!(format!("{prefix}{n}"));
        }
        if cmd == "solid.box" {
            for key in ["length", "width", "height"] {
                if let Some(e) = args.get(key).and_then(Value::as_str) {
                    args[key] = json!(rewrite(e, &prefix));
                }
            }
            if let Some(v) = args.get("corner").and_then(Value::as_array) {
                let v = v
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        json!(format!("({}) + {} mm", rewrite(e.as_str().unwrap_or("0 mm"), &prefix), origin.get(i).copied().unwrap_or(0.)))
                    })
                    .collect::<Vec<_>>();
                args["corner"] = json!(v);
            }
            for key in ["name", "body_name"] {
                if let Some(n) = args.get(key).and_then(Value::as_str) {
                    args[key] = json!(format!("{body_prefix}{n}"));
                }
            }
        }
        if cmd == "material.create" {
            args["name"] = json!(format!("{prefix}{}", c.material));
        }
        if cmd == "material.assign" {
            args["material"] = json!(format!("{prefix}{}", c.material));
            args["bodies"] = json!(c.panels().map_err(error)?.iter().map(|p| format!("{body_prefix}{}", p.name)).collect::<Vec<_>>());
        }
        nested(s, cmd, &args)?;
    }
    records.insert(id.to_string(), json!({"config":c,"prefix":body_prefix,"parameter_prefix":prefix}));
    nested(s, "document.metadata.set", &json!({"key":META,"value":records}))?;
    Ok(json!({"instance":id,"panels":c.panels().map_err(error)?.len()}))
}
fn rewrite(e: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if ["Breite", "Hoehe", "Tiefe", "Staerke", "Rueckwand", "Fuge"].contains(&word.as_str()) {
            out.push_str(prefix);
        }
        out.push_str(word);
        word.clear();
    };
    for ch in e.chars() {
        if ch.is_ascii_alphabetic() {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}
fn bom(s: &mut Session, p: &Value) -> Result<Value> {
    let records = s.doc.metadata.get(META).and_then(Value::as_object).ok_or_else(|| error("Noch kein Möbelkorpus in diesem Dokument."))?;
    let id = p
        .get("instance")
        .and_then(Value::as_u64)
        .or_else(|| records.keys().filter_map(|k| k.parse::<u64>().ok()).max())
        .ok_or_else(|| error("Kein Möbelkorpus gefunden."))?;
    let record = records.get(&id.to_string()).ok_or_else(|| error("Korpus nicht gefunden."))?;
    if record.get("kind").and_then(Value::as_str) == Some("panelized") {
        return advanced::bom(s, p, record.clone(), id);
    }
    let c: Cabinet = serde_json::from_value(record.get("config").cloned().unwrap_or(Value::Null)).map_err(|e| error(e.to_string()))?;
    let prefix = record.get("prefix").and_then(Value::as_str).ok_or_else(|| error("Fehlende Plattenzuordnung."))?;
    let mut panels = c.panels().map_err(error)?;
    let st = s.world_state();
    for panel in &mut panels {
        let b = st.body(&format!("{prefix}{}", panel.name)).ok_or_else(|| error(format!("Platte fehlt oder wurde umbenannt: {}", panel.name)))?;
        let m = solidcraft_kernel::measure(&b.body)?;
        let d = m.bbox.max - m.bbox.min;
        panel.origin = [m.bbox.min.x, m.bbox.min.y, m.bbox.min.z];
        panel.size = [d.x, d.y, d.z];
        if (m.volume - d.x * d.y * d.z).abs() > (m.volume.abs() * 1e-7).max(1e-5) {
            panel.shaped = true;
        }
        panel.actual_volume_mm3 = Some(m.volume);
        if panel.name.contains("Seite") || panel.name.contains("Trennwand") {
            panel.length = d.z;
            panel.width = d.y;
            panel.thickness = d.x;
        } else if panel.back || panel.name.contains("Tür") {
            panel.length = d.z;
            panel.width = d.x;
            panel.thickness = d.y;
        } else {
            panel.length = d.x;
            panel.width = d.y;
            panel.thickness = d.z;
        }
    }
    let csv = c.csv_for(&panels);
    if let Some(path) = p.get("path").and_then(Value::as_str) {
        solidcraft_io::vfs::write(path, csv.as_bytes()).map_err(|e| error(e.to_string()))?;
    }
    Ok(json!({"instance":id,"panels":panels,"csv":csv,"material":c.material,"density":c.density}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_undo_preserves_existing() {
        let mut s = Session::default();
        s.execute("solid.box", &json!({"length":10,"width":10,"height":10,"body_name":"Existing"})).unwrap();
        let n = s.undo.len();
        s.execute("woodwork.cabinet", &json!({})).unwrap();
        assert_eq!(s.undo.len(), n + 1);
        assert_eq!(s.model.state().bodies.len(), 9);
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.model.state().bodies.len(), 1);
        s.execute("edit.redo", &json!({})).unwrap();
        assert_eq!(s.model.state().bodies.len(), 9);
    }
    #[test]
    fn bom_follows_parameters() {
        let mut s = Session::default();
        s.execute("woodwork.cabinet", &json!({"config":{"dividers":1}})).unwrap();
        s.execute("parameters.change", &json!({"name":"Holz1_Breite","expression":"1000 mm"})).unwrap();
        let v = s.execute("woodwork.bom", &json!({})).unwrap();
        assert_eq!(v["panels"][2]["length"], 964.);
    }
    #[test]
    fn failure_rolls_back_and_instances_work() {
        let mut s = Session::default();
        assert!(s.execute("woodwork.cabinet", &json!({"config":{"width":0}})).is_err());
        assert!(s.model.state().bodies.is_empty());
        s.execute("woodwork.cabinet", &json!({})).unwrap();
        s.execute("woodwork.cabinet", &json!({"origin":[1000,0,0]})).unwrap();
        assert_eq!(s.model.state().bodies.len(), 16);
    }
}
