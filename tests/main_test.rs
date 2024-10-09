use tx_processor::process_file_and_output;

#[test]
fn main_test() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/example.csv");

    let mut output = vec![];
    process_file_and_output(file, &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("client, available, held, total, locked"));
    assert!(output.contains("\n1, 1.5, 0, 1.5, false"));
    assert!(output.contains("\n2, 2, 0, 2, false"));

}
