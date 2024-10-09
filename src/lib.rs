use crate::tx_processor::TxProcessor;
use csv::StringRecord;
use model::{Transaction, TxType};
use std::error::Error;
use std::io;

pub mod model;
pub mod tx_processor;

// Result alias to be less verbose
pub type GResult<T> = Result<T, Box<dyn Error>>;

pub fn process_file_and_output<OUT: io::Write>(path: &str, stdout: &mut OUT) -> GResult<()> {
    let file = std::fs::File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);
    let mut iter = reader.records().map::<GResult<Transaction>, _>(|record| {
        let transaction = parse_csv_transaction(&record?)?;
        Ok(transaction)
    });
    let mut tx_processor = TxProcessor::new();
    tx_processor.process_input(&mut iter)?;

    // Write output
    write!(stdout, "client, available, held, total, locked\n")?;
    let values = tx_processor.clients_balance.values();

    for cb in values {
        let client = cb.client;
        let (available, held, total, locked) = (cb.available, cb.held, cb.total, cb.locked);
        write!(stdout, "{client}, {available}, {held}, {total}, {locked}\n")?;
    }
    Ok(())
}

fn parse_csv_transaction(record: &StringRecord) -> GResult<Transaction> {
    // not using serde with CSV reader directly because it seems to
    // have problems parsing number with leading spaces?

    let tx_type: TxType = record[0].parse()?;
    let client: u16 = record[1].trim().parse()?;
    let tx: u32 = record[2].trim().parse()?;
    let amount = record[3].trim();
    let amount: Option<f64> = if amount.is_empty() {
        None
    } else {
        Some(amount.parse()?)
    };

    Ok(Transaction {
        tx_type,
        client,
        tx_id: tx,
        amount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TxType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};

    // test serialization
    #[test]
    fn test_parse_csv_transaction() {
        let input = r#"type, client,tx, amount
deposit, 1, 2, 3.0
withdrawal, 4, 5, 6.0
dispute, 1, 2,
resolve, 3, 4,
chargeback, 5, 6,
"#
        .as_bytes();

        let mut reader = csv::Reader::from_reader(input);
        let iter = reader.records().map::<Transaction, _>(|record| {
            let transaction = parse_csv_transaction(&record.unwrap()).unwrap();
            transaction
        });
        let txs = iter.collect::<Vec<Transaction>>();

        assert!(txs.len() == 5);

        assert_eq!(
            txs[0],
            Transaction {
                tx_type: Deposit,
                client: 1,
                tx_id: 2,
                amount: Some(3.0),
            }
        );
        assert_eq!(
            txs[1],
            Transaction {
                tx_type: Withdrawal,
                client: 4,
                tx_id: 5,
                amount: Some(6.0),
            }
        );
        assert_eq!(
            txs[2],
            Transaction {
                tx_type: Dispute,
                client: 1,
                tx_id: 2,
                amount: None,
            }
        );
        assert_eq!(
            txs[3],
            Transaction {
                tx_type: Resolve,
                client: 3,
                tx_id: 4,
                amount: None,
            }
        );
        assert_eq!(
            txs[4],
            Transaction {
                tx_type: Chargeback,
                client: 5,
                tx_id: 6,
                amount: None,
            }
        );
    }
}
