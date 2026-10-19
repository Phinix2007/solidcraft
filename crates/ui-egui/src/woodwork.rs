//! Furniture tools integrated into the native SolidCraft window.
use crate::{
    SolveApp,
    i18n::{format as localized_format, tr},
};
use egui::{Context, DragValue, Grid, Window};
use serde_json::{Value, json};
use solidcraft_engine::{Sel, woodwork::Cabinet};
#[derive(Default)]
pub struct State {
    pub open: bool,
    pub advanced: bool,
    source: String,
    faces: Vec<Value>,
    opening: usize,
    a: String,
    b: String,
    fa: usize,
    fb: usize,
    faces_a: Vec<Value>,
    faces_b: Vec<Value>,
    cut_body: String,
    cut_tool: String,
    cut_with_tool: bool,
    cut_origin: [f64; 3],
    cut_normal: [f64; 3],
    cut_negative: bool,
    pub config: Cabinet,
    origin: [f64; 3],
    reference: Option<String>,
    bom: Option<Value>,
    path: String,
    message: String,
}
pub fn show(app: &mut SolveApp, ctx: &Context) {
    let mut state = std::mem::take(&mut app.woodwork);
    if state.open {
        state.render(app, ctx);
    }
    app.woodwork = state;
}
fn mm(ui: &mut egui::Ui, name: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>) {
    ui.label(name);
    ui.add(DragValue::new(v).range(range).update_while_editing(false).speed(1.).suffix(" mm"));
    ui.end_row();
}
impl State {
    fn render(&mut self, app: &mut SolveApp, ctx: &Context) {
        let mut open = self.open;
        Window::new(tr("woodwork.text_000"))
            .open(&mut open)
            .default_width(560.)
            .default_height(780.)
            .default_pos(egui::pos2(1000., 140.))
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(680.).show(ui, |ui| {
                    self.advanced_ui(app, ui);
                    ui.separator();
                    ui.label(tr("woodwork.text_001"));
                    if ui.button(tr("woodwork.text_002")).clicked() {
                        let name = app.session.selection.iter().find_map(|s| if let Sel::Body { name } = s { Some(name.clone()) } else { None });
                        if let Some(name) = name {
                            match app.run("woodwork.bounds", json!({"body":name})) {
                                Ok(v) => {
                                    self.config.width = v["width"].as_f64().unwrap_or(self.config.width);
                                    self.config.height = v["height"].as_f64().unwrap_or(self.config.height);
                                    self.config.depth = v["depth"].as_f64().unwrap_or(self.config.depth);
                                    if let Ok(o) = serde_json::from_value(v["origin"].clone()) {
                                        self.origin = o;
                                    }
                                    self.reference = Some(name);
                                    self.message = tr("woodwork.text_003").into();
                                }
                                Err(e) => self.message = e,
                            }
                        } else {
                            self.message = tr("woodwork.text_004").into();
                        }
                    }
                    if let Some(name) = &self.reference {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(localized_format("woodwork.format_0", &[("name", name)]));
                            if ui.button(tr("woodwork.text_005")).clicked() {
                                let _ = app.run("browser.visibility", json!({"items":[name],"visible":false}));
                            }
                        });
                    }
                    ui.separator();
                    Grid::new("woodwork-size").num_columns(2).spacing([18., 7.]).show(ui, |ui| {
                        mm(ui, tr("Width"), &mut self.config.width, 100. ..=5000.);
                        mm(ui, tr("Height"), &mut self.config.height, 100. ..=5000.);
                        mm(ui, tr("woodwork.text_008"), &mut self.config.depth, 100. ..=2000.);
                        mm(ui, tr("woodwork.text_009"), &mut self.config.thickness, 3. ..=100.);
                        ui.label(tr("woodwork.text_010"));
                        ui.add(DragValue::new(&mut self.config.shelves).range(0..=20));
                        ui.end_row();
                        ui.label(tr("woodwork.text_011"));
                        ui.add(DragValue::new(&mut self.config.dividers).range(0..=4));
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.config.back, tr("woodwork.text_012"));
                    if self.config.back {
                        Grid::new("woodwork-back").show(ui, |ui| mm(ui, tr("woodwork.text_013"), &mut self.config.back_thickness, 2. ..=50.));
                    }
                    ui.checkbox(&mut self.config.doors, tr("woodwork.text_014"));
                    if self.config.doors {
                        Grid::new("woodwork-doors").show(ui, |ui| mm(ui, tr("woodwork.text_015"), &mut self.config.gap, 0.5..=10.));
                    }
                    ui.separator();
                    ui.label(tr("woodwork.text_016"));
                    ui.text_edit_singleline(&mut self.config.material);
                    Grid::new("woodwork-material").show(ui, |ui| {
                        ui.label(tr("ui.material_review.density"));
                        ui.add(DragValue::new(&mut self.config.density).range(0.05..=2.).speed(0.01).suffix(" g/cm³"));
                        ui.end_row();
                        mm(ui, tr("woodwork.text_018"), &mut self.config.band, 0. ..=5.);
                        mm(ui, tr("woodwork.text_019"), &mut self.config.allowance, 0. ..=30.);
                    });
                    ui.small(tr("woodwork.text_020"));
                    let valid = self.config.validate();
                    if let Err(e) = &valid {
                        ui.colored_label(egui::Color32::LIGHT_RED, localized_message(e));
                    }
                    if ui.add_enabled(valid.is_ok() && app.session.active_sketch.is_none(), egui::Button::new(tr("woodwork.text_021"))).clicked() {
                        match app.run("woodwork.cabinet", json!({"config":self.config,"origin":self.origin})) {
                            Ok(v) => {
                                self.message = localized_format(
                                    "woodwork.format_1",
                                    &[("instance", &v["instance"].to_string()), ("panels", &v["panels"].to_string())],
                                );
                                app.fit_view();
                                self.bom = app.run("woodwork.bom", json!({"language":app.preferences.language,"instance":v["instance"]})).ok();
                            }
                            Err(e) => self.message = e,
                        }
                    }
                    ui.separator();
                    ui.strong(tr("woodwork.text_022"));
                    ui.small(tr("woodwork.text_023"));
                    if ui.button(tr("woodwork.text_024")).clicked() {
                        match app.run("woodwork.bom", json!({"language":app.preferences.language})) {
                            Ok(v) => {
                                self.bom = Some(v);
                                self.message = tr("woodwork.text_025").into();
                            }
                            Err(e) => {
                                self.bom = None;
                                self.message = e;
                            }
                        }
                    }
                    if let Some(v) = &self.bom
                        && let Some(rows) = v["panels"].as_array()
                    {
                        Grid::new("woodwork-bom").striped(true).show(ui, |ui| {
                            ui.strong(tr("woodwork.text_026"));
                            ui.strong(tr("woodwork.text_027"));
                            ui.end_row();
                            for p in rows {
                                ui.label(p["name"].as_str().unwrap_or(""));
                                ui.label(format!(
                                    "{:.1} × {:.1} × {:.1}",
                                    p["length"].as_f64().unwrap_or(0.),
                                    p["width"].as_f64().unwrap_or(0.),
                                    p["thickness"].as_f64().unwrap_or(0.)
                                ));
                                ui.end_row();
                            }
                        });
                    }
                    if self.path.is_empty() {
                        self.path = app
                            .session
                            .path
                            .as_deref()
                            .map(|p| format!("{p}.stueckliste.csv"))
                            .unwrap_or_else(|| "/tmp/SolidCraft-Stueckliste.csv".into());
                    }
                    ui.label(tr("woodwork.text_028"));
                    ui.text_edit_singleline(&mut self.path);
                    if ui.button(tr("woodwork.text_029")).clicked() {
                        match app.run("woodwork.bom", json!({"language":app.preferences.language,"path":self.path})) {
                            Ok(v) => {
                                self.bom = Some(v);
                                self.message = localized_format("woodwork.format_2", &[("path", &self.path)]);
                            }
                            Err(e) => self.message = e,
                        }
                    }
                    ui.separator();
                    ui.label(localized_message(&self.message));
                    ui.small(tr("woodwork.text_030"));
                });
            });
        self.open = open;
    }
}

fn body_choice(ui: &mut egui::Ui, id: &str, value: &mut String, names: &[String]) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).selected_text(if value.is_empty() { tr("woodwork.text_031") } else { value.as_str() }).show_ui(ui, |ui| {
        for name in names {
            changed |= ui.selectable_value(value, name.clone(), name).changed();
        }
    });
    changed
}
fn face_choice(ui: &mut egui::Ui, id: &str, value: &mut usize, faces: &[Value]) {
    egui::ComboBox::from_id_salt(id).selected_text(localized_format("woodwork.format_3", &[("index", &value.to_string())])).show_ui(ui, |ui| {
        for face in faces {
            if let Some(i) = face["index"].as_u64() {
                ui.selectable_value(
                    value,
                    i as usize,
                    localized_format("woodwork.format_4", &[("index", &i.to_string()), ("normal", &face["plane_normal"].to_string())]),
                );
            }
        }
    });
}
fn read_faces(app: &mut SolveApp, name: &str) -> Vec<Value> {
    app.run("model.faces", json!({"body":name})).ok().and_then(|v| v["faces"].as_array().cloned()).unwrap_or_default()
}
impl State {
    fn advanced_ui(&mut self, app: &mut SolveApp, ui: &mut egui::Ui) {
        let names: Vec<String> = app.session.model.state().bodies.iter().map(|b| b.name.clone()).collect();
        egui::CollapsingHeader::new(tr("woodwork.text_032")).id_salt(("woodwork-advanced",self.advanced)).default_open(self.advanced).show(ui,|ui|{
   ui.strong(tr("woodwork.text_033"));ui.small(tr("woodwork.text_034"));
   if body_choice(ui,"holz-source",&mut self.source,&names){self.faces=read_faces(app,&self.source);self.opening=self.faces.iter().filter_map(|v|Some((v["index"].as_u64()? as usize,v["plane_normal"][1].as_f64()?))).min_by(|a,b|a.1.total_cmp(&b.1)).map(|p|p.0).unwrap_or(0);}
   if ui.button(tr("woodwork.text_035")).clicked(){
    let pick=app.session.selection.iter().find_map(|s|if let Sel::Face{body,index,..}=s{Some((body.clone(),*index))}else{None});
    if let Some((body,index))=pick{self.source=body;self.opening=index;self.faces=read_faces(app,&self.source);}else{self.message=tr("woodwork.text_036").into();}
   }
   ui.label(tr("Material"));ui.text_edit_singleline(&mut self.config.material);
   ui.label(tr("woodwork.text_038"));face_choice(ui,"holz-opening",&mut self.opening,&self.faces);ui.horizontal(|ui|{ui.label(tr("woodwork.text_009"));ui.add(DragValue::new(&mut self.config.thickness).range(1. ..=100.).update_while_editing(false).suffix(" mm"));});
   if ui.add_enabled(!self.source.is_empty(),egui::Button::new(tr("woodwork.text_039"))).clicked(){match app.run("woodwork.panelize",json!({"body":self.source,"thickness":self.config.thickness,"opening":self.opening,"material":self.config.material,"density":self.config.density})){
    Ok(v)=>{self.reference=Some(self.source.clone());self.message=localized_format("woodwork.format_5", &[("panels",&v["panels"].to_string())]);self.bom=app.run("woodwork.bom",json!({"language":app.preferences.language,"instance":v["instance"]})).ok();app.fit_view();},Err(e)=>self.message=e,
   }}
   if !self.source.is_empty(){ui.horizontal(|ui|{
    if ui.button(tr("woodwork.text_040")).clicked(){let _=app.run("browser.visibility",json!({"items":[self.source],"visible":false}));}
    if ui.button(tr("woodwork.text_041")).clicked(){let _=app.run("browser.visibility",json!({"items":[self.source],"visible":true}));}
   });}
   ui.separator();ui.strong(tr("woodwork.text_042"));ui.small(tr("woodwork.text_043"));
   if ui.button(tr("woodwork.text_044")).clicked(){
    let picks:Vec<_>=app.session.selection.iter().filter_map(|s|if let Sel::Face{body,index,..}=s{Some((body.clone(),*index))}else{None}).collect();
    if let (Some(a),Some(b))=(picks.first(),picks.get(1)){self.a=a.0.clone();self.fa=a.1;self.b=b.0.clone();self.fb=b.1;self.faces_a=read_faces(app,&self.a);self.faces_b=read_faces(app,&self.b);}else{self.message=tr("woodwork.text_045").into();}
   }
   if body_choice(ui,"holz-miter-a",&mut self.a,&names){self.faces_a=read_faces(app,&self.a);self.fa=self.faces_a.first().and_then(|v|v["index"].as_u64()).unwrap_or(0)as usize;}face_choice(ui,"holz-face-a",&mut self.fa,&self.faces_a);
   if body_choice(ui,"holz-miter-b",&mut self.b,&names){self.faces_b=read_faces(app,&self.b);self.fb=self.faces_b.first().and_then(|v|v["index"].as_u64()).unwrap_or(0)as usize;}face_choice(ui,"holz-face-b",&mut self.fb,&self.faces_b);
   if ui.add_enabled(!self.a.is_empty()&&!self.b.is_empty()&&self.a!=self.b,egui::Button::new(tr("woodwork.text_046"))).clicked(){match app.run("woodwork.miter",json!({"a":self.a,"b":self.b,"face_a":self.fa,"face_b":self.fb})){Ok(_)=>self.message=tr("woodwork.text_047").into(),Err(e)=>self.message=e}}
   ui.separator();ui.strong(tr("woodwork.text_048"));ui.label(tr("woodwork.text_049"));body_choice(ui,"holz-trim-body",&mut self.cut_body,&names);ui.checkbox(&mut self.cut_with_tool,tr("woodwork.text_050"));
   if ui.button(tr("woodwork.text_051")).clicked(){
    let pick=app.session.selection.iter().find_map(|s|if let Sel::Face{body,index,point}=s{Some((body.clone(),*index,*point))}else{None});
    if let Some((body,index,point))=pick{let faces=read_faces(app,&body);if let Some(f)=faces.iter().find(|v|v["index"].as_u64()==Some(index as u64)) && let Ok(n)=serde_json::from_value::<[f64;3]>(f["plane_normal"].clone()){self.cut_origin=[point.x,point.y,point.z];self.cut_normal=n;self.cut_with_tool=false;}}
   }
   if self.cut_with_tool{body_choice(ui,"holz-trim-tool",&mut self.cut_tool,&names);}else{
    if self.cut_normal==[0.;3]{self.cut_normal=[0.,0.,1.];}
    ui.horizontal(|ui|{ui.label(tr("woodwork.text_052"));for v in &mut self.cut_origin{ui.add(DragValue::new(v).speed(1.).suffix(" mm"));}});
    ui.horizontal(|ui|{ui.label(tr("woodwork.text_053"));for v in &mut self.cut_normal{ui.add(DragValue::new(v).speed(0.1));}});ui.checkbox(&mut self.cut_negative,tr("woodwork.text_054"));
   }
   if ui.add_enabled(!self.cut_body.is_empty(),egui::Button::new(tr("woodwork.text_055"))).clicked(){let p=if self.cut_with_tool{json!({"body":self.cut_body,"tool":self.cut_tool})}else{json!({"body":self.cut_body,"plane":{"origin":self.cut_origin,"normal":self.cut_normal},"keep_positive":!self.cut_negative})};match app.run("woodwork.trim",p){Ok(_)=>self.message=tr("woodwork.text_056").into(),Err(e)=>self.message=e}}
   ui.label(localized_message(&self.message));
  });
    }
}

fn localized_message(text: &str) -> String {
    match text {
        "Möbelbau · SolidCraft Holz" => tr("woodwork.text_000").to_owned(),
        "Rechteckigen Korpus über Maße erstellen" => tr("woodwork.text_001").to_owned(),
        "Maße vom ausgewählten Quader übernehmen" => tr("woodwork.text_002").to_owned(),
        "Grundkörpermaße übernommen. Der Korpus erhält eigene Parameter." => tr("woodwork.text_003").to_owned(),
        "Zuerst einen achsenparallelen Quader im Modellbaum auswählen." => tr("woodwork.text_004").to_owned(),
        "Referenz ausblenden" => tr("woodwork.text_005").to_owned(),
        "Breite" => tr("Width").to_owned(),
        "Höhe" => tr("Height").to_owned(),
        "Korpustiefe ohne Front" => tr("woodwork.text_008").to_owned(),
        "Plattenstärke" => tr("woodwork.text_009").to_owned(),
        "Böden je Fach" => tr("woodwork.text_010").to_owned(),
        "Senkrechte Trennwände" => tr("woodwork.text_011").to_owned(),
        "Rückwand" => tr("woodwork.text_012").to_owned(),
        "Rückwandstärke" => tr("woodwork.text_013").to_owned(),
        "Zwei aufliegende Türen" => tr("woodwork.text_014").to_owned(),
        "Fugenbreite" => tr("woodwork.text_015").to_owned(),
        "Plattenmaterial (einschließlich Rückwand)" => tr("woodwork.text_016").to_owned(),
        "Dichte" => tr("ui.material_review.density").to_owned(),
        "Kantenband" => tr("woodwork.text_018").to_owned(),
        "Übermaß je Kante" => tr("woodwork.text_019").to_owned(),
        "Maserung entlang Fertiglänge; vordere Sichtkanten bekantet, Türen umlaufend." => tr("woodwork.text_020").to_owned(),
        "Korpus im Dokument erstellen" => tr("woodwork.text_021").to_owned(),
        "Stückliste aus dem Modell" => tr("woodwork.text_022").to_owned(),
        "Liest die aktuellen Körpermaße des zuletzt erstellten Korpus. Material und Kantenangaben stammen aus seiner Erstellung." => {
            tr("woodwork.text_023").to_owned()
        }
        "Stückliste aktualisieren" => tr("woodwork.text_024").to_owned(),
        "Aktuelle Körpermaße eingelesen." => tr("woodwork.text_025").to_owned(),
        "Bauteil" => tr("woodwork.text_026").to_owned(),
        "L × B × S (mm)" => tr("woodwork.text_027").to_owned(),
        "CSV-Datei" => tr("woodwork.text_028").to_owned(),
        "Aktuelle Stückliste als CSV speichern" => tr("woodwork.text_029").to_owned(),
        "Freie Grundkörper: konvex mit ebenen Flächen. Gehrungen: äußere Breitflächen wählen. Konturteile vor Fertigung prüfen; Rohmaße sind Hüllmaße." => {
            tr("woodwork.text_030").to_owned()
        }
        "Körper wählen" => tr("woodwork.text_031").to_owned(),
        "Schräge Grundkörper · Gehrungen · Trimmungen" => tr("woodwork.text_032").to_owned(),
        "Platten aus einem Grundkörper" => tr("woodwork.text_033").to_owned(),
        "Konvexe Körper mit 4–12 ebenen Flächen. Eine Außenfläche bleibt offen; die übrigen werden mit Gehrungen aufgeteilt." => {
            tr("woodwork.text_034").to_owned()
        }
        "Ausgewählte Modellfläche als Öffnung übernehmen" => tr("woodwork.text_035").to_owned(),
        "Eine ebene Fläche im Modell auswählen." => tr("woodwork.text_036").to_owned(),
        "Material" => tr("Material").to_owned(),
        "Öffnungsfläche (Nummer aus Flächenliste)" => tr("woodwork.text_038").to_owned(),
        "Grundkörper in Gehrungsplatten aufteilen" => tr("woodwork.text_039").to_owned(),
        "Grundkörper ausblenden" => tr("woodwork.text_040").to_owned(),
        "Grundkörper einblenden" => tr("woodwork.text_041").to_owned(),
        "Gehrung zwischen zwei vorhandenen Platten" => tr("woodwork.text_042").to_owned(),
        "Zwei überlappende Platten wählen, dann je eine äußere Breitfläche. Der Winkel wird aus diesen Ebenen berechnet." => {
            tr("woodwork.text_043").to_owned()
        }
        "Zwei ausgewählte Modellflächen für Gehrung übernehmen" => tr("woodwork.text_044").to_owned(),
        "Zwei Außenflächen mit Mehrfachauswahl wählen." => tr("woodwork.text_045").to_owned(),
        "Gehrung schneiden" => tr("woodwork.text_046").to_owned(),
        "Gehrung geschnitten. Ein Rückgängig-Schritt stellt beide Platten wieder her." => tr("woodwork.text_047").to_owned(),
        "Freie Trimmung" => tr("woodwork.text_048").to_owned(),
        "Zu trimmende Platte" => tr("woodwork.text_049").to_owned(),
        "Grenzkörper abziehen (Grenzkörper bleibt erhalten)" => tr("woodwork.text_050").to_owned(),
        "Ausgewählte Modellfläche als Schnittebene übernehmen" => tr("woodwork.text_051").to_owned(),
        "Punkt auf Schnittebene" => tr("woodwork.text_052").to_owned(),
        "Ebenennormale" => tr("woodwork.text_053").to_owned(),
        "Negative Seite der Ebene behalten" => tr("woodwork.text_054").to_owned(),
        "Platte trimmen" => tr("woodwork.text_055").to_owned(),
        "Platte getrimmt. Mit Rückgängig wiederherstellbar." => tr("woodwork.text_056").to_owned(),
        "Einen Quader auswählen." => tr("woodwork.text_057").to_owned(),
        "Grundkörper nicht gefunden." => tr("woodwork.text_058").to_owned(),
        "Diese Maßübernahme unterstützt achsenparallele Quader. Für schräge Körper „Grundkörper in Platten“ verwenden." => {
            tr("woodwork.text_059").to_owned()
        }
        "Höchstens 100 Möbelkorpusse pro Dokument." => tr("woodwork.text_060").to_owned(),
        "Kein freier Korpusname verfügbar." => tr("woodwork.text_061").to_owned(),
        "Ungültiger Ursprung." => tr("woodwork.text_062").to_owned(),
        "Ungültige Möbeldefinition." => tr("woodwork.text_063").to_owned(),
        "Noch kein Möbelkorpus in diesem Dokument." => tr("woodwork.text_064").to_owned(),
        "Kein Möbelkorpus gefunden." => tr("woodwork.text_065").to_owned(),
        "Korpus nicht gefunden." => tr("woodwork.text_066").to_owned(),
        "Fehlende Plattenzuordnung." => tr("woodwork.text_067").to_owned(),
        "Drei Koordinaten erforderlich." => tr("woodwork.text_068").to_owned(),
        "Ungültige Koordinaten." => tr("woodwork.text_069").to_owned(),
        "Fläche nicht gefunden." => tr("woodwork.text_070").to_owned(),
        "Eine ebene Außenfläche auswählen." => tr("woodwork.text_071").to_owned(),
        "Ebenennormale darf nicht null sein." => tr("woodwork.text_072").to_owned(),
        "Die Trimmung würde die gesamte Platte entfernen." => tr("woodwork.text_073").to_owned(),
        "Die Ebene muss genau ein zusammenhängendes Restteil ergeben." => tr("woodwork.text_074").to_owned(),
        "Platte und Grenzkörper müssen verschieden sein." => tr("woodwork.text_075").to_owned(),
        "Zwei verschiedene Platten auswählen." => tr("woodwork.text_076").to_owned(),
        "Außenfläche A fehlt." => tr("woodwork.text_077").to_owned(),
        "Außenfläche B fehlt." => tr("woodwork.text_078").to_owned(),
        "Flächenindex zu groß." => tr("woodwork.text_079").to_owned(),
        "Parallele Außenflächen definieren keine Gehrung." => tr("woodwork.text_080").to_owned(),
        "Die gewählten Außenflächen ergeben keine beidseitige Gehrung. Überlappende Platten und die äußeren Breitflächen wählen." => {
            tr("woodwork.text_081").to_owned()
        }
        "Plattenstärke: 1 bis 100 mm." => tr("woodwork.text_082").to_owned(),
        "Grundkörper: 4 bis 12 ebene Flächen erforderlich." => tr("woodwork.text_083").to_owned(),
        "Nur konvexe Grundkörper ohne Aussparungen werden unterstützt." => tr("woodwork.text_084").to_owned(),
        "Plattenstärke zu groß für diesen Grundkörper." => tr("woodwork.text_085").to_owned(),
        "Ungültige Öffnungsfläche." => tr("woodwork.text_086").to_owned(),
        "Öffnungsfläche nicht gefunden." => tr("woodwork.text_087").to_owned(),
        "Höchstens 100 Korpusse." => tr("woodwork.text_088").to_owned(),
        "Keine freie Korpusnummer." => tr("woodwork.text_089").to_owned(),
        "Grundkörper konnte nicht kopiert werden." => tr("woodwork.text_090").to_owned(),
        "Ungültige Flächenebene." => tr("woodwork.text_091").to_owned(),
        "Ungültige Plattenzuordnung." => tr("woodwork.text_092").to_owned(),
        "Höchstens 20 Böden und 4 Trennwände sind zulässig." => tr("woodwork.text_093").to_owned(),
        "Die Platten lassen keinen ausreichenden Innenraum frei. Maße oder Bodenanzahl ändern." => tr("woodwork.text_094").to_owned(),
        "Projektname und Material müssen 1 bis 100 Zeichen enthalten." => tr("woodwork.text_095").to_owned(),
        "Tiefe" => crate::i18n::tr_label("Depth").to_owned(),
        "Fuge" => crate::i18n::tr_label("Gap").to_owned(),
        "Übermaß" => tr("woodwork.text_019").to_owned(),
        _ => {
            for (prefix, id) in [("Körper fehlt: ", "woodwork.error_body"), ("Platte fehlt oder wurde umbenannt: ", "woodwork.error_panel")] {
                if let Some(name) = text.strip_prefix(prefix) {
                    return localized_format(id, &[("name", name)]);
                }
            }
            if let Some(key) = text.strip_suffix(": Körpername fehlt.") {
                return localized_format("woodwork.error_body_name", &[("key", key)]);
            }
            if let Some((name, rest)) = text.split_once(": zulässig sind ")
                && let Some((lo, hi)) = rest.strip_suffix('.').and_then(|r| r.split_once(" bis "))
            {
                return localized_format("woodwork.error_range", &[("name", &localized_message(name)), ("lo", lo), ("hi", hi)]);
            }
            text.to_owned()
        }
    }
}

#[cfg(test)]
mod english_tests {
    use super::*;
    #[test]
    fn furniture_labels_and_messages_follow_language() {
        let _english = crate::i18n::scope("en");
        assert_eq!(tr("woodwork.text_009"), "Panel thickness");
        assert_eq!(localized_message("Körper fehlt: Door {left}"), "Body not found: Door {left}");
        assert_eq!(localized_message("Plattenstärke: zulässig sind 3 bis 100."), "Panel thickness: allowed range is 3 to 100.");
        {
            let _german = crate::i18n::scope("de");
            assert_eq!(tr("woodwork.text_009"), "Plattenstärke");
            assert_eq!(localized_message("Körper fehlt: Door {left}"), "Körper fehlt: Door {left}");
        }
        assert_eq!(tr("woodwork.text_009"), "Panel thickness");
    }
}
