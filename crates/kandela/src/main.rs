mod app;
mod controller;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let has = |flag: &str| std::env::args().any(|a| a == flag);
    if has("--help") || has("-h") {
        println!(
            "Usage: kandela [--hidden] [--demo]\n\n  \
             --hidden  Start in the system tray without showing the window\n  \
             --demo    Show a made-up home instead of connecting to a bridge"
        );
        return;
    }
    // The demo and screenshot runs are separate from the everyday instance.
    let single_instance = !has("--demo") && std::env::var_os("KANDELA_SCREENSHOT").is_none();
    kandela_core::config::migrate_legacy();
    cxx_qt::init_qml_module!("io.github.pcmciakai.kandela");
    std::process::exit(app::run_app(has("--hidden"), single_instance));
}
