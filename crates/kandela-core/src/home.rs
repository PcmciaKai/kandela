//! The bridge's resources, kept as raw JSON so that partial updates from the
//! event stream can be merged in without modelling every field, plus the
//! flattened [`Snapshot`] the UI renders.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use serde_json::{Value, json};

use crate::color::{self, Rgb};

/// Every resource the bridge reported, keyed by resource id.
#[derive(Debug, Default, Clone)]
pub struct Home {
    resources: HashMap<String, Value>,
}

/// An action the user took in the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    LightOn {
        id: String,
        on: bool,
    },
    LightBrightness {
        id: String,
        brightness: f64,
    },
    LightColor {
        id: String,
        x: f64,
        y: f64,
    },
    LightTemperature {
        id: String,
        mirek: u16,
    },
    /// `id` is a grouped_light id.
    GroupOn {
        id: String,
        on: bool,
    },
    GroupBrightness {
        id: String,
        brightness: f64,
    },
    RecallScene {
        id: String,
    },
    AllOn {
        on: bool,
    },
}

impl Command {
    /// Commands with the same key replace each other while waiting to be sent,
    /// so dragging a slider only sends the latest value.
    pub fn coalesce_key(&self) -> String {
        match self {
            Command::LightOn { id, .. } => format!("light/{id}/on"),
            Command::LightBrightness { id, .. } => format!("light/{id}/dimming"),
            Command::LightColor { id, .. } | Command::LightTemperature { id, .. } => {
                format!("light/{id}/color")
            }
            Command::GroupOn { id, .. } => format!("group/{id}/on"),
            Command::GroupBrightness { id, .. } => format!("group/{id}/dimming"),
            Command::RecallScene { id } => format!("scene/{id}"),
            Command::AllOn { .. } => "all/on".into(),
        }
    }
}

/// One `PUT /clip/v2/resource/{rtype}/{id}` request.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub rtype: &'static str,
    pub id: String,
    pub body: Value,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub rooms: Vec<GroupView>,
    pub zones: Vec<GroupView>,
    pub lights: Vec<LightView>,
    pub scenes: Vec<SceneView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub archetype: String,
    pub grouped_light_id: String,
    pub on: bool,
    pub brightness: f64,
    pub light_ids: Vec<String>,
    pub lights_on: usize,
    /// Colors of the lights that are on, for tinting the room card.
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LightView {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub on: bool,
    pub reachable: bool,
    pub brightness: f64,
    pub supports_dimming: bool,
    pub supports_color: bool,
    pub supports_temperature: bool,
    /// "color", "temperature" or "white".
    pub mode: String,
    pub color: String,
    /// Position on the color wheel (degrees, 0..1).
    pub hue: f64,
    pub saturation: f64,
    pub mirek: f64,
    pub mirek_min: f64,
    pub mirek_max: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SceneView {
    pub id: String,
    pub name: String,
    pub group_id: String,
    pub colors: Vec<String>,
    pub active: bool,
}

const WARM_WHITE: Rgb = Rgb {
    r: 1.0,
    g: 0.87,
    b: 0.68,
};

/// Recursively merges `patch` into `target`. Objects merge key by key;
/// everything else replaces the old value.
fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                merge(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (t, p) => *t = p.clone(),
    }
}

/// The state change a light request causes, including the color mode flags
/// the bridge would report afterwards.
fn light_patch(body: &Value) -> Value {
    let mut patch = body.clone();
    if body.get("color").is_some() {
        merge(
            &mut patch,
            &json!({"color_temperature": {"mirek": null, "mirek_valid": false}}),
        );
    }
    if body.get("color_temperature").is_some() {
        merge(
            &mut patch,
            &json!({"color_temperature": {"mirek_valid": true}}),
        );
    }
    patch
}

fn str_at<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(Value::as_str).unwrap_or_default()
}

fn rids<'a>(v: &'a Value, field: &str, rtype: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    v.get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(move |r| r.get("rtype").and_then(Value::as_str) == Some(rtype))
        .filter_map(|r| r.get("rid").and_then(Value::as_str))
}

fn xy_of(v: &Value) -> Option<(f64, f64)> {
    Some((v.get("x")?.as_f64()?, v.get("y")?.as_f64()?))
}

impl Home {
    pub fn from_resources(resources: impl IntoIterator<Item = Value>) -> Self {
        let resources = resources
            .into_iter()
            .filter_map(|r| Some((r.get("id")?.as_str()?.to_owned(), r)))
            .collect();
        Self { resources }
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    fn get(&self, id: &str) -> Option<&Value> {
        self.resources.get(id)
    }

    fn of_type<'a>(&'a self, rtype: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.resources
            .values()
            .filter(move |r| r.get("type").and_then(Value::as_str) == Some(rtype))
    }

    /// Applies one message from the event stream. Returns whether anything changed.
    pub fn apply_event(&mut self, event: &Value) -> bool {
        let kind = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(data) = event.get("data").and_then(Value::as_array) else {
            return false;
        };
        let mut changed = false;
        for item in data {
            let Some(id) = item.get("id").and_then(Value::as_str) else {
                continue;
            };
            match kind {
                "update" => {
                    if let Some(existing) = self.resources.get_mut(id) {
                        merge(existing, item);
                        changed = true;
                    }
                }
                "add" => {
                    self.resources.insert(id.to_owned(), item.clone());
                    changed = true;
                }
                "delete" => changed |= self.resources.remove(id).is_some(),
                _ => {}
            }
        }
        changed
    }

    /// Light ids belonging to a room (via its devices) or a zone (directly).
    fn group_light_ids(&self, group: &Value) -> Vec<String> {
        let mut ids: Vec<String> = rids(group, "children", "light")
            .map(str::to_owned)
            .collect();
        for device_id in rids(group, "children", "device") {
            if let Some(device) = self.get(device_id) {
                ids.extend(rids(device, "services", "light").map(str::to_owned));
            }
        }
        ids.retain(|id| self.get(id).is_some());
        ids
    }

    fn group_of_grouped_light(&self, grouped_light_id: &str) -> Option<&Value> {
        let owner = self
            .get(grouped_light_id)?
            .pointer("/owner/rid")?
            .as_str()?;
        self.get(owner)
    }

    /// Translates a command into the HTTP requests that carry it out.
    pub fn requests_for(&self, command: &Command) -> Vec<Request> {
        let light = |id: &str, body| Request {
            rtype: "light",
            id: id.to_owned(),
            body,
        };
        let group = |id: &str, body| Request {
            rtype: "grouped_light",
            id: id.to_owned(),
            body,
        };
        match command {
            Command::LightOn { id, on } => vec![light(id, json!({"on": {"on": on}}))],
            Command::LightBrightness { id, brightness } => vec![light(
                id,
                json!({"on": {"on": true}, "dimming": {"brightness": brightness}}),
            )],
            Command::LightColor { id, x, y } => {
                vec![light(
                    id,
                    json!({"on": {"on": true}, "color": {"xy": {"x": x, "y": y}}}),
                )]
            }
            Command::LightTemperature { id, mirek } => vec![light(
                id,
                json!({"on": {"on": true}, "color_temperature": {"mirek": mirek}}),
            )],
            Command::GroupOn { id, on } => vec![group(id, json!({"on": {"on": on}}))],
            Command::GroupBrightness { id, brightness } => vec![group(
                id,
                json!({"on": {"on": true}, "dimming": {"brightness": brightness}}),
            )],
            Command::RecallScene { id } => vec![Request {
                rtype: "scene",
                id: id.clone(),
                body: json!({"recall": {"action": "active"}}),
            }],
            Command::AllOn { on } => {
                // The bridge_home resource owns a grouped_light covering every light.
                let home_group = self
                    .of_type("bridge_home")
                    .find_map(|h| rids(h, "services", "grouped_light").next());
                match home_group {
                    Some(gid) => vec![group(gid, json!({"on": {"on": on}}))],
                    None => self
                        .of_type("room")
                        .flat_map(|r| rids(r, "services", "grouped_light"))
                        .map(|gid| group(gid, json!({"on": {"on": on}})))
                        .collect(),
                }
            }
        }
    }

    /// Updates local state right away so the UI responds without waiting for
    /// the bridge. With `simulate`, also performs the side effects the bridge
    /// would report through the event stream (used by the demo).
    pub fn apply_locally(&mut self, command: &Command, simulate: bool) {
        for req in self.requests_for(command) {
            if req.rtype == "scene" {
                self.activate_scene(&req.id, simulate);
                continue;
            }
            let patch = if req.rtype == "light" {
                light_patch(&req.body)
            } else {
                req.body.clone()
            };
            if let Some(res) = self.resources.get_mut(&req.id) {
                merge(res, &patch);
            }
            if !simulate {
                continue;
            }
            if req.rtype == "grouped_light" {
                let members = match self.group_of_grouped_light(&req.id) {
                    Some(g) if g.get("type").and_then(Value::as_str) == Some("bridge_home") => self
                        .of_type("light")
                        .filter_map(|l| l.get("id")?.as_str().map(str::to_owned))
                        .collect(),
                    Some(g) => self.group_light_ids(g),
                    None => vec![],
                };
                for id in members {
                    if let Some(l) = self.resources.get_mut(&id) {
                        merge(l, &req.body);
                    }
                }
            }
            self.deactivate_scenes_touching(&req.id);
            self.recompute_grouped_lights();
        }
    }

    fn activate_scene(&mut self, scene_id: &str, simulate: bool) {
        let Some(scene) = self.get(scene_id).cloned() else {
            return;
        };
        let group_id = str_at(&scene, "/group/rid").to_owned();
        let siblings: Vec<String> = self
            .of_type("scene")
            .filter(|s| str_at(s, "/group/rid") == group_id)
            .filter_map(|s| s.get("id")?.as_str().map(str::to_owned))
            .collect();
        for id in siblings {
            let state = if id == scene_id { "static" } else { "inactive" };
            if let Some(s) = self.resources.get_mut(&id) {
                merge(s, &json!({"status": {"active": state}}));
            }
        }
        if simulate {
            for action in scene
                .get("actions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let target = str_at(action, "/target/rid").to_owned();
                if let (Some(light), Some(action)) =
                    (self.resources.get_mut(&target), action.get("action"))
                {
                    merge(light, &light_patch(action));
                }
            }
            self.recompute_grouped_lights();
        }
    }

    /// In the demo, changing a light by hand ends the scene its group was in.
    fn deactivate_scenes_touching(&mut self, resource_id: &str) {
        let groups: Vec<String> = self
            .of_type("room")
            .chain(self.of_type("zone"))
            .filter(|g| {
                rids(g, "services", "grouped_light").any(|id| id == resource_id)
                    || self.group_light_ids(g).iter().any(|id| id == resource_id)
            })
            .filter_map(|g| g.get("id")?.as_str().map(str::to_owned))
            .collect();
        for scene in self.resources.values_mut() {
            if scene.get("type").and_then(Value::as_str) == Some("scene")
                && groups.iter().any(|g| g == str_at(scene, "/group/rid"))
            {
                merge(scene, &json!({"status": {"active": "inactive"}}));
            }
        }
    }

    /// Derives each grouped_light's on/brightness from its member lights, as
    /// the bridge does.
    fn recompute_grouped_lights(&mut self) {
        let mut updates = Vec::new();
        for gl in self.of_type("grouped_light") {
            let gid = gl.get("id").and_then(Value::as_str).unwrap_or_default();
            let members: Vec<&Value> = match self.group_of_grouped_light(gid) {
                Some(g) if g.get("type").and_then(Value::as_str) == Some("bridge_home") => {
                    self.of_type("light").collect()
                }
                Some(g) => self
                    .group_light_ids(g)
                    .iter()
                    .filter_map(|id| self.get(id))
                    .collect(),
                None => continue,
            };
            let on: Vec<&&Value> = members
                .iter()
                .filter(|l| l.pointer("/on/on").and_then(Value::as_bool) == Some(true))
                .collect();
            let mut patch = json!({"on": {"on": !on.is_empty()}});
            if !on.is_empty() {
                let avg = on
                    .iter()
                    .filter_map(|l| l.pointer("/dimming/brightness").and_then(Value::as_f64))
                    .sum::<f64>()
                    / on.len() as f64;
                merge(&mut patch, &json!({"dimming": {"brightness": avg}}));
            }
            updates.push((gid.to_owned(), patch));
        }
        for (id, patch) in updates {
            if let Some(gl) = self.resources.get_mut(&id) {
                merge(gl, &patch);
            }
        }
    }

    fn light_view(&self, l: &Value) -> LightView {
        let id = str_at(l, "/id").to_owned();
        let owner = str_at(l, "/owner/rid");
        let reachable = self
            .of_type("zigbee_connectivity")
            .find(|z| str_at(z, "/owner/rid") == owner)
            .map(|z| str_at(z, "/status") == "connected")
            .unwrap_or(true);
        let on = l
            .pointer("/on/on")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let brightness = l
            .pointer("/dimming/brightness")
            .and_then(Value::as_f64)
            .unwrap_or(100.0);
        let ct = l.get("color_temperature");
        let mirek = ct.and_then(|c| c.get("mirek")).and_then(Value::as_f64);
        let mirek_valid = ct
            .and_then(|c| c.get("mirek_valid"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let xy = l.pointer("/color/xy").and_then(xy_of);

        let (mode, rgb) = match (mirek_valid, mirek, xy) {
            (true, Some(m), _) => ("temperature", color::mirek_to_rgb(m)),
            (_, _, Some((x, y))) => ("color", color::xy_to_rgb(x, y)),
            _ => ("white", WARM_WHITE),
        };
        let (hue, saturation, _) = color::rgb_to_hsv(rgb);
        LightView {
            id,
            name: str_at(l, "/metadata/name").to_owned(),
            archetype: str_at(l, "/metadata/archetype").to_owned(),
            on,
            reachable,
            brightness,
            supports_dimming: l.get("dimming").is_some(),
            supports_color: l.get("color").is_some(),
            supports_temperature: ct.is_some(),
            mode: mode.into(),
            color: rgb.to_hex(),
            hue,
            saturation,
            mirek: mirek.unwrap_or(366.0),
            mirek_min: l
                .pointer("/color_temperature/mirek_schema/mirek_minimum")
                .and_then(Value::as_f64)
                .unwrap_or(153.0),
            mirek_max: l
                .pointer("/color_temperature/mirek_schema/mirek_maximum")
                .and_then(Value::as_f64)
                .unwrap_or(500.0),
        }
    }

    fn group_view(&self, g: &Value, lights: &BTreeMap<String, LightView>) -> GroupView {
        let grouped_light_id = rids(g, "services", "grouped_light")
            .next()
            .unwrap_or_default()
            .to_owned();
        let gl = self.get(&grouped_light_id);
        let light_ids = self.group_light_ids(g);
        let members: Vec<&LightView> = light_ids.iter().filter_map(|id| lights.get(id)).collect();
        let lit: Vec<&&LightView> = members.iter().filter(|l| l.on).collect();
        let mut colors: Vec<String> = Vec::new();
        for l in &lit {
            if !colors.contains(&l.color) {
                colors.push(l.color.clone());
            }
        }
        colors.truncate(4);
        GroupView {
            id: str_at(g, "/id").to_owned(),
            kind: str_at(g, "/type").to_owned(),
            name: str_at(g, "/metadata/name").to_owned(),
            archetype: str_at(g, "/metadata/archetype").to_owned(),
            grouped_light_id,
            on: gl
                .and_then(|gl| gl.pointer("/on/on"))
                .and_then(Value::as_bool)
                .unwrap_or(!lit.is_empty()),
            brightness: gl
                .and_then(|gl| gl.pointer("/dimming/brightness"))
                .and_then(Value::as_f64)
                .unwrap_or(100.0),
            lights_on: lit.len(),
            light_ids,
            colors,
        }
    }

    fn scene_view(s: &Value) -> SceneView {
        let mut colors: Vec<String> = Vec::new();
        let push = |colors: &mut Vec<String>, rgb: Rgb| {
            let hex = rgb.to_hex();
            if !colors.contains(&hex) {
                colors.push(hex);
            }
        };
        let palette = s.pointer("/palette/color").and_then(Value::as_array);
        for c in palette.into_iter().flatten() {
            if let Some((x, y)) = c.pointer("/color/xy").and_then(xy_of) {
                push(&mut colors, color::xy_to_rgb(x, y));
            }
        }
        for a in s
            .get("actions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some((x, y)) = a.pointer("/action/color/xy").and_then(xy_of) {
                push(&mut colors, color::xy_to_rgb(x, y));
            } else if let Some(m) = a
                .pointer("/action/color_temperature/mirek")
                .and_then(Value::as_f64)
            {
                push(&mut colors, color::mirek_to_rgb(m));
            }
        }
        if colors.is_empty() {
            push(&mut colors, WARM_WHITE);
        }
        colors.truncate(4);
        let active = s.pointer("/status/active").and_then(Value::as_str);
        SceneView {
            id: str_at(s, "/id").to_owned(),
            name: str_at(s, "/metadata/name").to_owned(),
            group_id: str_at(s, "/group/rid").to_owned(),
            colors,
            active: matches!(active, Some(a) if a != "inactive"),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let lights: BTreeMap<String, LightView> = self
            .of_type("light")
            .map(|l| self.light_view(l))
            .map(|v| (v.id.clone(), v))
            .collect();
        let by_name =
            |a: &GroupView, b: &GroupView| a.name.to_lowercase().cmp(&b.name.to_lowercase());
        let mut rooms: Vec<GroupView> = self
            .of_type("room")
            .map(|g| self.group_view(g, &lights))
            .collect();
        let mut zones: Vec<GroupView> = self
            .of_type("zone")
            .map(|g| self.group_view(g, &lights))
            .collect();
        rooms.sort_by(by_name);
        zones.sort_by(by_name);
        let mut scenes: Vec<SceneView> = self.of_type("scene").map(Self::scene_view).collect();
        scenes.sort_by_key(|s| s.name.to_lowercase());
        let mut lights: Vec<LightView> = lights.into_values().collect();
        lights.sort_by_key(|l| l.name.to_lowercase());
        Snapshot {
            rooms,
            zones,
            lights,
            scenes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;

    fn demo_home() -> Home {
        Home::from_resources(demo::resources())
    }

    fn room<'a>(s: &'a Snapshot, name: &str) -> &'a GroupView {
        s.rooms.iter().find(|r| r.name == name).unwrap()
    }

    #[test]
    fn snapshot_groups_lights_by_room() {
        let s = demo_home().snapshot();
        let living = room(&s, "Living room");
        assert_eq!(living.light_ids.len(), 3);
        assert!(living.on);
        assert!(!living.colors.is_empty());
        assert!(!s.zones.is_empty());
    }

    #[test]
    fn update_event_merges_partial_state() {
        let mut home = demo_home();
        let light = home.snapshot().lights[0].clone();
        let event = json!({"type": "update", "data": [
            {"id": light.id, "type": "light", "dimming": {"brightness": 12.5}}
        ]});
        assert!(home.apply_event(&event));
        let after = home.snapshot();
        let l = after.lights.iter().find(|l| l.id == light.id).unwrap();
        assert_eq!(l.brightness, 12.5);
        assert_eq!(l.name, light.name, "untouched fields are kept");
    }

    #[test]
    fn delete_event_removes_resource() {
        let mut home = demo_home();
        let id = home.snapshot().lights[0].id.clone();
        home.apply_event(&json!({"type": "delete", "data": [{"id": id, "type": "light"}]}));
        assert!(home.snapshot().lights.iter().all(|l| l.id != id));
    }

    #[test]
    fn simulated_group_off_turns_member_lights_off() {
        let mut home = demo_home();
        let living = room(&home.snapshot(), "Living room").clone();
        home.apply_locally(
            &Command::GroupOn {
                id: living.grouped_light_id.clone(),
                on: false,
            },
            true,
        );
        let s = home.snapshot();
        assert!(!room(&s, "Living room").on);
        assert!(
            s.lights
                .iter()
                .filter(|l| living.light_ids.contains(&l.id))
                .all(|l| !l.on)
        );
    }

    #[test]
    fn simulated_scene_recall_marks_scene_active() {
        let mut home = demo_home();
        let scene = home.snapshot().scenes[0].clone();
        home.apply_locally(
            &Command::RecallScene {
                id: scene.id.clone(),
            },
            true,
        );
        let s = home.snapshot();
        let active: Vec<_> = s
            .scenes
            .iter()
            .filter(|x| x.group_id == scene.group_id && x.active)
            .collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, scene.id);
    }

    #[test]
    fn brightness_command_also_turns_light_on() {
        let home = demo_home();
        let reqs = home.requests_for(&Command::LightBrightness {
            id: "x".into(),
            brightness: 40.0,
        });
        assert_eq!(reqs[0].body["on"]["on"], json!(true));
        assert_eq!(reqs[0].body["dimming"]["brightness"], json!(40.0));
    }

    #[test]
    fn color_command_switches_light_to_color_mode() {
        let mut home = demo_home();
        let l = home
            .snapshot()
            .lights
            .into_iter()
            .find(|l| l.supports_color && l.mode == "temperature")
            .expect("demo has a color light in temperature mode");
        home.apply_locally(
            &Command::LightColor {
                id: l.id.clone(),
                x: 0.68,
                y: 0.31,
            },
            false,
        );
        let after = home.snapshot();
        let l = after.lights.iter().find(|x| x.id == l.id).unwrap();
        assert_eq!(l.mode, "color");
    }
}
