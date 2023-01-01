fn as_priority(level: log::Level) -> crate::raw::Priority {
    match level {
        log::Level::Error => crate::raw::Priority::Error,
        log::Level::Warn => crate::raw::Priority::Warning,
        log::Level::Info => crate::raw::Priority::Info,
        log::Level::Debug => crate::raw::Priority::Debug,
        log::Level::Trace => crate::raw::Priority::Debug,
    }
}

const TARGET: crate::raw::Field = crate::raw::Field::unchecked("TARGET");
const MODULE_PATH: crate::raw::Field = crate::raw::Field::unchecked("MODULE_PATH");

impl log::Log for crate::raw::JournalWriter {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let line = record.line().as_ref().map(ToString::to_string);
        let msg = record.args().as_str().map_or_else(
            || record.args().to_string().into(),
            std::borrow::Cow::Borrowed,
        );

        let values = [
            as_priority(record.level()).as_value(),
            (crate::raw::MESSAGE, &msg),
            (TARGET, record.target()),
        ];
        let opt_values = [
            record.file().map(|x| (crate::raw::CODE_FILE, x)),
            line.as_ref().map(|x| (crate::raw::CODE_LINE, x.as_str())),
            record.module_path().map(|x| (MODULE_PATH, x)),
        ];

        if let Err(err) = self.send(values.into_iter().chain(opt_values.into_iter().flatten())) {
            eprintln!("logging failed: {}", err);
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    static LOGGER: once_cell::sync::OnceCell<crate::raw::JournalWriter> =
        once_cell::sync::OnceCell::new();

    let logger = LOGGER.get_or_try_init(crate::raw::JournalWriter::new)?;
    logger.check()?;
    log::set_logger(logger)?;
    log::set_max_level(log::LevelFilter::Info);

    Ok(())
}
