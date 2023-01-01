fn as_priority(level: slog::Level) -> crate::raw::Priority {
    match level {
        slog::Level::Critical => crate::raw::Priority::Critical,
        slog::Level::Error => crate::raw::Priority::Error,
        slog::Level::Warning => crate::raw::Priority::Warning,
        slog::Level::Info => crate::raw::Priority::Info,
        slog::Level::Debug => crate::raw::Priority::Debug,
        slog::Level::Trace => crate::raw::Priority::Debug,
    }
}

const MODULE_PATH: crate::raw::Field = crate::raw::Field::unchecked("MODULE_PATH");

#[derive(Debug, Default)]
struct Serializer {
    fields: Vec<(crate::raw::OwnedField, String)>,
}

impl slog::Serializer for Serializer {
    fn emit_arguments(&mut self, key: slog::Key, val: &std::fmt::Arguments<'_>) -> slog::Result {
        if let Some(field) = crate::raw::OwnedField::sanitize(key) {
            self.fields.push((field, format!("{}", val)));
        }
        Ok(())
    }
}

impl slog::Drain for crate::raw::JournalWriter {
    type Ok = ();
    type Err = std::io::Error;

    fn log(
        &self,
        record: &slog::Record<'_>,
        logger_kv: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        if !self.is_enabled(record.level()) {
            return Ok(());
        }

        let line = record.line().to_string();
        let msg = record.msg().as_str().map_or_else(
            || record.msg().to_string().into(),
            std::borrow::Cow::Borrowed,
        );

        let values = [
            (crate::raw::CODE_FILE, record.file()),
            (crate::raw::CODE_LINE, &line),
            (crate::raw::MESSAGE, &msg),
            as_priority(record.level()).as_value(),
            (MODULE_PATH, record.module()),
        ];

        let serializer = {
            use slog::KV;

            let mut serializer = Serializer::default();
            logger_kv.serialize(record, &mut serializer)?;
            record.kv().serialize(record, &mut serializer)?;
            serializer
        };

        self.send(
            values.into_iter().chain(
                serializer
                    .fields
                    .iter()
                    .map(|(k, v)| (k.into(), v.as_str())),
            ),
        )
    }
}
