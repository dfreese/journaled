#[cfg(feature = "stdlog")]
#[test]
fn test_write() {
    journaled::log::init().expect("unable to initialize logger");

    log::info!("Info");
    log::warn!("Warn");
    log::error!("Error");
}
