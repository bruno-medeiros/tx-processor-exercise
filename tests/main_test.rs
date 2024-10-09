use tx_processor::process_file_and_output;

#[test]
fn main_test() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/example.csv");
    process_file_and_output(file).unwrap();
}
