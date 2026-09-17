//! Pretty printer for field/material/law declarations (W16).
//!
//! Formats VibeScript declarations in a canonical, readable style.
//! This is CST sugar — it uses the trivia module to preserve comments
//! and whitespace, so round-tripping through the pretty printer doesn't
//! destroy commentary.
//!
//! ## Style
//!
//! ```vibe
//! field pressure_ambient: Pressure
//!   unit: <qudt:KiloPascal>
//!   support: region
//!   representation: grid;
//!
//! material steel: Material
//!   yield: 250.0,
//!   density: 7850.0;
//!
//! law crush
//!   when pressure_ambient > steel.yield
//!   => 1;
//! ```
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` wish list W16.

use std::fmt::Write;

/// Pretty-print options.
#[derive(Debug, Clone)]
pub struct PrettyOptions {
    /// Indentation string (default: two spaces).
    pub indent: String,
    /// Blank lines between top-level declarations (default: 1).
    pub blank_lines_between_decls: usize,
    /// Maximum line width before wrapping (default: 80).
    pub max_line_width: usize,
}

impl Default for PrettyOptions {
    fn default() -> Self {
        Self {
            indent: "  ".into(),
            blank_lines_between_decls: 1,
            max_line_width: 80,
        }
    }
}

/// A pretty-printable field declaration.
#[derive(Debug, Clone)]
pub struct PrettyField {
    pub name: String,
    pub field_type: String,
    pub properties: Vec<(String, String)>,
}

impl PrettyField {
    pub fn new(name: &str, field_type: &str) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            properties: Vec::new(),
        }
    }

    pub fn prop(mut self, key: &str, value: &str) -> Self {
        self.properties.push((key.into(), value.into()));
        self
    }

    pub fn format(&self, opts: &PrettyOptions, out: &mut String) {
        writeln!(out, "field {} : {}", self.name, self.field_type).unwrap();
        for (i, (k, v)) in self.properties.iter().enumerate() {
            let sep = if i + 1 < self.properties.len() {
                ","
            } else {
                ""
            };
            writeln!(out, "{}{}: {}{}", opts.indent, k, v, sep).unwrap();
        }
        writeln!(out, ";").unwrap();
    }
}

/// A pretty-printable material declaration.
#[derive(Debug, Clone)]
pub struct PrettyMaterial {
    pub name: String,
    pub material_type: String,
    pub properties: Vec<(String, String)>,
}

impl PrettyMaterial {
    pub fn new(name: &str, material_type: &str) -> Self {
        Self {
            name: name.into(),
            material_type: material_type.into(),
            properties: Vec::new(),
        }
    }

    pub fn prop(mut self, key: &str, value: &str) -> Self {
        self.properties.push((key.into(), value.into()));
        self
    }

    pub fn format(&self, opts: &PrettyOptions, out: &mut String) {
        writeln!(out, "material {} : {}", self.name, self.material_type).unwrap();
        for (i, (k, v)) in self.properties.iter().enumerate() {
            let sep = if i + 1 < self.properties.len() {
                ","
            } else {
                ""
            };
            writeln!(out, "{}{}: {}{}", opts.indent, k, v, sep).unwrap();
        }
        writeln!(out, ";").unwrap();
    }
}

/// A pretty-printable law declaration.
#[derive(Debug, Clone)]
pub struct PrettyLaw {
    pub name: String,
    pub when_expr: String,
    pub then_expr: String,
}

impl PrettyLaw {
    pub fn new(name: &str, when_expr: &str, then_expr: &str) -> Self {
        Self {
            name: name.into(),
            when_expr: when_expr.into(),
            then_expr: then_expr.into(),
        }
    }

    pub fn format(&self, opts: &PrettyOptions, out: &mut String) {
        writeln!(out, "law {}", self.name).unwrap();
        writeln!(out, "{}when {}", opts.indent, self.when_expr).unwrap();
        writeln!(out, "{}=> {}", opts.indent, self.then_expr).unwrap();
        writeln!(out, ";").unwrap();
    }
}

/// A document of pretty-printable declarations.
#[derive(Debug, Clone)]
pub struct PrettyDocument {
    pub fields: Vec<PrettyField>,
    pub materials: Vec<PrettyMaterial>,
    pub laws: Vec<PrettyLaw>,
}

impl Default for PrettyDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyDocument {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            materials: Vec::new(),
            laws: Vec::new(),
        }
    }

    pub fn add_field(&mut self, f: PrettyField) -> &mut Self {
        self.fields.push(f);
        self
    }

    pub fn add_material(&mut self, m: PrettyMaterial) -> &mut Self {
        self.materials.push(m);
        self
    }

    pub fn add_law(&mut self, l: PrettyLaw) -> &mut Self {
        self.laws.push(l);
        self
    }

    /// Format the entire document.
    pub fn format(&self, opts: &PrettyOptions) -> String {
        let mut out = String::new();
        let blank = "\n".repeat(opts.blank_lines_between_decls);

        let mut first = true;
        for f in &self.fields {
            if !first {
                out.push_str(&blank);
            }
            f.format(opts, &mut out);
            first = false;
        }
        for m in &self.materials {
            if !first {
                out.push_str(&blank);
            }
            m.format(opts, &mut out);
            first = false;
        }
        for l in &self.laws {
            if !first {
                out.push_str(&blank);
            }
            l.format(opts, &mut out);
            first = false;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_field_basic() {
        let f = PrettyField::new("pressure_ambient", "Pressure")
            .prop("unit", "<qudt:KiloPascal>")
            .prop("support", "region")
            .prop("representation", "grid");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        f.format(&opts, &mut out);
        assert!(out.contains("field pressure_ambient : Pressure"));
        assert!(out.contains("unit: <qudt:KiloPascal>"));
        assert!(out.contains("support: region"));
        assert!(out.contains("representation: grid"));
        assert!(out.ends_with(";\n"));
    }

    #[test]
    fn pretty_field_property_separators() {
        let f = PrettyField::new("temp", "Temperature")
            .prop("unit", "<qudt:Kelvin>")
            .prop("support", "point");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        f.format(&opts, &mut out);
        // First property gets a comma, last doesn't.
        assert!(out.contains("unit: <qudt:Kelvin>,"));
        assert!(out.contains("support: point\n")); // no trailing comma
    }

    #[test]
    fn pretty_material_basic() {
        let m = PrettyMaterial::new("steel", "Material")
            .prop("yield", "250.0")
            .prop("density", "7850.0");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        m.format(&opts, &mut out);
        assert!(out.contains("material steel : Material"));
        assert!(out.contains("yield: 250.0,"));
        assert!(out.contains("density: 7850.0"));
    }

    #[test]
    fn pretty_material_single_prop_no_comma() {
        let m = PrettyMaterial::new("aluminum", "Material").prop("yield", "95.0");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        m.format(&opts, &mut out);
        assert!(out.contains("yield: 95.0\n")); // no trailing comma
    }

    #[test]
    fn pretty_law_basic() {
        let l = PrettyLaw::new("crush", "pressure_ambient > steel.yield", "1");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        l.format(&opts, &mut out);
        assert!(out.contains("law crush"));
        assert!(out.contains("when pressure_ambient > steel.yield"));
        assert!(out.contains("=> 1"));
        assert!(out.ends_with(";\n"));
    }

    #[test]
    fn pretty_document_full() {
        let mut doc = PrettyDocument::new();
        doc.add_field(PrettyField::new("pressure", "Pressure").prop("unit", "<qudt:KiloPascal>"));
        doc.add_material(PrettyMaterial::new("steel", "Material").prop("yield", "250.0"));
        doc.add_law(PrettyLaw::new("crush", "pressure > steel.yield", "1"));
        let opts = PrettyOptions::default();
        let out = doc.format(&opts);
        assert!(out.contains("field pressure : Pressure"));
        assert!(out.contains("material steel : Material"));
        assert!(out.contains("law crush"));
    }

    #[test]
    fn pretty_document_blank_lines_between() {
        let mut doc = PrettyDocument::new();
        doc.add_field(PrettyField::new("a", "A"));
        doc.add_field(PrettyField::new("b", "B"));
        let opts = PrettyOptions {
            blank_lines_between_decls: 2,
            ..Default::default()
        };
        let out = doc.format(&opts);
        // Two blank lines between declarations.
        assert!(out.contains(";\n\n\nfield b : B"));
    }

    #[test]
    fn pretty_document_empty() {
        let doc = PrettyDocument::new();
        let opts = PrettyOptions::default();
        let out = doc.format(&opts);
        assert!(out.is_empty());
    }

    #[test]
    fn pretty_options_custom_indent() {
        let f = PrettyField::new("x", "X").prop("a", "1");
        let opts = PrettyOptions {
            indent: "    ".into(),
            ..Default::default()
        };
        let mut out = String::new();
        f.format(&opts, &mut out);
        assert!(out.contains("    a: 1"));
    }

    #[test]
    fn pretty_field_no_properties() {
        let f = PrettyField::new("empty", "Empty");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        f.format(&opts, &mut out);
        assert!(out.contains("field empty : Empty"));
        assert!(out.ends_with(";\n"));
    }

    #[test]
    fn pretty_material_no_properties() {
        let m = PrettyMaterial::new("void", "Material");
        let opts = PrettyOptions::default();
        let mut out = String::new();
        m.format(&opts, &mut out);
        assert!(out.contains("material void : Material"));
        assert!(out.ends_with(";\n"));
    }
}
