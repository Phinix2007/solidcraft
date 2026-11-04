//! Original furniture geometry and manufacturing lists.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Cabinet {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub thickness: f64,
    pub shelves: usize,
    pub dividers: usize,
    pub back: bool,
    pub back_thickness: f64,
    pub doors: bool,
    pub gap: f64,
    pub material: String,
    pub density: f64,
    pub band: f64,
    pub allowance: f64,
}
impl Default for Cabinet {
    fn default() -> Self {
        Self {
            name: "Regalkorpus".into(),
            width: 800.,
            height: 1200.,
            depth: 400.,
            thickness: 18.,
            shelves: 3,
            dividers: 0,
            back: true,
            back_thickness: 6.,
            doors: false,
            gap: 2.,
            material: "Birke Multiplex".into(),
            density: 0.65,
            band: 1.,
            allowance: 2.,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Panel {
    pub name: String,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    pub origin: [f64; 3],
    pub size: [f64; 3],
    pub size_expr: [String; 3],
    pub origin_expr: [String; 3],
    pub edges_length: u32,
    pub edges_width: u32,
    pub back: bool,
    pub actual_volume_mm3: Option<f64>,
    pub shaped: bool,
}
impl Panel {
    pub fn blank(&self, c: &Cabinet) -> [f64; 2] {
        if self.back {
            return [self.length + 2. * c.allowance, self.width + 2. * c.allowance];
        }
        [self.length - f64::from(self.edges_width) * c.band + 2. * c.allowance, self.width - f64::from(self.edges_length) * c.band + 2. * c.allowance]
    }
    pub fn volume(&self) -> f64 {
        self.actual_volume_mm3.unwrap_or(self.length * self.width * self.thickness) / 1e9
    }
    pub fn edge_m(&self) -> f64 {
        (self.length * f64::from(self.edges_length) + self.width * f64::from(self.edges_width)) / 1000.
    }
}
impl Cabinet {
    pub fn validate(&self) -> Result<(), String> {
        for (name, v, lo, hi) in [
            ("Breite", self.width, 100., 5000.),
            ("Höhe", self.height, 100., 5000.),
            ("Tiefe", self.depth, 100., 2000.),
            ("Plattenstärke", self.thickness, 3., 100.),
            ("Rückwandstärke", self.back_thickness, 2., 50.),
            ("Fuge", self.gap, 0.5, 10.),
            ("Dichte", self.density, 0.05, 2.),
            ("Kantenband", self.band, 0., 5.),
            ("Übermaß", self.allowance, 0., 30.),
        ] {
            if !v.is_finite() || v < lo || v > hi {
                return Err(format!("{name}: zulässig sind {lo} bis {hi}."));
            }
        }
        if self.shelves > 20 || self.dividers > 4 {
            return Err("Höchstens 20 Böden und 4 Trennwände sind zulässig.".into());
        }
        if self.width <= (self.dividers + 2) as f64 * self.thickness + 30.
            || self.depth <= self.back_thickness + 30.
            || self.height <= (self.shelves + 2) as f64 * self.thickness + 30.
        {
            return Err("Die Platten lassen keinen ausreichenden Innenraum frei. Maße oder Bodenanzahl ändern.".into());
        }
        if self.material.trim().is_empty() || self.material.len() > 100 || self.name.trim().is_empty() || self.name.len() > 100 {
            return Err("Projektname und Material müssen 1 bis 100 Zeichen enthalten.".into());
        }
        Ok(())
    }
    pub fn panels(&self) -> Result<Vec<Panel>, String> {
        self.validate()?;
        let (w, h, d, t) = (self.width, self.height, self.depth, self.thickness);
        let b = if self.back { self.back_thickness } else { 0. };
        let de = if self.back { "Tiefe - Rueckwand" } else { "Tiefe" };
        let mut out = Vec::new();
        let mut add = |name: &str,
                       l: f64,
                       pw: f64,
                       thick: f64,
                       origin: [f64; 3],
                       size: [f64; 3],
                       se: [&str; 3],
                       oe: [&str; 3],
                       el: u32,
                       ew: u32,
                       back: bool| {
            out.push(Panel {
                name: name.into(),
                length: l,
                width: pw,
                thickness: thick,
                origin,
                size,
                size_expr: se.map(str::to_string),
                origin_expr: oe.map(str::to_string),
                edges_length: el,
                edges_width: ew,
                back,
                actual_volume_mm3: None,
                shaped: false,
            });
        };
        add("01 Seite links", h, d - b, t, [0., 0., 0.], [t, d - b, h], ["Staerke", de, "Hoehe"], ["0 mm", "0 mm", "0 mm"], 1, 0, false);
        add(
            "02 Seite rechts",
            h,
            d - b,
            t,
            [w - t, 0., 0.],
            [t, d - b, h],
            ["Staerke", de, "Hoehe"],
            ["Breite - Staerke", "0 mm", "0 mm"],
            1,
            0,
            false,
        );
        add(
            "03 Boden",
            w - 2. * t,
            d - b,
            t,
            [t, 0., 0.],
            [w - 2. * t, d - b, t],
            ["Breite - 2 * Staerke", de, "Staerke"],
            ["Staerke", "0 mm", "0 mm"],
            1,
            0,
            false,
        );
        add(
            "04 Deckel",
            w - 2. * t,
            d - b,
            t,
            [t, 0., h - t],
            [w - 2. * t, d - b, t],
            ["Breite - 2 * Staerke", de, "Staerke"],
            ["Staerke", "0 mm", "Hoehe - Staerke"],
            1,
            0,
            false,
        );
        let bay_width = (w - (self.dividers + 2) as f64 * t) / (self.dividers + 1) as f64;
        let bwe = format!("(Breite - {} * Staerke) / {}", self.dividers + 2, self.dividers + 1);
        for j in 1..=self.dividers {
            let x = t + j as f64 * bay_width + (j - 1) as f64 * t;
            let xe = format!("Staerke + {j} * ({bwe}) + {} * Staerke", j - 1);
            add(
                &format!("Trennwand {j}"),
                h - 2. * t,
                d - b,
                t,
                [x, 0., t],
                [t, d - b, h - 2. * t],
                ["Staerke", de, "Hoehe - 2 * Staerke"],
                [&xe, "0 mm", "Staerke"],
                1,
                0,
                false,
            );
        }
        for i in 1..=self.shelves {
            let z = t + i as f64 * (h - (self.shelves + 2) as f64 * t) / (self.shelves + 1) as f64 + (i - 1) as f64 * t;
            let ze = format!("Staerke + {i} * (Hoehe - {} * Staerke) / {} + {} * Staerke", self.shelves + 2, self.shelves + 1, i - 1);
            for bay in 0..=self.dividers {
                let x = t + bay as f64 * (bay_width + t);
                let xe = format!("Staerke + {bay} * (({bwe}) + Staerke)");
                add(
                    &format!("Einlegeboden {i}.{}", bay + 1),
                    bay_width,
                    d - b,
                    t,
                    [x, 0., z],
                    [bay_width, d - b, t],
                    [&bwe, de, "Staerke"],
                    [&xe, "0 mm", &ze],
                    1,
                    0,
                    false,
                );
            }
        }
        if self.back {
            add(
                "90 Rückwand",
                h,
                w,
                b,
                [0., d - b, 0.],
                [w, b, h],
                ["Breite", "Rueckwand", "Hoehe"],
                ["0 mm", "Tiefe - Rueckwand", "0 mm"],
                0,
                0,
                true,
            );
        }
        if self.doors {
            let dw = (w - 3. * self.gap) / 2.;
            let dh = h - 2. * self.gap;
            for (name, x, xe) in [("91 Tür links", self.gap, "Fuge"), ("92 Tür rechts", 2. * self.gap + dw, "(Breite + Fuge) / 2")] {
                add(
                    name,
                    dh,
                    dw,
                    t,
                    [x, -t - self.gap, self.gap],
                    [dw, t, dh],
                    ["(Breite - 3 * Fuge) / 2", "Staerke", "Hoehe - 2 * Fuge"],
                    [xe, "-Staerke - Fuge", "Fuge"],
                    2,
                    2,
                    false,
                );
            }
        }
        Ok(out)
    }
    pub fn script(&self) -> Result<Value, String> {
        let panels = self.panels()?;
        let mut cmds = vec![json!({"command":"file.new","params":{"name":self.name}})];
        for (name, v) in [
            ("Breite", self.width),
            ("Hoehe", self.height),
            ("Tiefe", self.depth),
            ("Staerke", self.thickness),
            ("Rueckwand", self.back_thickness),
            ("Fuge", self.gap),
        ] {
            cmds.push(json!({"command":"parameters.add","params":{"name":name,"expression":format!("{v} mm"),"favorite":true}}));
        }
        cmds.push(
            json!({"command":"material.create","params":{"name":format!("Holz: {}",self.material),"density_g_cm3":self.density,"color":"#ba8e59"}}),
        );
        for p in &panels {
            cmds.push(json!({"command":"solid.box","params":{"length":p.size_expr[0],"width":p.size_expr[1],"height":p.size_expr[2],"corner":p.origin_expr,"operation":"new_body","body_name":p.name,"name":p.name}}));
        }
        cmds.push(json!({"command":"material.assign","params":{"bodies":panels.iter().map(|p|p.name.clone()).collect::<Vec<_>>(),"material":format!("Holz: {}",self.material)}}));
        Ok(json!({"commands":cmds}))
    }
    pub fn csv(&self) -> Result<String, String> {
        Ok(self.csv_for(&self.panels()?))
    }
    pub fn csv_for(&self, panels: &[Panel]) -> String {
        self.csv_for_language(panels, "de")
    }
    pub fn csv_for_language(&self, panels: &[Panel], language: &str) -> String {
        let english = language != "de";
        let mut s="\u{feff}Position;Bauteil;Anzahl;Material;Fertiglaenge_mm;Fertigbreite_mm;Staerke_mm;Rohlaenge_mm;Rohbreite_mm;Maserung;Kanten_lang;Kanten_kurz;Kantenband_mm;Volumen_m3;Masse_kg;Konturpruefung\r\n".to_string();
        if english {
            s = "\u{feff}Position;Part;Quantity;Material;FinishedLength_mm;FinishedWidth_mm;Thickness_mm;BlankLength_mm;BlankWidth_mm;Grain;LongEdges;ShortEdges;EdgeBand_mm;Volume_m3;Mass_kg;ContourCheck\r\n".to_string();
        }
        for (i, p) in panels.iter().enumerate() {
            let blank = p.blank(self);
            let mat =
                if p.back { format!("{} ({})", self.material, if english { "back panel" } else { "Rückwand" }) } else { self.material.clone() };
            let fields = vec![
                (i + 1).to_string(),
                safe_cell(&p.name),
                "1".into(),
                safe_cell(&mat),
                num(p.length),
                num(p.width),
                num(p.thickness),
                num(blank[0]),
                num(blank[1]),
                if english { "along finished length".into() } else { "entlang Fertiglänge".into() },
                p.edges_length.to_string(),
                p.edges_width.to_string(),
                num(if p.back { 0. } else { self.band }),
                format!("{:.6}", p.volume()).replace('.', ","),
                num(p.volume() * self.density * 1000.),
                if p.shaped {
                    if english { "Yes – check contour/miter".into() } else { "Ja – Kontur/Gehrung prüfen".into() }
                } else if english {
                    "No".into()
                } else {
                    "Nein".into()
                },
            ];
            let fields = if english {
                fields
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| if [4, 5, 6, 7, 8, 12, 13, 14].contains(&i) { v.replace(',', ".") } else { v })
                    .collect::<Vec<_>>()
            } else {
                fields
            };
            s.push_str(&fields.iter().map(|x| format!("\"{}\"", x.replace('"', "\"\""))).collect::<Vec<_>>().join(";"));
            s.push_str("\r\n");
        }
        s
    }
}
fn num(v: f64) -> String {
    format!("{v:.3}").replace('.', ",")
}
fn safe_cell(s: &str) -> String {
    if s.trim_start().starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_csv_preserves_user_names_and_uses_decimal_points() {
        let c = Cabinet { material: "Material, custom".into(), ..Cabinet::default() };
        let mut panels = c.panels().unwrap();
        panels[0].name = "=custom, part".into();
        let en = c.csv_for_language(&panels, "en");
        let de = c.csv_for(&panels);
        assert!(en.starts_with("\u{feff}Position;Part;Quantity;Material;FinishedLength_mm"));
        assert!(en.contains("along finished length"));
        assert!(en.contains("\"18.000\""));
        assert!(en.contains("'=custom, part"));
        assert!(en.contains("Material, custom"));
        assert!(de.contains("Fertiglaenge_mm"));
        assert!(de.contains("\"18,000\""));
    }
    #[test]
    fn dividers_and_shelves_have_disjoint_interiors() {
        let c = Cabinet { dividers: 2, shelves: 4, doors: true, ..Default::default() };
        let panels = c.panels().unwrap();
        assert_eq!(panels.len(), 21);
        for (i, a) in panels.iter().enumerate() {
            for b in panels.iter().skip(i + 1) {
                let overlap = (0..3).all(|k| a.origin[k] + a.size[k] > b.origin[k] + 1e-7 && b.origin[k] + b.size[k] > a.origin[k] + 1e-7);
                assert!(!overlap, "{} overlaps {}", a.name, b.name);
            }
        }
    }
    #[test]
    fn default_cabinet_is_consistent() {
        let c = Cabinet::default();
        let p = c.panels().unwrap();
        assert_eq!(p.len(), 8);
        assert_eq!(p[2].length, 764.);
        assert_eq!(p[0].width, 394.);
        assert_eq!(p[2].blank(&c), [768., 397.]);
    }
    #[test]
    fn shelves_do_not_overlap() {
        for n in [0, 1, 3, 20] {
            let c = Cabinet { shelves: n, ..Default::default() };
            let p = c.panels().unwrap();
            let mut last = c.thickness;
            for s in p.iter().filter(|p| p.name.contains("Einlegeboden")) {
                assert!(s.origin[2] > last);
                last = s.origin[2] + c.thickness;
            }
            assert!(last < c.height - c.thickness);
        }
    }
    #[test]
    fn doors_fit_with_three_gaps() {
        let c = Cabinet { doors: true, ..Default::default() };
        let p = c.panels().unwrap();
        let doors: Vec<_> = p.iter().filter(|p| p.name.contains("Tür")).collect();
        assert_eq!(doors.len(), 2);
        assert!((doors[0].width * 2. + 3. * c.gap - c.width).abs() < 1e-9);
        assert_eq!(doors[0].origin[1] + doors[0].size[1], -c.gap);
    }
    #[test]
    fn invalid_dimensions_fail() {
        for v in [f64::NAN, f64::INFINITY, -1., 0., 10000.] {
            assert!(Cabinet { width: v, ..Default::default() }.panels().is_err());
        }
        assert!(Cabinet { shelves: 21, ..Default::default() }.panels().is_err());
        assert!(Cabinet { height: 100., shelves: 20, ..Default::default() }.panels().is_err());
    }
    #[test]
    fn csv_has_no_formula_injection() {
        let c = Cabinet { material: "=1+2;\"test\"".into(), ..Default::default() };
        let csv = c.csv().unwrap();
        assert!(csv.contains("\"'=1+2;\"\"test\"\"\""));
        assert_eq!(csv.lines().count(), 9);
    }
    #[test]
    fn no_back_has_full_depth() {
        let c = Cabinet { back: false, ..Default::default() };
        assert_eq!(c.panels().unwrap().len(), 7);
        assert_eq!(c.panels().unwrap()[0].width, 400.);
    }
}
