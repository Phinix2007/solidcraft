//! Furniture tools integrated into the native SolidCraft window.
use crate::SolveApp;
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
        Window::new("Möbelbau · SolidCraft Holz").open(&mut open).default_width(560.).default_height(780.).default_pos(egui::pos2(1000.,140.)).resizable(true).show(ctx,|ui|{
   egui::ScrollArea::vertical().max_height(680.).show(ui,|ui|{
    self.advanced_ui(app,ui);
    ui.separator();ui.label("Rechteckigen Korpus über Maße erstellen");
    if ui.button("Maße vom ausgewählten Quader übernehmen").clicked(){
     let name=app.session.selection.iter().find_map(|s|if let Sel::Body{name}=s{Some(name.clone())}else{None});
     if let Some(name)=name {match app.run("woodwork.bounds",json!({"body":name})){
      Ok(v)=>{self.config.width=v["width"].as_f64().unwrap_or(self.config.width);self.config.height=v["height"].as_f64().unwrap_or(self.config.height);self.config.depth=v["depth"].as_f64().unwrap_or(self.config.depth);if let Ok(o)=serde_json::from_value(v["origin"].clone()){self.origin=o;}self.reference=Some(name);self.message="Grundkörpermaße übernommen. Der Korpus erhält eigene Parameter.".into();},Err(e)=>self.message=e,
     }}else{self.message="Zuerst einen achsenparallelen Quader im Modellbaum auswählen.".into();}
    }
    if let Some(name)=&self.reference {ui.horizontal_wrapped(|ui|{ui.label(format!("Referenz: {name}"));if ui.button("Referenz ausblenden").clicked(){let _=app.run("browser.visibility",json!({"items":[name],"visible":false}));}});}
    ui.separator();Grid::new("woodwork-size").num_columns(2).spacing([18.,7.]).show(ui,|ui|{
     mm(ui,"Breite",&mut self.config.width,100. ..=5000.);mm(ui,"Höhe",&mut self.config.height,100. ..=5000.);mm(ui,"Korpustiefe ohne Front",&mut self.config.depth,100. ..=2000.);mm(ui,"Plattenstärke",&mut self.config.thickness,3. ..=100.);
     ui.label("Böden je Fach");ui.add(DragValue::new(&mut self.config.shelves).range(0..=20));ui.end_row();ui.label("Senkrechte Trennwände");ui.add(DragValue::new(&mut self.config.dividers).range(0..=4));ui.end_row();
    });
    ui.checkbox(&mut self.config.back,"Rückwand");if self.config.back{Grid::new("woodwork-back").show(ui,|ui|mm(ui,"Rückwandstärke",&mut self.config.back_thickness,2. ..=50.));}
    ui.checkbox(&mut self.config.doors,"Zwei aufliegende Türen");if self.config.doors{Grid::new("woodwork-doors").show(ui,|ui|mm(ui,"Fugenbreite",&mut self.config.gap,0.5..=10.));}
    ui.separator();ui.label("Plattenmaterial (einschließlich Rückwand)");ui.text_edit_singleline(&mut self.config.material);
    Grid::new("woodwork-material").show(ui,|ui|{ui.label("Dichte");ui.add(DragValue::new(&mut self.config.density).range(0.05..=2.).speed(0.01).suffix(" g/cm³"));ui.end_row();mm(ui,"Kantenband",&mut self.config.band,0. ..=5.);mm(ui,"Übermaß je Kante",&mut self.config.allowance,0. ..=30.);});
    ui.small("Maserung entlang Fertiglänge; vordere Sichtkanten bekantet, Türen umlaufend.");
    let valid=self.config.validate();if let Err(e)=&valid{ui.colored_label(egui::Color32::LIGHT_RED,e);}
    if ui.add_enabled(valid.is_ok()&&app.session.active_sketch.is_none(),egui::Button::new("Korpus im Dokument erstellen")).clicked(){match app.run("woodwork.cabinet",json!({"config":self.config,"origin":self.origin})){
     Ok(v)=>{self.message=format!("Korpus {} mit {} Platten erstellt. Rückgängig entfernt ihn in einem Schritt.",v["instance"],v["panels"]);app.fit_view();self.bom=app.run("woodwork.bom",json!({"instance":v["instance"]})).ok();},Err(e)=>self.message=e,
    }}
    ui.separator();ui.strong("Stückliste aus dem Modell");ui.small("Liest die aktuellen Körpermaße des zuletzt erstellten Korpus. Material und Kantenangaben stammen aus seiner Erstellung.");
    if ui.button("Stückliste aktualisieren").clicked(){match app.run("woodwork.bom",json!({})){Ok(v)=>{self.bom=Some(v);self.message="Aktuelle Körpermaße eingelesen.".into();},Err(e)=>{self.bom=None;self.message=e;}}}
    if let Some(v)=&self.bom && let Some(rows)=v["panels"].as_array(){Grid::new("woodwork-bom").striped(true).show(ui,|ui|{ui.strong("Bauteil");ui.strong("L × B × S (mm)");ui.end_row();for p in rows {ui.label(p["name"].as_str().unwrap_or(""));ui.label(format!("{:.1} × {:.1} × {:.1}",p["length"].as_f64().unwrap_or(0.),p["width"].as_f64().unwrap_or(0.),p["thickness"].as_f64().unwrap_or(0.)));ui.end_row();}});}
    if self.path.is_empty(){self.path=app.session.path.as_deref().map(|p|format!("{p}.stueckliste.csv")).unwrap_or_else(||"/tmp/SolidCraft-Stueckliste.csv".into());}
    ui.label("CSV-Datei");ui.text_edit_singleline(&mut self.path);
    if ui.button("Aktuelle Stückliste als CSV speichern").clicked(){match app.run("woodwork.bom",json!({"path":self.path})){Ok(v)=>{self.bom=Some(v);self.message=format!("Gespeichert: {}",self.path);},Err(e)=>self.message=e}}
    ui.separator();ui.label(&self.message);ui.small("Freie Grundkörper: konvex mit ebenen Flächen. Gehrungen: äußere Breitflächen wählen. Konturteile vor Fertigung prüfen; Rohmaße sind Hüllmaße.");
   });
  });
        self.open = open;
    }
}

fn body_choice(ui: &mut egui::Ui, id: &str, value: &mut String, names: &[String]) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).selected_text(if value.is_empty() { "Körper wählen" } else { value.as_str() }).show_ui(ui, |ui| {
        for name in names {
            changed |= ui.selectable_value(value, name.clone(), name).changed();
        }
    });
    changed
}
fn face_choice(ui: &mut egui::Ui, id: &str, value: &mut usize, faces: &[Value]) {
    egui::ComboBox::from_id_salt(id).selected_text(format!("Fläche {}", *value)).show_ui(ui, |ui| {
        for face in faces {
            if let Some(i) = face["index"].as_u64() {
                ui.selectable_value(value, i as usize, format!("Fläche {i} · Normale {}", face["plane_normal"]));
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
        egui::CollapsingHeader::new("Schräge Grundkörper · Gehrungen · Trimmungen").id_salt(("woodwork-advanced",self.advanced)).default_open(self.advanced).show(ui,|ui|{
   ui.strong("Platten aus einem Grundkörper");ui.small("Konvexe Körper mit 4–12 ebenen Flächen. Eine Außenfläche bleibt offen; die übrigen werden mit Gehrungen aufgeteilt.");
   if body_choice(ui,"holz-source",&mut self.source,&names){self.faces=read_faces(app,&self.source);self.opening=self.faces.iter().filter_map(|v|Some((v["index"].as_u64()? as usize,v["plane_normal"][1].as_f64()?))).min_by(|a,b|a.1.total_cmp(&b.1)).map(|p|p.0).unwrap_or(0);}
   if ui.button("Ausgewählte Modellfläche als Öffnung übernehmen").clicked(){
    let pick=app.session.selection.iter().find_map(|s|if let Sel::Face{body,index,..}=s{Some((body.clone(),*index))}else{None});
    if let Some((body,index))=pick{self.source=body;self.opening=index;self.faces=read_faces(app,&self.source);}else{self.message="Eine ebene Fläche im Modell auswählen.".into();}
   }
   ui.label("Material");ui.text_edit_singleline(&mut self.config.material);
   ui.label("Öffnungsfläche (Nummer aus Flächenliste)");face_choice(ui,"holz-opening",&mut self.opening,&self.faces);ui.horizontal(|ui|{ui.label("Plattenstärke");ui.add(DragValue::new(&mut self.config.thickness).range(1. ..=100.).update_while_editing(false).suffix(" mm"));});
   if ui.add_enabled(!self.source.is_empty(),egui::Button::new("Grundkörper in Gehrungsplatten aufteilen")).clicked(){match app.run("woodwork.panelize",json!({"body":self.source,"thickness":self.config.thickness,"opening":self.opening,"material":self.config.material,"density":self.config.density})){
    Ok(v)=>{self.reference=Some(self.source.clone());self.message=format!("{} Gehrungsplatten erstellt. Referenzkörper bei Bedarf ausblenden.",v["panels"]);self.bom=app.run("woodwork.bom",json!({"instance":v["instance"]})).ok();app.fit_view();},Err(e)=>self.message=e,
   }}
   if !self.source.is_empty(){ui.horizontal(|ui|{
    if ui.button("Grundkörper ausblenden").clicked(){let _=app.run("browser.visibility",json!({"items":[self.source],"visible":false}));}
    if ui.button("Grundkörper einblenden").clicked(){let _=app.run("browser.visibility",json!({"items":[self.source],"visible":true}));}
   });}
   ui.separator();ui.strong("Gehrung zwischen zwei vorhandenen Platten");ui.small("Zwei überlappende Platten wählen, dann je eine äußere Breitfläche. Der Winkel wird aus diesen Ebenen berechnet.");
   if ui.button("Zwei ausgewählte Modellflächen für Gehrung übernehmen").clicked(){
    let picks:Vec<_>=app.session.selection.iter().filter_map(|s|if let Sel::Face{body,index,..}=s{Some((body.clone(),*index))}else{None}).collect();
    if let (Some(a),Some(b))=(picks.first(),picks.get(1)){self.a=a.0.clone();self.fa=a.1;self.b=b.0.clone();self.fb=b.1;self.faces_a=read_faces(app,&self.a);self.faces_b=read_faces(app,&self.b);}else{self.message="Zwei Außenflächen mit Mehrfachauswahl wählen.".into();}
   }
   if body_choice(ui,"holz-miter-a",&mut self.a,&names){self.faces_a=read_faces(app,&self.a);self.fa=self.faces_a.first().and_then(|v|v["index"].as_u64()).unwrap_or(0)as usize;}face_choice(ui,"holz-face-a",&mut self.fa,&self.faces_a);
   if body_choice(ui,"holz-miter-b",&mut self.b,&names){self.faces_b=read_faces(app,&self.b);self.fb=self.faces_b.first().and_then(|v|v["index"].as_u64()).unwrap_or(0)as usize;}face_choice(ui,"holz-face-b",&mut self.fb,&self.faces_b);
   if ui.add_enabled(!self.a.is_empty()&&!self.b.is_empty()&&self.a!=self.b,egui::Button::new("Gehrung schneiden")).clicked(){match app.run("woodwork.miter",json!({"a":self.a,"b":self.b,"face_a":self.fa,"face_b":self.fb})){Ok(_)=>self.message="Gehrung geschnitten. Ein Rückgängig-Schritt stellt beide Platten wieder her.".into(),Err(e)=>self.message=e}}
   ui.separator();ui.strong("Freie Trimmung");ui.label("Zu trimmende Platte");body_choice(ui,"holz-trim-body",&mut self.cut_body,&names);ui.checkbox(&mut self.cut_with_tool,"Grenzkörper abziehen (Grenzkörper bleibt erhalten)");
   if ui.button("Ausgewählte Modellfläche als Schnittebene übernehmen").clicked(){
    let pick=app.session.selection.iter().find_map(|s|if let Sel::Face{body,index,point}=s{Some((body.clone(),*index,*point))}else{None});
    if let Some((body,index,point))=pick{let faces=read_faces(app,&body);if let Some(f)=faces.iter().find(|v|v["index"].as_u64()==Some(index as u64)) && let Ok(n)=serde_json::from_value::<[f64;3]>(f["plane_normal"].clone()){self.cut_origin=[point.x,point.y,point.z];self.cut_normal=n;self.cut_with_tool=false;}}
   }
   if self.cut_with_tool{body_choice(ui,"holz-trim-tool",&mut self.cut_tool,&names);}else{
    if self.cut_normal==[0.;3]{self.cut_normal=[0.,0.,1.];}
    ui.horizontal(|ui|{ui.label("Punkt auf Schnittebene");for v in &mut self.cut_origin{ui.add(DragValue::new(v).speed(1.).suffix(" mm"));}});
    ui.horizontal(|ui|{ui.label("Ebenennormale");for v in &mut self.cut_normal{ui.add(DragValue::new(v).speed(0.1));}});ui.checkbox(&mut self.cut_negative,"Negative Seite der Ebene behalten");
   }
   if ui.add_enabled(!self.cut_body.is_empty(),egui::Button::new("Platte trimmen")).clicked(){let p=if self.cut_with_tool{json!({"body":self.cut_body,"tool":self.cut_tool})}else{json!({"body":self.cut_body,"plane":{"origin":self.cut_origin,"normal":self.cut_normal},"keep_positive":!self.cut_negative})};match app.run("woodwork.trim",p){Ok(_)=>self.message="Platte getrimmt. Mit Rückgängig wiederherstellbar.".into(),Err(e)=>self.message=e}}
   ui.label(&self.message);
  });
    }
}
