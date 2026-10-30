//! Exact native plane cuts and convex planar panel construction.
use super::{META, error, nested};
use crate::{
    Result, Session,
    woodwork::{Cabinet, Panel},
};
use serde_json::{Value, json};
use solidcraft_geom::{Plane, Vec3};
fn text<'a>(p: &'a Value, key: &str) -> Result<&'a str> {
    p.get(key).and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() < 256).ok_or_else(|| error(format!("{key}: Körpername fehlt.")))
}
fn vector(v: &Value) -> Result<Vec3> {
    let a: [f64; 3] = serde_json::from_value(v.clone()).map_err(|_| error("Drei Koordinaten erforderlich."))?;
    if a.iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
        return Err(error("Ungültige Koordinaten."));
    }
    Ok(Vec3::new(a[0], a[1], a[2]))
}
fn body(s: &Session, name: &str) -> Result<solidcraft_kernel::Body> {
    s.model.state().body(name).map(|b| b.body.clone()).ok_or_else(|| error(format!("Körper fehlt: {name}")))
}
fn face(s: &Session, name: &str, index: usize) -> Result<(Vec3, Vec3)> {
    let b = body(s, name)?;
    let m = solidcraft_kernel::measure(&b)?;
    let f = b.faces((b.size() * 1e-6).max(1e-5))?.into_iter().find(|f| f.index == index).ok_or_else(|| error("Fläche nicht gefunden."))?;
    let mut n = f.plane_normal.and_then(|n| n.normalized()).ok_or_else(|| error("Eine ebene Außenfläche auswählen."))?;
    if n.dot(f.centroid - m.centroid) < 0. {
        n = -n;
    }
    Ok((f.centroid, n))
}
/// Keep n·(x-origin)>=0. The surviving body keeps its original name.
fn clip(s: &mut Session, name: &str, origin: Vec3, n: Vec3) -> Result<bool> {
    s.refresh();
    let b = body(s, name)?;
    let n = n.normalized().ok_or_else(|| error("Ebenennormale darf nicht null sein."))?;
    let mesh = b.tessellate((b.size() * 1e-6).max(1e-5))?;
    let tol = (b.size() * 1e-8).max(1e-6);
    let lo = mesh.positions.iter().map(|v| n.dot(*v - origin)).fold(f64::INFINITY, f64::min);
    let hi = mesh.positions.iter().map(|v| n.dot(*v - origin)).fold(f64::NEG_INFINITY, f64::max);
    if lo >= -tol {
        return Ok(false);
    }
    if hi <= tol {
        return Err(error("Die Trimmung würde die gesamte Platte entfernen."));
    }
    let split = nested(s, "solid.split_body", &json!({"body":name,"plane":{"origin":origin,"normal":n}}))?;
    let mut names = vec![name.to_string()];
    if let Some(a) = split["created_bodies"].as_array() {
        for v in a {
            if let Some(v) = v.as_str()
                && !names.iter().any(|n| n == v)
            {
                names.push(v.into());
            }
        }
    }
    let mut keep = Vec::new();
    let mut remove = Vec::new();
    for part in names {
        let m = solidcraft_kernel::measure(&body(s, &part)?)?;
        if n.dot(m.centroid - origin) >= -tol {
            keep.push(part);
        } else {
            remove.push(part);
        }
    }
    if keep.len() != 1 || remove.is_empty() {
        return Err(error("Die Ebene muss genau ein zusammenhängendes Restteil ergeben."));
    }
    nested(s, "solid.remove", &json!({"bodies":remove}))?;
    if let Some(kept) = keep.first()
        && kept != name
    {
        nested(s, "body.rename", &json!({"body":kept,"name":name}))?;
    }
    Ok(true)
}
pub(super) fn trim(s: &mut Session, p: &Value) -> Result<Value> {
    let name = text(p, "body")?;
    let _ = body(s, name)?;
    if let Some(tool) = p.get("tool").and_then(Value::as_str) {
        if tool == name {
            return Err(error("Platte und Grenzkörper müssen verschieden sein."));
        }
        let _ = body(s, tool)?;
        nested(s, "solid.combine", &json!({"target":name,"tools":[tool],"operation":"cut","keep_tools":true}))?;
    } else {
        let origin = vector(&p["plane"]["origin"])?;
        let n = vector(&p["plane"]["normal"])?;
        let positive = p.get("keep_positive").and_then(Value::as_bool).unwrap_or(true);
        clip(s, name, origin, if positive { n } else { -n })?;
    }
    Ok(json!({"body":name,"trimmed":true}))
}
pub(super) fn miter(s: &mut Session, p: &Value) -> Result<Value> {
    let a = text(p, "a")?;
    let b = text(p, "b")?;
    if a == b {
        return Err(error("Zwei verschiedene Platten auswählen."));
    }
    let fa = p.get("face_a").and_then(Value::as_u64).ok_or_else(|| error("Außenfläche A fehlt."))?;
    let fb = p.get("face_b").and_then(Value::as_u64).ok_or_else(|| error("Außenfläche B fehlt."))?;
    let (pa, na) = face(s, a, usize::try_from(fa).map_err(|_| error("Flächenindex zu groß."))?)?;
    let (pb, nb) = face(s, b, usize::try_from(fb).map_err(|_| error("Flächenindex zu groß."))?)?;
    if na.cross(nb).len() < 1e-6 {
        return Err(error("Parallele Außenflächen definieren keine Gehrung."));
    }
    let n = na - nb;
    let origin = n * ((na.dot(pa) - nb.dot(pb)) / n.dot(n));
    let ca = clip(s, a, origin, n)?;
    let cb = clip(s, b, origin, -n)?;
    if !ca || !cb {
        return Err(error("Die gewählten Außenflächen ergeben keine beidseitige Gehrung. Überlappende Platten und die äußeren Breitflächen wählen."));
    }
    Ok(json!({"bodies":[a,b],"plane":{"origin":origin,"normal":n.normalized()}}))
}
pub(super) fn panelize(s: &mut Session, p: &Value) -> Result<Value> {
    let source = text(p, "body")?.to_string();
    let thick = p.get("thickness").and_then(Value::as_f64).unwrap_or(18.);
    if !thick.is_finite() || !(1. ..=100.).contains(&thick) {
        return Err(error("Plattenstärke: 1 bis 100 mm."));
    }
    let src = body(s, &source)?;
    let measure = solidcraft_kernel::measure(&src)?;
    let faces = src.faces((src.size() * 1e-6).max(1e-5))?;
    if !(4..=12).contains(&faces.len()) {
        return Err(error("Grundkörper: 4 bis 12 ebene Flächen erforderlich."));
    }
    let mesh = src.tessellate((src.size() * 1e-6).max(1e-5))?;
    let tol = (src.size() * 1e-7).max(1e-5);
    let mut planes = Vec::new();
    for f in faces {
        let (point, n) = face(s, &source, f.index)?;
        if mesh.positions.iter().any(|v| n.dot(*v - point) > tol) {
            return Err(error("Nur konvexe Grundkörper ohne Aussparungen werden unterstützt."));
        }
        if n.dot(point - measure.centroid) <= thick + tol {
            return Err(error("Plattenstärke zu groß für diesen Grundkörper."));
        }
        planes.push((f.index, point, n));
    }
    let opening = match p.get("opening") {
        Some(v) => Some(v.as_u64().and_then(|v| usize::try_from(v).ok()).ok_or_else(|| error("Ungültige Öffnungsfläche."))?),
        None => planes.iter().min_by(|a, b| a.2.y.total_cmp(&b.2.y)).map(|a| a.0),
    };
    if let Some(i) = opening
        && !planes.iter().any(|v| v.0 == i)
    {
        return Err(error("Öffnungsfläche nicht gefunden."));
    }
    let mut records = s.doc.metadata.get(META).and_then(Value::as_object).cloned().unwrap_or_default();
    if records.len() >= 100 {
        return Err(error("Höchstens 100 Korpusse."));
    }
    let id = (1..=1000).find(|i| !records.contains_key(&i.to_string())).ok_or_else(|| error("Keine freie Korpusnummer."))?;
    let material = p.get("material").and_then(Value::as_str).unwrap_or("Holzplatte");
    let density = p.get("density").and_then(Value::as_f64).unwrap_or(0.65);
    let c = Cabinet { material: material.into(), density, band: 0., ..Default::default() };
    c.validate().map_err(error)?;
    let material_name = format!("Holz{id}_Konturmaterial");
    nested(s, "material.create", &json!({"name":material_name,"density_g_cm3":density,"color":"#ba8e59"}))?;
    let mut made = Vec::new();
    for (index, point, n) in planes.iter().filter(|x| Some(x.0) != opening) {
        let copy = nested(s, "solid.move", &json!({"bodies":[source],"copy":true,"translate":[0,0,0]}))?;
        let from = copy["created_bodies"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .ok_or_else(|| error("Grundkörper konnte nicht kopiert werden."))?;
        let name = format!("Holz {id} · Konturplatte {:02}", index + 1);
        nested(s, "body.rename", &json!({"body":from,"name":name}))?;
        clip(s, &name, *point - *n * thick, *n)?;
        for (j, q, m) in planes.iter().filter(|x| Some(x.0) != opening) {
            if j == index {
                continue;
            }
            let normal = *n - *m;
            if normal.len() < 1e-8 {
                continue;
            }
            let origin = normal * ((n.dot(*point) - m.dot(*q)) / normal.dot(normal));
            clip(s, &name, origin, normal)?;
        }
        let plane = Plane::from_normal(*point, *n).ok_or_else(|| error("Ungültige Flächenebene."))?;
        made.push(json!({"name":name,"normal":n,"u":plane.x,"v":plane.y}));
    }
    nested(s, "material.assign", &json!({"bodies":made.iter().filter_map(|v|v["name"].as_str()).collect::<Vec<_>>(),"material":material_name}))?;
    records.insert(id.to_string(), json!({"kind":"panelized","config":c,"panels":made,"source":source}));
    nested(s, "document.metadata.set", &json!({"key":META,"value":records}))?;
    Ok(json!({"instance":id,"panels":made.len(),"opening":opening,"source_preserved":true}))
}
pub(super) fn bom(s: &mut Session, p: &Value, record: Value, id: u64) -> Result<Value> {
    let c: Cabinet = serde_json::from_value(record["config"].clone()).map_err(|e| error(e.to_string()))?;
    c.validate().map_err(error)?;
    let entries = record["panels"].as_array().filter(|a| a.len() <= 12).ok_or_else(|| error("Ungültige Plattenzuordnung."))?;
    let mut panels = Vec::new();
    for entry in entries {
        let name = text(entry, "name")?;
        let b = body(s, name)?;
        let mesh = b.tessellate((b.size() * 1e-6).max(1e-5))?;
        let u = vector(&entry["u"])?;
        let v = vector(&entry["v"])?;
        let n = vector(&entry["normal"])?;
        let span = |axis: Vec3| {
            let lo = mesh.positions.iter().map(|p| p.dot(axis)).fold(f64::INFINITY, f64::min);
            let hi = mesh.positions.iter().map(|p| p.dot(axis)).fold(f64::NEG_INFINITY, f64::max);
            hi - lo
        };
        let a = span(u);
        let bb = span(v);
        let t = span(n);
        let m = solidcraft_kernel::measure(&b)?;
        panels.push(Panel {
            name: name.into(),
            length: a.max(bb),
            width: a.min(bb),
            thickness: t,
            origin: [m.bbox.min.x, m.bbox.min.y, m.bbox.min.z],
            size: [m.bbox.max.x - m.bbox.min.x, m.bbox.max.y - m.bbox.min.y, m.bbox.max.z - m.bbox.min.z],
            size_expr: Default::default(),
            origin_expr: Default::default(),
            edges_length: 0,
            edges_width: 0,
            back: false,
            actual_volume_mm3: Some(m.volume),
            shaped: true,
        });
    }
    let csv = c.csv_for_language(&panels, p.get("language").and_then(Value::as_str).unwrap_or("de"));
    if let Some(path) = p.get("path").and_then(Value::as_str) {
        solidcraft_io::vfs::write(path, csv.as_bytes()).map_err(|e| error(e.to_string()))?;
    }
    Ok(json!({"instance":id,"panels":panels,"csv":csv,"contour_check_required":true,"material":c.material,"density":c.density}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(s: &mut Session, c: &str, p: &str) -> Value {
        s.execute(c, &p.parse().unwrap()).unwrap()
    }
    fn volume(s: &Session, name: &str) -> f64 {
        solidcraft_kernel::measure(&body(s, name).unwrap()).unwrap().volume
    }
    #[test]
    fn plane_cut_is_atomic_and_reversible() {
        let mut s = Session::default();
        run(&mut s, "solid.box", r#"{"length":100,"width":80,"height":60,"body_name":"Panel"}"#);
        let count = s.undo.len();
        run(&mut s, "woodwork.trim", r#"{"body":"Panel","plane":{"origin":[50,0,0],"normal":[1,0,0]}}"#);
        assert!((volume(&s, "Panel") - 240000.).abs() < 1e-4);
        assert_eq!(s.undo.len(), count + 1);
        run(&mut s, "edit.undo", "{}");
        assert!((volume(&s, "Panel") - 480000.).abs() < 1e-4);
        run(&mut s, "edit.redo", "{}");
        assert!((volume(&s, "Panel") - 240000.).abs() < 1e-4);
        let n = s.undo.len();
        assert!(s.execute("woodwork.trim", &json!({"body":"Panel","plane":{"origin":[500,0,0],"normal":[1,0,0]}})).is_err());
        assert_eq!(n, s.undo.len());
        assert!((volume(&s, "Panel") - 240000.).abs() < 1e-4);
    }
    #[test]
    fn curved_source_is_refused_without_modification() {
        let mut s = Session::default();
        run(&mut s, "solid.sphere", r#"{"radius":100,"body_name":"Sphere"}"#);
        let n = s.undo.len();
        assert!(s.execute("woodwork.panelize", &json!({"body":"Sphere","thickness":18})).is_err());
        assert_eq!(n, s.undo.len());
        assert_eq!(s.model.state().bodies.len(), 1);
    }
    #[test]
    fn zero_normal_is_refused() {
        let mut s = Session::default();
        run(&mut s, "solid.box", r#"{"length":100,"width":100,"height":100,"body_name":"Panel"}"#);
        assert!(s.execute("woodwork.trim", &json!({"body":"Panel","plane":{"origin":[0,0,0],"normal":[0,0,0]}})).is_err());
        assert_eq!(s.model.state().bodies.len(), 1);
    }
}
