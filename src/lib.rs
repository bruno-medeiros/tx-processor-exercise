use crate::tx_processor::TxProcessor;
use csv::StringRecord;
use model::{Transaction, TxType};
use std::error::Error;

pub mod model;
pub mod tx_processor;

// Result alias to be less verbose
pub type GResult<T> = Result<T, Box<dyn Error>>;

pub fn process_file_and_output(path: &str) -> GResult<()> {
    let file = std::fs::File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);
    let mut iter = reader.records().map::<GResult<Transaction>, _>(|record| {
        let transaction = parse_csv_transaction(&record?)?;
        Ok(transaction)
    });
    let mut tx_processor = TxProcessor::new();
    tx_processor.process_input(&mut iter)?;
    // TODO: output.
    Ok(())
}

fn parse_csv_transaction(record: &StringRecord) -> GResult<Transaction> {
    // not using serde with CSV reader directly because it seems to
    // have problems parsing number with leading spaces?

    let tx_type: TxType = record[0].parse()?;
    let client: u16 = record[1].trim().parse()?;
    let tx: u32 = record[2].trim().parse()?;
    let amount: f64 = record[3].trim().parse()?;

    Ok(Transaction {
        tx_type,
        client,
        tx_id: tx,
        amount,
    })
}
