use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    let qml_files = [
        "qml/Main.qml",
        "qml/SetupPage.qml",
        "qml/HomePage.qml",
        "qml/GroupPage.qml",
        "qml/ColorPage.qml",
        "qml/GroupCard.qml",
        "qml/SceneTile.qml",
        "qml/LightTile.qml",
        "qml/LightChip.qml",
        "qml/ColorWheel.qml",
        "qml/TemperatureBar.qml",
        "qml/Archetypes.qml",
        "qml/Tray.qml",
    ];
    let icons = [
        "qml/icons/kandela.svg",
        "qml/icons/tray-on.svg",
        "qml/icons/tray-off.svg",
    ];
    for f in qml_files.iter().chain(icons.iter()) {
        println!("cargo:rerun-if-changed={f}");
    }
    let builder = CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.pcmciakai.kandela").qml_files(qml_files),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("Widgets")
    .qt_module("Network")
    .qrc_resources(icons)
    .files(["src/controller.rs", "src/app.rs"])
    .include_dir("cpp")
    .cpp_file("cpp/app.cpp");
    builder.build();
}
