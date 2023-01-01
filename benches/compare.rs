fn baseline(socket: &std::os::unix::net::UnixDatagram, data: &[u8]) {
    socket
        .send_to(&data, "/run/systemd/journal/socket")
        .unwrap();
}

fn baseline_benchmark(c: &mut criterion::Criterion) {
    let socket = std::os::unix::net::UnixDatagram::unbound().unwrap();
    c.bench_function("baseline", |b| {
        b.iter(|| {
            baseline(
                &socket,
                criterion::black_box(b"PRIORITY=6\nMESSAGE=Hello World\n".as_slice()),
            )
        })
    });
}

fn basic_benchmark(c: &mut criterion::Criterion) {
    let custom = journaled::raw::JournalWriter::new().unwrap();

    c.bench_function("libsystemd", |b| {
        b.iter(|| {
            libsystemd::logging::journal_print(libsystemd::logging::Priority::Info, "Hello World")
        })
    });
    c.bench_function("journaled", |b| {
        b.iter(|| {
            custom.send(
                [
                    (journaled::raw::MESSAGE, "Hello World"),
                    (
                        journaled::raw::PRIORITY,
                        journaled::raw::Priority::Info.into(),
                    ),
                ]
                .into_iter(),
            )
        })
    });
}

#[cfg(feature = "stdlog")]
mod stdlog {
    fn into_priority(level: log::Level) -> libsystemd::logging::Priority {
        match level {
            log::Level::Error => libsystemd::logging::Priority::Error,
            log::Level::Warn => libsystemd::logging::Priority::Warning,
            log::Level::Info => libsystemd::logging::Priority::Info,
            log::Level::Debug => libsystemd::logging::Priority::Debug,
            log::Level::Trace => libsystemd::logging::Priority::Debug,
        }
    }

    struct Comparison();

    impl log::Log for Comparison {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }

            let line = record.line().map(|x| format!("{}", x));
            let msg = record.args().as_str().map_or_else(
                || format!("{}", record.args()).into(),
                std::borrow::Cow::Borrowed,
            );

            let values = [("TARGET", record.target())];
            let opt_values = [
                record.file().map(|x| ("CODE_FILE", x)),
                line.as_ref().map(|x| ("CODE_LINE", x.as_str())),
                record.module_path().map(|x| ("MODULE_PATH", x)),
            ];

            if let Err(err) = libsystemd::logging::journal_send(
                into_priority(record.level()),
                &msg,
                values.into_iter().chain(opt_values.into_iter().flatten()),
            ) {
                eprintln!("logging failed: {}", err);
            }
        }

        fn flush(&self) {}
    }

    fn logging_benchmark(c: &mut criterion::Criterion) {
        use log::Log;

        let record = log::Record::builder()
            .args(format_args!("Error!"))
            .level(log::Level::Error)
            .target("myApp")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("server"))
            .build();

        let libsystemd = Comparison();
        let custom = journaled::raw::JournalWriter::new().unwrap();

        c.bench_function("libsystemd_logger", |b| {
            b.iter(|| libsystemd.log(criterion::black_box(&record)))
        });
        c.bench_function("journaled_logger", |b| {
            b.iter(|| custom.log(criterion::black_box(&record)))
        });
    }

    criterion::criterion_group!(log_benches, logging_benchmark);
}
criterion::criterion_group!(baseline_benches, baseline_benchmark);
criterion::criterion_group!(basic_benches, basic_benchmark);

#[cfg(feature = "stdlog")]
criterion::criterion_main!(baseline_benches, basic_benches, stdlog::log_benches);

#[cfg(not(feature = "stdlog"))]
criterion::criterion_main!(baseline_benches, basic_benches,);
