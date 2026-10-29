//! The QObject QML uses to read the home's state and send commands. All the
//! work happens in [`kandela_core::Service`]; this only forwards in both
//! directions.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, status)]
        #[qproperty(QString, bridge_name)]
        #[qproperty(QString, message)]
        #[qproperty(QString, home_json)]
        #[qproperty(QString, bridges_json)]
        #[qproperty(bool, discovering)]
        #[qproperty(QString, pairing_state)]
        #[qproperty(QString, pairing_message)]
        // The last command the bridge rejected; QML clears it when dismissed.
        #[qproperty(QString, error_message)]
        type HueController = super::HueControllerRust;

        #[qinvokable]
        fn discover(self: &HueController);
        #[qinvokable]
        fn pair(self: &HueController, address: &QString);
        #[qinvokable]
        fn cancel_pairing(self: &HueController);
        #[qinvokable]
        fn start_demo(self: &HueController);
        #[qinvokable]
        fn forget_bridge(self: &HueController);
        #[qinvokable]
        fn reconnect(self: &HueController);

        #[qinvokable]
        fn set_light_on(self: &HueController, id: &QString, on: bool);
        #[qinvokable]
        fn set_light_brightness(self: &HueController, id: &QString, brightness: f64);
        /// Hue in degrees, saturation in 0..1, as picked on the color wheel.
        #[qinvokable]
        fn set_light_hue_saturation(self: &HueController, id: &QString, hue: f64, saturation: f64);
        #[qinvokable]
        fn set_light_mirek(self: &HueController, id: &QString, mirek: f64);
        #[qinvokable]
        fn set_group_on(self: &HueController, grouped_light_id: &QString, on: bool);
        #[qinvokable]
        fn set_group_brightness(self: &HueController, grouped_light_id: &QString, brightness: f64);
        #[qinvokable]
        fn recall_scene(self: &HueController, id: &QString);
        #[qinvokable]
        fn set_all_on(self: &HueController, on: bool);
        /// The display color of a white light at the given mirek, as "#rrggbb".
        #[qinvokable]
        fn mirek_color(self: &HueController, mirek: f64) -> QString;
    }

    impl cxx_qt::Threading for HueController {}
    impl cxx_qt::Initialize for HueController {}
}

use core::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kandela_core::color::{self, hsv_to_rgb, rgb_to_xy};
use kandela_core::{Command, Request, Service, Update};

pub struct HueControllerRust {
    status: QString,
    bridge_name: QString,
    message: QString,
    home_json: QString,
    bridges_json: QString,
    discovering: bool,
    pairing_state: QString,
    pairing_message: QString,
    error_message: QString,
    service: Option<Service>,
}

impl Default for HueControllerRust {
    fn default() -> Self {
        Self {
            status: QString::from("connecting"),
            bridge_name: QString::default(),
            message: QString::default(),
            home_json: QString::from("{}"),
            bridges_json: QString::from("[]"),
            discovering: false,
            pairing_state: QString::from("idle"),
            pairing_message: QString::default(),
            error_message: QString::default(),
            service: None,
        }
    }
}

impl cxx_qt::Initialize for qobject::HueController {
    fn initialize(mut self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        let service = Service::spawn(move |update| {
            // Runs on the service thread; hop to the Qt thread to touch the object.
            let queued =
                qt_thread.queue(
                    move |mut obj: Pin<&mut qobject::HueController>| match update {
                        Update::Status {
                            status,
                            bridge_name,
                            message,
                        } => {
                            obj.as_mut().set_status(QString::from(status));
                            if !bridge_name.is_empty() || status == "setup" {
                                obj.as_mut().set_bridge_name(QString::from(&bridge_name));
                            }
                            obj.as_mut().set_message(QString::from(&message));
                        }
                        Update::Home(json) => obj.as_mut().set_home_json(QString::from(&json)),
                        Update::Discovering(d) => obj.as_mut().set_discovering(d),
                        Update::Bridges(json) => {
                            obj.as_mut().set_bridges_json(QString::from(&json))
                        }
                        Update::CommandFailed(message) => {
                            obj.as_mut().set_error_message(QString::from(&message));
                        }
                        Update::Pairing { state, message } => {
                            obj.as_mut().set_pairing_message(QString::from(&message));
                            obj.as_mut().set_pairing_state(QString::from(state));
                        }
                    },
                );
            if let Err(e) = queued {
                log::debug!("UI is gone, dropping update: {e:?}");
            }
        });
        let demo = std::env::args().any(|a| a == "--demo");
        service.send(if demo {
            Request::StartDemo
        } else {
            Request::Start
        });
        self.as_mut().rust_mut().service = Some(service);
    }
}

impl qobject::HueController {
    fn send(&self, request: Request) {
        if let Some(s) = &self.rust().service {
            s.send(request);
        }
    }

    fn command(&self, command: Command) {
        self.send(Request::Command(command));
    }

    pub fn discover(&self) {
        self.send(Request::Discover);
    }

    pub fn pair(&self, address: &QString) {
        self.send(Request::Pair {
            address: address.to_string(),
        });
    }

    pub fn cancel_pairing(&self) {
        self.send(Request::CancelPair);
    }

    pub fn start_demo(&self) {
        self.send(Request::StartDemo);
    }

    pub fn forget_bridge(&self) {
        self.send(Request::Forget);
    }

    pub fn reconnect(&self) {
        self.send(Request::Start);
    }

    pub fn set_light_on(&self, id: &QString, on: bool) {
        self.command(Command::LightOn {
            id: id.to_string(),
            on,
        });
    }

    pub fn set_light_brightness(&self, id: &QString, brightness: f64) {
        self.command(Command::LightBrightness {
            id: id.to_string(),
            brightness: brightness.clamp(1.0, 100.0),
        });
    }

    pub fn set_light_hue_saturation(&self, id: &QString, hue: f64, saturation: f64) {
        let (x, y) = rgb_to_xy(hsv_to_rgb(hue, saturation.clamp(0.0, 1.0), 1.0));
        self.command(Command::LightColor {
            id: id.to_string(),
            x,
            y,
        });
    }

    pub fn set_light_mirek(&self, id: &QString, mirek: f64) {
        self.command(Command::LightTemperature {
            id: id.to_string(),
            // Clamped to the light's own range in kandela-core.
            mirek: mirek.round().clamp(0.0, f64::from(u16::MAX)) as u16,
        });
    }

    pub fn set_group_on(&self, grouped_light_id: &QString, on: bool) {
        self.command(Command::GroupOn {
            id: grouped_light_id.to_string(),
            on,
        });
    }

    pub fn set_group_brightness(&self, grouped_light_id: &QString, brightness: f64) {
        self.command(Command::GroupBrightness {
            id: grouped_light_id.to_string(),
            brightness: brightness.clamp(1.0, 100.0),
        });
    }

    pub fn recall_scene(&self, id: &QString) {
        self.command(Command::RecallScene { id: id.to_string() });
    }

    pub fn set_all_on(&self, on: bool) {
        self.command(Command::AllOn { on });
    }

    pub fn mirek_color(&self, mirek: f64) -> QString {
        QString::from(&color::mirek_to_rgb(mirek).to_hex())
    }
}
