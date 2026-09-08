//! G-COORD map chrome: Realm + Position on live Cosmic.* / SPARQL remaps.
//!
//! Not a CRS Host. Not QDNF. DNS/IP-free networking stays a design programme.

use std::collections::BTreeMap;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use crate::browser::open_maps::{
    normalize_osm_layer, osm_embed_url, osm_site_url, OSM_COPYRIGHT_URL, OSM_DEFAULT_HALF_SPAN_DEG,
    OSM_DEFAULT_LAYER,
};
use crate::vibe_host::{capability_invoke, Span, Value};

const NORTH_SPRING_LAT: f64 = -37.8;
const NORTH_SPRING_LON: f64 = 144.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Realm {
    Earth,
    Cosmos,
    Fictional,
}

impl Realm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Earth => "earth",
            Self::Cosmos => "cosmos",
            Self::Fictional => "fictional",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        // Unicode case-fold; never ASCII-only. Machine ids stay earth/cosmos/fictional.
        match s.trim().to_lowercase().as_str() {
            "earth" | "地球" | "tierra" | "terre" => Some(Self::Earth),
            "cosmos" | "宇宙" | "kosmos" => Some(Self::Cosmos),
            "fictional" | "fiction" | "speculative" | "虚构" | "fictionnel" => {
                Some(Self::Fictional)
            }
            "viewpoint" | "視点" => Some(Self::Earth),
            _ => None,
        }
    }

    /// Live Family.method that carries this realm. No dotted invent.
    pub fn invoke_id(self) -> &'static str {
        match self {
            Self::Earth => "Cosmic.geodetic_to_ecef",
            Self::Cosmos => "Cosmic.body_profile",
            Self::Fictional => "Cosmic.stardate_to_gregorian",
        }
    }

    pub fn system(self) -> &'static str {
        match self {
            Self::Earth => "wgs84",
            Self::Cosmos => "ocs",
            Self::Fictional => "stardate",
        }
    }
}

pub fn evaluate_realm(realm: Realm) -> Result<Value, String> {
    let mut args = BTreeMap::new();
    match realm {
        Realm::Earth => {
            args.insert("lat_deg".into(), Value::F64(NORTH_SPRING_LAT));
            args.insert("lon_deg".into(), Value::F64(NORTH_SPRING_LON));
            args.insert("alt_m".into(), Value::F64(0.0));
        }
        Realm::Cosmos => {
            args.insert("name".into(), Value::String("earth".into()));
        }
        Realm::Fictional => {
            args.insert("stardate".into(), Value::F64(41000.0));
        }
    }
    capability_invoke(realm.invoke_id(), &Value::Record(args), Span::point(0))
        .map_err(|e| format!("{}: {}", e.code.as_str(), e.message))
}

fn display_value(v: &Value) -> String {
    match v {
        Value::F64(n) => format!("{n:.4}"),
        Value::I64(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Record(map) => map
            .iter()
            .map(|(k, val)| format!("{k}={}", display_value(val)))
            .collect::<Vec<_>>()
            .join(" · "),
        Value::List(items) => format!("[{} rows]", items.len()),
        other => format!("{other:?}"),
    }
}

/// Map container: one geo path + one non-geo path on live remaps.
pub fn build_map_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("g-coord-map");
    wrapper.set_attribute("data-shape", "container").ok();
    wrapper.set_attribute("data-media-surface", "map").ok();
    wrapper.set_attribute("data-coord-bind", "remap").ok();
    wrapper
        .set_attribute("data-realm", Realm::Earth.as_str())
        .ok();
    super::surface_aspects::mark(&wrapper, "entrance");
    let wrap_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrap_el
        .style()
        .set_css_text("display:flex;flex-direction:column;flex:1;gap:8px;min-height:0;");

    let realm_row = document.create_element("div").unwrap();
    realm_row.set_class_name("realm-chip-row");
    realm_row.set_attribute("role", "tablist").ok();
    for (realm, title) in [
        (
            Realm::Earth,
            "Earth · WGS84 via Cosmic.geodetic_to_ecef. Graph query remaps to GraphDatabase.sparql.",
        ),
        (
            Realm::Cosmos,
            "Cosmos · OCS body profile via Cosmic.body_profile. FLRW is Cosmic.flrw_distance.",
        ),
        (
            Realm::Fictional,
            "Fiction · authored place + Cosmic.stardate_to_gregorian. Not a network address.",
        ),
    ] {
        let chip = document.create_element("button").unwrap();
        chip.set_class_name("realm-chip");
        chip.set_attribute("type", "button").ok();
        chip.set_attribute("data-realm", realm.as_str()).ok();
        chip.set_attribute("role", "tab").ok();
        chip.set_attribute("title", title).ok();
        chip.set_text_content(Some(match realm {
            Realm::Earth => "Earth",
            Realm::Cosmos => "Cosmos",
            Realm::Fictional => "Fiction",
        }));
        if realm == Realm::Earth {
            chip.set_attribute("aria-pressed", "true").ok();
            chip.class_list().add_1("active").ok();
        } else {
            chip.set_attribute("aria-pressed", "false").ok();
        }
        realm_row.append_child(&chip).unwrap();
    }
    wrapper.append_child(&realm_row).unwrap();

    let pos = document.create_element("div").unwrap();
    pos.set_class_name("g-coord-position");
    pos.set_attribute("data-aspect", "layout").ok();
    pos.set_id("g-coord-position");
    wrapper.append_child(&pos).unwrap();

    let honesty = document.create_element("div").unwrap();
    honesty.set_class_name("gated-reason");
    honesty.set_id("g-coord-honesty");
    honesty.set_attribute("role", "status").ok();
    wrapper.append_child(&honesty).unwrap();

    let result = document.create_element("div").unwrap();
    result.set_class_name("g-coord-result diag-report");
    result.set_id("g-coord-result");
    result.set_attribute("data-honesty", "local").ok();
    wrapper.append_child(&result).unwrap();

    let stage = document.create_element("div").unwrap();
    stage.set_class_name("g-coord-stage preview-stage");
    stage.set_id("g-coord-stage");
    stage.set_attribute("data-aspect", "stage").ok();
    mount_open_map_stage(document, &wrapper, &stage);
    wrapper.append_child(&stage).unwrap();

    let time_row = document.create_element("div").unwrap();
    time_row.set_class_name("g-coord-timeline");
    time_row.set_attribute("data-aspect", "timeline").ok();
    let time_lab = document.create_element("label").unwrap();
    time_lab.set_text_content(Some("Timeline"));
    time_lab.set_attribute("for", "g-coord-time").ok();
    let time = document.create_element("input").unwrap();
    time.set_id("g-coord-time");
    time.set_attribute("type", "range").ok();
    time.set_attribute("min", "0").ok();
    time.set_attribute("max", "100").ok();
    time.set_attribute("value", "50").ok();
    time.set_attribute("aria-label", "Map timeline (named beats, not a Host clock)")
        .ok();
    time_row.append_child(&time_lab).unwrap();
    time_row.append_child(&time).unwrap();
    wrapper.append_child(&time_row).unwrap();

    paint_realm(&wrapper, Realm::Earth);
    wire_realms(&wrapper);
    wire_osm_layers(&wrapper);
    wire_timeline(&wrapper, &time);

    wrapper
        .append_child(&super::render_preview::build(document, "map", 800, 480))
        .unwrap();
    wrapper
}

fn wire_realms(root: &Element) {
    let chips = root.query_selector_all(".realm-chip").unwrap();
    for i in 0..chips.length() {
        let chip = chips.get(i).unwrap().dyn_into::<Element>().unwrap();
        let root = root.clone();
        let chip_listen = chip.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let realm = chip
                .get_attribute("data-realm")
                .and_then(|s| Realm::from_str(&s))
                .unwrap_or(Realm::Earth);
            let all = root.query_selector_all(".realm-chip").unwrap();
            for j in 0..all.length() {
                let other = all.get(j).unwrap().dyn_into::<Element>().unwrap();
                let on = other.get_attribute("data-realm").as_deref() == Some(realm.as_str());
                other
                    .set_attribute("aria-pressed", if on { "true" } else { "false" })
                    .ok();
                let _ = other.class_list().toggle_with_force("active", on);
            }
            paint_realm(&root, realm);
        }) as Box<dyn FnMut(web_sys::Event)>);
        chip_listen
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn wire_timeline(root: &Element, input: &Element) {
    let root = root.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(target) = event.target() else {
            return;
        };
        let Ok(range) = target.dyn_into::<HtmlInputElement>() else {
            return;
        };
        let v: f64 = range.value().parse().unwrap_or(50.0);
        let beat = if v < 33.0 {
            "entrance"
        } else if v > 66.0 {
            "exit"
        } else {
            "dwell"
        };
        root.set_attribute("data-beat", beat).ok();
        let realm = root
            .get_attribute("data-realm")
            .and_then(|s| Realm::from_str(&s))
            .unwrap_or(Realm::Earth);
        if let Some(result) = root.query_selector("#g-coord-result").ok().flatten() {
            match realm {
                Realm::Fictional => {
                    let stardate = 41000.0 + (v - 50.0) * 20.0;
                    let mut args = BTreeMap::new();
                    args.insert("stardate".into(), Value::F64(stardate));
                    if let Ok(val) = capability_invoke(
                        "Cosmic.stardate_to_gregorian",
                        &Value::Record(args),
                        Span::point(0),
                    ) {
                        result.set_text_content(Some(&format!(
                            "Cosmic.stardate_to_gregorian · {}",
                            display_value(&val)
                        )));
                    }
                }
                Realm::Cosmos => {
                    let z = (v / 100.0) * 0.2;
                    let mut args = BTreeMap::new();
                    args.insert("z".into(), Value::F64(z));
                    if let Ok(val) = capability_invoke(
                        "Cosmic.flrw_distance",
                        &Value::Record(args),
                        Span::point(0),
                    ) {
                        result.set_text_content(Some(&format!(
                            "Cosmic.flrw_distance · {}",
                            display_value(&val)
                        )));
                    }
                }
                Realm::Earth => {}
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    input
        .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}


/// Re-paint open G-COORD map honesty when Native daemon connects (initial paint may race probe).
pub fn refresh_g_coord_from_daemon(document: &Document) {
    if let Ok(list) = document.query_selector_all(".g-coord-map") {
        for i in 0..list.length() {
            let Some(node) = list.item(i) else { continue };
            let Ok(root) = node.dyn_into::<Element>() else { continue };
            let realm = root
                .get_attribute("data-realm")
                .as_deref()
                .and_then(Realm::from_str)
                .unwrap_or(Realm::Earth);
            paint_realm(&root, realm);
        }
    }
}

fn paint_realm(root: &Element, realm: Realm) {
    root.set_attribute("data-realm", realm.as_str()).ok();
    root.set_attribute("data-coord-bind", "remap").ok();
    root.set_attribute("data-beat", "dwell").ok();

    if let Some(pos) = root.query_selector("#g-coord-position").ok().flatten() {
        let text = match realm {
            Realm::Earth => format!(
                "Position · realm=earth · system=wgs84 · lat={NORTH_SPRING_LAT} lon={NORTH_SPRING_LON} · North Spring / 北泉"
            ),
            Realm::Cosmos => "Position · realm=cosmos · system=ocs · body=Earth / 地球".into(),
            Realm::Fictional => {
                "Position · realm=fictional · system=stardate · place=Qo'noS · 克罗诺斯 (authored, not DNS)"
                    .into()
            }
        };
        pos.set_text_content(Some(&text));
    }

    if let Some(h) = root.query_selector("#g-coord-honesty").ok().flatten() {
        let sparql_note = if super::native_daemon::is_daemon_connected() {
            "GraphDatabase.sparql is live on the daemon when queried."
        } else {
            "held / not yet — open native daemon for GraphDatabase.sparql."
        };
        let map_note = match realm {
            Realm::Earth => {
                let layer = root
                    .get_attribute("data-osm-layer")
                    .unwrap_or_else(|| OSM_DEFAULT_LAYER.to_string());
                format!(
                    "OpenStreetMap {} · public open-map basemap (not daemon GIS; not a placeholder tile).",
                    normalize_osm_layer(&layer)
                )
            }
            Realm::Cosmos | Realm::Fictional => {
                "Authored realm chrome (not an open-map tile)."
                    .to_string()
            }
        };
        h.set_text_content(Some(&format!(
            "{map_note} Remap {} · {}. QDNF (no DNS/IP network) is not this surface.",
            realm.invoke_id(),
            sparql_note
        )));
    }

    if let Some(stage) = root.query_selector("#g-coord-stage").ok().flatten() {
        apply_stage_realm(&stage, realm);
    }

    if let Some(result) = root.query_selector("#g-coord-result").ok().flatten() {
        match evaluate_realm(realm) {
            Ok(val) => {
                result.set_attribute("data-honesty", "live").ok();
                result.set_text_content(Some(&format!(
                    "{} · {}",
                    realm.invoke_id(),
                    display_value(&val)
                )));
            }
            Err(err) => {
                result.set_attribute("data-honesty", "error").ok();
                result.set_text_content(Some(&err));
            }
        }
        if realm == Realm::Earth {
            maybe_sparql(&result);
        }
    }
}

fn maybe_sparql(result: &Element) {
    if !super::native_daemon::is_daemon_connected() {
        return;
    }
    let args = serde_json::json!({
        "query": "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 8",
        "take": 8
    });
    result.set_attribute("data-honesty", "running").ok();
    let result = result.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match super::native_daemon::daemon_invoke("GraphDatabase.sparql", args).await {
            Ok(response) if response.ok => {
                result.set_attribute("data-honesty", "live").ok();
                let prior = result.text_content().unwrap_or_default();
                result.set_text_content(Some(&format!(
                    "{prior}\nGraphDatabase.sparql · {}",
                    response.value
                )));
            }
            Ok(response) => {
                result.set_attribute("data-honesty", "unavailable").ok();
                result.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("held / not yet — GraphDatabase.sparql returned no bindings (daemon up; graph may be empty)."),
                ));
            }
            Err(error) => {
                result.set_attribute("data-honesty", "error").ok();
                result.set_text_content(Some(&error));
            }
        }
    });
}

fn current_osm_layer(root: &Element) -> &'static str {
    root.get_attribute("data-osm-layer")
        .map(|s| normalize_osm_layer(&s))
        .unwrap_or(OSM_DEFAULT_LAYER)
}

fn set_osm_iframe_src(root: &Element, frame: &Element) {
    let layer = current_osm_layer(root);
    let src = osm_embed_url(
        NORTH_SPRING_LAT,
        NORTH_SPRING_LON,
        layer,
        OSM_DEFAULT_HALF_SPAN_DEG,
    );
    frame.set_attribute("src", &src).ok();
    frame
        .set_attribute(
            "title",
            &format!("OpenStreetMap {layer} · North Spring / 北泉"),
        )
        .ok();
}

fn mount_open_map_stage(document: &Document, root: &Element, stage: &Element) {
    root.set_attribute("data-osm-layer", OSM_DEFAULT_LAYER).ok();
    root.set_attribute("data-basemap", "openstreetmap").ok();

    let osm = document.create_element("div").unwrap();
    osm.set_class_name("osm-open-map");
    osm.set_attribute("data-basemap", "openstreetmap").ok();

    let layers = document.create_element("div").unwrap();
    layers.set_class_name("gis-layer-bar");
    layers
        .set_attribute("aria-label", "Open map layers")
        .ok();
    layers.set_attribute("role", "tablist").ok();
    for (id, label) in [
        ("mapnik", "OSM"),
        ("cyclemap", "Cycle"),
        ("transportmap", "Transit"),
        ("hot", "HOT"),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("gis-layer-btn");
        btn.set_attribute("type", "button").ok();
        btn.set_attribute("data-osm-layer", id).ok();
        btn.set_attribute("role", "tab").ok();
        btn.set_text_content(Some(label));
        if id == OSM_DEFAULT_LAYER {
            btn.class_list().add_1("active").ok();
            btn.set_attribute("aria-pressed", "true").ok();
        } else {
            btn.set_attribute("aria-pressed", "false").ok();
        }
        layers.append_child(&btn).unwrap();
    }
    osm.append_child(&layers).unwrap();

    let frame = document.create_element("iframe").unwrap();
    frame.set_class_name("osm-embed-frame");
    frame.set_attribute("loading", "lazy").ok();
    frame.set_attribute("referrerpolicy", "no-referrer-when-downgrade").ok();
    frame.set_attribute("allow", "fullscreen").ok();
    set_osm_iframe_src(root, &frame);
    osm.append_child(&frame).unwrap();

    let attrib = document.create_element("p").unwrap();
    attrib.set_class_name("osm-attribution");
    attrib.set_inner_html(&format!(
        "<a href=\"{OSM_COPYRIGHT_URL}\" target=\"_blank\" rel=\"noopener noreferrer\">© OpenStreetMap contributors</a> · \
         <a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">View larger map</a>",
        osm_site_url(NORTH_SPRING_LAT, NORTH_SPRING_LON, 13)
    ));
    osm.append_child(&attrib).unwrap();
    stage.append_child(&osm).unwrap();

    let authored = document.create_element("div").unwrap();
    authored.set_class_name("gis-authored-stage");
    authored.set_attribute("hidden", "").ok();
    stage.append_child(&authored).unwrap();
}

fn apply_stage_realm(stage: &Element, realm: Realm) {
    let osm = stage.query_selector(".osm-open-map").ok().flatten();
    let authored = stage.query_selector(".gis-authored-stage").ok().flatten();
    match realm {
        Realm::Earth => {
            if let Some(osm) = osm {
                osm.remove_attribute("hidden").ok();
            }
            if let Some(authored) = authored {
                authored.set_attribute("hidden", "").ok();
                authored.set_inner_html("");
            }
        }
        Realm::Cosmos | Realm::Fictional => {
            if let Some(osm) = osm {
                osm.set_attribute("hidden", "").ok();
            }
            if let Some(authored) = authored {
                authored.remove_attribute("hidden").ok();
                authored.set_inner_html(&authored_realm_svg(realm));
            }
        }
    }
}

fn authored_realm_svg(realm: Realm) -> String {
    match realm {
        Realm::Earth => String::new(),
        Realm::Cosmos => concat!(
            r#"<svg class="gis-map-svg" viewBox="0 0 400 300">"#,
            r#"<circle cx="200" cy="150" r="28" fill="rgba(168,85,247,0.35)" stroke="var(--media-3d)" stroke-width="1.5"/>"#,
            r#"<circle cx="200" cy="150" r="70" fill="none" stroke="rgba(168,85,247,0.35)" stroke-dasharray="4 6"/>"#,
            r#"<text x="20" y="24" fill="var(--media-3d)" font-size="10" font-family="sans-serif">OCS · Cosmic.body_profile · 地球</text>"#,
            r#"<text x="20" y="280" fill="var(--text-muted)" font-size="9" font-family="sans-serif">FLRW / body math — not a star map Host</text>"#,
            "</svg>",
        )
        .into(),
        Realm::Fictional => concat!(
            r#"<svg class="gis-map-svg" viewBox="0 0 400 300">"#,
            r#"<rect x="40" y="40" width="320" height="220" fill="none" stroke="var(--media-film)" stroke-dasharray="6 4"/>"#,
            r#"<text x="56" y="80" fill="var(--media-film)" font-size="12" font-family="sans-serif">Qo'noS · 克罗诺斯</text>"#,
            r#"<text x="56" y="104" fill="var(--text-muted)" font-size="9" font-family="sans-serif">stardate 41000 · Cosmic.stardate_to_gregorian</text>"#,
            r#"<text x="56" y="240" fill="var(--text-muted)" font-size="9" font-family="sans-serif">Not DNS. Not QDNF. Same container chrome.</text>"#,
            "</svg>",
        )
        .into(),
    }
}

fn wire_osm_layers(root: &Element) {
    let buttons = root.query_selector_all(".gis-layer-btn").unwrap();
    for i in 0..buttons.length() {
        let btn = buttons.get(i).unwrap().dyn_into::<Element>().unwrap();
        let root = root.clone();
        let btn_listen = btn.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let layer = btn
                .get_attribute("data-osm-layer")
                .map(|s| normalize_osm_layer(&s).to_string())
                .unwrap_or_else(|| OSM_DEFAULT_LAYER.to_string());
            root.set_attribute("data-osm-layer", &layer).ok();
            let all = root.query_selector_all(".gis-layer-btn").unwrap();
            for j in 0..all.length() {
                let other = all.get(j).unwrap().dyn_into::<Element>().unwrap();
                let on = other.get_attribute("data-osm-layer").as_deref() == Some(layer.as_str());
                other
                    .set_attribute("aria-pressed", if on { "true" } else { "false" })
                    .ok();
                let _ = other.class_list().toggle_with_force("active", on);
            }
            if let Some(frame) = root.query_selector(".osm-embed-frame").ok().flatten() {
                set_osm_iframe_src(&root, &frame);
            }
            paint_realm(&root, Realm::Earth);
        }) as Box<dyn FnMut(web_sys::Event)>);
        btn_listen
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realms_remap_to_live_cosmic_ids() {
        assert_eq!(Realm::Earth.invoke_id(), "Cosmic.geodetic_to_ecef");
        assert_eq!(Realm::Cosmos.invoke_id(), "Cosmic.body_profile");
        assert_eq!(Realm::Fictional.invoke_id(), "Cosmic.stardate_to_gregorian");
        assert_eq!(Realm::Earth.system(), "wgs84");
        assert!(Realm::from_str("speculative") == Some(Realm::Fictional));
        assert_eq!(Realm::from_str("地球"), Some(Realm::Earth));
        assert_eq!(Realm::from_str("宇宙"), Some(Realm::Cosmos));
    }

    #[test]
    fn earth_open_map_defaults_to_osm_mapnik() {
        use crate::browser::open_maps::{osm_embed_url, OSM_DEFAULT_LAYER, OSM_EMBED_ORIGIN};
        let url = osm_embed_url(
            NORTH_SPRING_LAT,
            NORTH_SPRING_LON,
            OSM_DEFAULT_LAYER,
            crate::browser::open_maps::OSM_DEFAULT_HALF_SPAN_DEG,
        );
        assert!(url.starts_with(OSM_EMBED_ORIGIN));
        assert!(url.contains("layer=mapnik"));
        assert!(url.contains("marker=-37.8,144.9"));
        assert!(!authored_realm_svg(Realm::Earth).contains("North Spring"));
        assert!(authored_realm_svg(Realm::Cosmos).contains("OCS"));
        assert!(authored_realm_svg(Realm::Fictional).contains("Qo'noS"));
    }

    #[test]
    fn earth_and_fiction_evaluate_in_process() {
        let earth = evaluate_realm(Realm::Earth).expect("earth");
        match earth {
            Value::Record(map) => {
                assert_eq!(map.get("system"), Some(&Value::String("wgs84".into())));
            }
            other => panic!("{other:?}"),
        }
        let fiction = evaluate_realm(Realm::Fictional).expect("fiction");
        match fiction {
            Value::Record(map) => {
                assert_eq!(map.get("realm"), Some(&Value::String("fictional".into())));
            }
            other => panic!("{other:?}"),
        }
        let cosmos = evaluate_realm(Realm::Cosmos).expect("cosmos");
        match cosmos {
            Value::Record(map) => {
                assert_eq!(map.get("name"), Some(&Value::String("Earth".into())));
            }
            other => panic!("{other:?}"),
        }
    }
}
