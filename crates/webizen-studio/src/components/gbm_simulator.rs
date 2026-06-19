use dioxus::prelude::*;

struct Lcg {
    state: u64,
}
impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let res = (self.state >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        if res == 0.0 {
            0.000001
        } else {
            res
        }
    }
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

#[component]
pub fn GbmSimulator() -> Element {
    let mut s0 = use_signal(|| 100.0f64);
    let mut mu = use_signal(|| 0.08f64);
    let mut sigma = use_signal(|| 0.20f64);
    let mut t_years = use_signal(|| 1.0f64);
    let mut num_paths = use_signal(|| 15usize);
    let mut seed = use_signal(|| 42u64);

    let paths = use_memo(move || {
        let s0_val = s0();
        let mu_val = mu();
        let sigma_val = sigma();
        let t_val = t_years();
        let paths_val = num_paths().min(100);
        let steps = 100;
        let dt = t_val / steps as f64;

        let mut rng = Lcg::new(seed());
        let mut all_paths = Vec::new();

        for _ in 0..paths_val {
            let mut path = Vec::with_capacity(steps + 1);
            path.push(s0_val);
            let mut current_s = s0_val;
            for _ in 0..steps {
                let z = rng.next_normal();
                current_s = current_s
                    * ((mu_val - 0.5 * sigma_val * sigma_val) * dt + sigma_val * dt.sqrt() * z)
                        .exp();
                path.push(current_s);
            }
            all_paths.push(path);
        }
        all_paths
    });

    let min_max = use_memo(move || {
        let p = paths();
        let mut min = s0();
        let mut max = s0();
        for path in p.iter() {
            for &val in path.iter() {
                if val < min {
                    min = val;
                }
                if val > max {
                    max = val;
                }
            }
        }
        let diff = max - min;
        if diff == 0.0 {
            (min * 0.9, max * 1.1)
        } else {
            (min - diff * 0.1, max + diff * 0.1)
        }
    });

    let svg_paths = paths()
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let min = min_max().0;
            let max = min_max().1;
            let range = max - min;
            let pts = path
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let x = (i as f64 / 100.0) * 1000.0;
                    let y = 400.0 - ((v - min) / range) * 400.0;
                    format!("{},{}", x, y)
                })
                .collect::<Vec<_>>()
                .join(" ");
            (idx, pts)
        })
        .collect::<Vec<_>>();

    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: linear-gradient(135deg, #1e1b4b, #312e81); border-radius: 16px; color: #e0e7ff; font-family: 'Inter', system-ui, sans-serif; box-shadow: 0 20px 40px rgba(0,0,0,0.5); display: flex; flex-direction: column; gap: 2rem; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 1rem;",
                h2 {
                    style: "margin: 0; font-size: 2.5rem; font-weight: 800; background: linear-gradient(to right, #818cf8, #c084fc); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                    "Geometric Brownian Motion"
                }
                button {
                    onclick: move |_| seed.set(seed() + 1),
                    style: "background: rgba(129, 140, 248, 0.2); border: 1px solid rgba(129, 140, 248, 0.5); color: #818cf8; padding: 0.6rem 1.2rem; border-radius: 8px; font-weight: 600; cursor: pointer; transition: all 0.2s; box-shadow: 0 4px 10px rgba(129, 140, 248, 0.2);",
                    "Rerun Simulation"
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1.5rem;",

                div {
                    style: "background: rgba(0,0,0,0.2); padding: 1.2rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    label { style: "display: block; font-size: 0.8rem; color: #a5b4fc; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Initial Price (S0)" }
                    input {
                        type: "number", value: "{s0()}",
                        oninput: move |e| s0.set(e.value().parse().unwrap_or(100.0)),
                        style: "width: 100%; background: transparent; border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(0,0,0,0.2); padding: 1.2rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    label { style: "display: block; font-size: 0.8rem; color: #a5b4fc; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Drift (μ)" }
                    input {
                        type: "number", step: "0.01", value: "{mu()}",
                        oninput: move |e| mu.set(e.value().parse().unwrap_or(0.08)),
                        style: "width: 100%; background: transparent; border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(0,0,0,0.2); padding: 1.2rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    label { style: "display: block; font-size: 0.8rem; color: #a5b4fc; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Volatility (σ)" }
                    input {
                        type: "number", step: "0.01", value: "{sigma()}",
                        oninput: move |e| sigma.set(e.value().parse().unwrap_or(0.2)),
                        style: "width: 100%; background: transparent; border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(0,0,0,0.2); padding: 1.2rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    label { style: "display: block; font-size: 0.8rem; color: #a5b4fc; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Time (Years)" }
                    input {
                        type: "number", step: "0.1", value: "{t_years()}",
                        oninput: move |e| t_years.set(e.value().parse().unwrap_or(1.0)),
                        style: "width: 100%; background: transparent; border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(0,0,0,0.2); padding: 1.2rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    label { style: "display: block; font-size: 0.8rem; color: #a5b4fc; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Paths" }
                    input {
                        type: "number", step: "1", value: "{num_paths()}",
                        oninput: move |e| num_paths.set(e.value().parse().unwrap_or(15)),
                        style: "width: 100%; background: transparent; border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; font-size: 1.1rem; outline: none;"
                    }
                }
            }

            // SVG Chart Container
            div {
                style: "background: rgba(0,0,0,0.3); border-radius: 16px; padding: 1.5rem; border: 1px solid rgba(255,255,255,0.05); height: 400px; position: relative; overflow: hidden; box-shadow: inset 0 4px 20px rgba(0,0,0,0.4);",
                svg {
                    width: "100%", height: "100%", preserve_aspect_ratio: "none", view_box: "0 0 1000 400",

                    // Grid lines
                    for i in 0..5 {
                        line {
                            x1: "0", y1: "{i * 100}", x2: "1000", y2: "{i * 100}",
                            stroke: "rgba(255,255,255,0.1)", stroke_width: "1", stroke_dasharray: "4 4"
                        }
                    }

                    // Paths
                    for (idx, pts) in svg_paths {
                        polyline {
                            fill: "none",
                            stroke: "hsla({(idx as f64 * 137.5) % 360.0}, 80%, 70%, 0.8)",
                            stroke_width: "2",
                            points: "{pts}",
                            style: "transition: all 0.5s ease-in-out;"
                        }
                    }
                }

                // Labels
                div { style: "position: absolute; top: 1.5rem; left: 2rem; color: #a5b4fc; font-size: 0.9rem; font-weight: bold; background: rgba(0,0,0,0.5); padding: 0.2rem 0.5rem; border-radius: 4px;", "${min_max().1:.2}" }
                div { style: "position: absolute; bottom: 1.5rem; left: 2rem; color: #a5b4fc; font-size: 0.9rem; font-weight: bold; background: rgba(0,0,0,0.5); padding: 0.2rem 0.5rem; border-radius: 4px;", "${min_max().0:.2}" }
            }
        }
    }
}
