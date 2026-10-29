//! A made-up home in the bridge's own resource format, so the app can be
//! tried (and developed) without a bridge.

use serde_json::{Value, json};

use crate::color::{Rgb, rgb_to_xy};

enum Look {
    Color(f64, f64, f64),
    Temp(u16),
    White,
}

struct DemoLight {
    name: &'static str,
    archetype: &'static str,
    on: bool,
    brightness: f64,
    look: Look,
    color_capable: bool,
}

const fn light(
    name: &'static str,
    archetype: &'static str,
    on: bool,
    brightness: f64,
    look: Look,
) -> DemoLight {
    let color_capable = matches!(look, Look::Color(..));
    DemoLight {
        name,
        archetype,
        on,
        brightness,
        look,
        color_capable,
    }
}

fn xy(r: f64, g: f64, b: f64) -> Value {
    let (x, y) = rgb_to_xy(Rgb { r, g, b });
    json!({"x": x, "y": y})
}

/// Builds the demo home's resources.
pub fn resources() -> Vec<Value> {
    let rooms: Vec<(&str, &str, Vec<DemoLight>)> = vec![
        (
            "Living room",
            "living_room",
            vec![
                light(
                    "Table",
                    "table_shade",
                    true,
                    70.0,
                    Look::Color(0.95, 0.45, 0.85),
                ),
                light(
                    "Ceiling",
                    "ceiling_round",
                    true,
                    55.0,
                    Look::Color(0.55, 0.35, 1.0),
                ),
                light("Floor lamp", "floor_shade", true, 40.0, Look::Temp(400)),
            ],
        ),
        (
            "Backyard",
            "garden",
            vec![
                light(
                    "Terrace lantern 1",
                    "wall_lantern",
                    true,
                    80.0,
                    Look::Color(1.0, 0.35, 0.55),
                ),
                light(
                    "Terrace lantern 2",
                    "wall_lantern",
                    true,
                    80.0,
                    Look::Color(1.0, 0.6, 0.3),
                ),
            ],
        ),
        (
            "Bedroom",
            "bedroom",
            vec![
                light("Bedside left", "table_wash", false, 30.0, Look::Temp(454)),
                light("Bedside right", "table_wash", false, 30.0, Look::Temp(454)),
                light("Wardrobe", "ceiling_square", false, 100.0, Look::White),
            ],
        ),
        (
            "Home office",
            "office",
            vec![
                light("Desk", "desk_lamp", true, 100.0, Look::Temp(233)),
                light(
                    "Shelf strip",
                    "hue_lightstrip",
                    true,
                    60.0,
                    Look::Color(0.3, 0.75, 1.0),
                ),
            ],
        ),
    ];

    let mut out = Vec::new();
    let mut n = 0;
    let mut next_id = |prefix: &str| {
        n += 1;
        format!("{prefix}-{n:04}")
    };
    let mut all_grouped_lights = Vec::new();
    let mut downstairs = Vec::new();

    for (room_name, room_arch, lights) in rooms {
        let room_id = next_id("room");
        let gl_id = next_id("grouped-light");
        let mut devices = Vec::new();
        let mut light_ids = Vec::new();
        for l in &lights {
            let dev_id = next_id("device");
            let light_id = next_id("light");
            let zb_id = next_id("zigbee");
            let mut res = json!({
                "id": light_id, "type": "light",
                "owner": {"rid": dev_id, "rtype": "device"},
                "metadata": {"name": l.name, "archetype": l.archetype},
                "on": {"on": l.on},
                "dimming": {"brightness": l.brightness, "min_dim_level": 0.2},
            });
            if !matches!(l.look, Look::White) {
                res["color_temperature"] = json!({
                    "mirek": null, "mirek_valid": false,
                    "mirek_schema": {"mirek_minimum": 153, "mirek_maximum": 500}
                });
            }
            if l.color_capable || l.archetype == "floor_shade" {
                res["color"] = json!({"xy": xy(1.0, 0.8, 0.6), "gamut_type": "C"});
            }
            match l.look {
                Look::Color(r, g, b) => res["color"]["xy"] = xy(r, g, b),
                Look::Temp(m) => {
                    res["color_temperature"]["mirek"] = json!(m);
                    res["color_temperature"]["mirek_valid"] = json!(true);
                }
                Look::White => {}
            }
            out.push(res);
            out.push(json!({
                "id": dev_id, "type": "device",
                "metadata": {"name": l.name, "archetype": l.archetype},
                "services": [{"rid": light_id, "rtype": "light"}, {"rid": zb_id, "rtype": "zigbee_connectivity"}],
            }));
            out.push(json!({
                "id": zb_id, "type": "zigbee_connectivity",
                "owner": {"rid": dev_id, "rtype": "device"},
                "status": if l.name == "Wardrobe" { "connectivity_issue" } else { "connected" },
            }));
            devices.push(json!({"rid": dev_id, "rtype": "device"}));
            light_ids.push(light_id);
        }
        if room_name == "Living room" || room_name == "Home office" {
            downstairs.extend(light_ids.iter().cloned());
        }
        out.push(json!({
            "id": room_id, "type": "room",
            "metadata": {"name": room_name, "archetype": room_arch},
            "children": devices,
            "services": [{"rid": gl_id, "rtype": "grouped_light"}],
        }));
        out.push(json!({
            "id": gl_id, "type": "grouped_light",
            "owner": {"rid": room_id, "rtype": "room"},
            "on": {"on": lights.iter().any(|l| l.on)},
            "dimming": {"brightness": 60.0},
        }));
        all_grouped_lights.push(gl_id);

        for (scene_name, colors) in scenes_for(room_name) {
            let actions: Vec<Value> = light_ids
                .iter()
                .zip(lights.iter())
                .enumerate()
                .map(|(i, (id, l))| {
                    let (r, g, b) = colors[i % colors.len()];
                    let mut action = json!({"on": {"on": true}, "dimming": {"brightness": 75.0}});
                    if l.color_capable {
                        action["color"] = json!({"xy": xy(r, g, b)});
                    } else if !matches!(l.look, Look::White) {
                        action["color_temperature"] =
                            json!({"mirek": if r > b { 400 } else { 220 }});
                    }
                    json!({"target": {"rid": id, "rtype": "light"}, "action": action})
                })
                .collect();
            out.push(json!({
                "id": next_id("scene"), "type": "scene",
                "metadata": {"name": scene_name},
                "group": {"rid": room_id, "rtype": "room"},
                "actions": actions,
                "palette": {"color": colors.iter().map(|&(r, g, b)| json!({"color": {"xy": xy(r, g, b)}})).collect::<Vec<_>>()},
                "status": {"active": "inactive"},
            }));
        }
    }

    let zone_id = next_id("zone");
    let zone_gl = next_id("grouped-light");
    out.push(json!({
        "id": zone_id, "type": "zone",
        "metadata": {"name": "Downstairs", "archetype": "downstairs"},
        "children": downstairs.iter().map(|id| json!({"rid": id, "rtype": "light"})).collect::<Vec<_>>(),
        "services": [{"rid": zone_gl, "rtype": "grouped_light"}],
    }));
    out.push(json!({
        "id": zone_gl, "type": "grouped_light",
        "owner": {"rid": zone_id, "rtype": "zone"},
        "on": {"on": true}, "dimming": {"brightness": 60.0},
    }));

    let home_id = next_id("bridge-home");
    let home_gl = next_id("grouped-light");
    out.push(json!({
        "id": home_id, "type": "bridge_home",
        "services": [{"rid": home_gl, "rtype": "grouped_light"}],
    }));
    out.push(json!({
        "id": home_gl, "type": "grouped_light",
        "owner": {"rid": home_id, "rtype": "bridge_home"},
        "on": {"on": true},
    }));
    out
}

type Palette = Vec<(f64, f64, f64)>;

fn scenes_for(room: &str) -> Vec<(&'static str, Palette)> {
    let read = ("Read", vec![(1.0, 0.92, 0.8)]);
    let dimmed = ("Dimmed", vec![(1.0, 0.75, 0.5)]);
    let mut v = vec![read, dimmed];
    match room {
        "Living room" => v.extend([
            (
                "Tropical twilight",
                vec![(0.95, 0.45, 0.3), (0.6, 0.2, 0.55), (1.0, 0.75, 0.35)],
            ),
            (
                "Arctic aurora",
                vec![(0.2, 0.85, 0.75), (0.25, 0.4, 1.0), (0.6, 0.3, 0.95)],
            ),
            (
                "Savanna sunset",
                vec![(1.0, 0.5, 0.15), (0.95, 0.3, 0.25), (1.0, 0.8, 0.4)],
            ),
        ]),
        "Backyard" => v.extend([
            (
                "Festive lights",
                vec![(1.0, 0.3, 0.6), (1.0, 0.65, 0.2), (0.95, 0.35, 0.95)],
            ),
            ("Lake mist", vec![(0.55, 0.75, 1.0), (0.7, 0.9, 0.95)]),
        ]),
        "Bedroom" => v.extend([("Nightlight", vec![(1.0, 0.45, 0.1)])]),
        _ => v.extend([
            ("Concentrate", vec![(0.85, 0.92, 1.0)]),
            ("Painted sky", vec![(1.0, 0.55, 0.6), (0.55, 0.45, 1.0)]),
        ]),
    }
    v
}
