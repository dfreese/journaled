#[test]
fn test_write() {
    let journal = journaled::raw::JournalWriter::new().expect("new failed");
    journal.check().expect("check failed");

    journal
        .send([(journaled::raw::MESSAGE, "Hello World")].into_iter())
        .expect("send failed");
}
