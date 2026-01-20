use crate::tx_processor::TxProcessor;
use fastnum::decimal::Context;
use futures::stream::StreamExt;
use futures::TryStreamExt;
use model::{Transaction, TxType};
use std::io;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;

pub mod model;
pub mod tx_processor;

#[derive(Debug, thiserror::Error)]
pub enum TxProcessorError {
    #[error("IoError")]
    IoError(#[from] io::Error),
    #[error("CSV parseError: {0}")]
    CSvParseError(String),
    #[error("ParseError: {0}")]
    ParseError(#[from] strum::ParseError),
    #[error("ParseIntError: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("DecimalParseError: {0}")]
    DecimalParseError(#[from] fastnum::decimal::ParseError),
    #[error("Amount missing")]
    AmountMissing,
    #[error("Not enough founds to withdraw. available={0}, requested={1}")]
    WithdrawalError(fastnum::D256, fastnum::D256),
}

pub async fn process_file_and_output<OUT: io::Write>(
    path: &str,
    stdout: &mut OUT,
) -> Result<(), TxProcessorError> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let lines = reader.lines();
    let mut line_stream = LinesStream::new(lines);

    // Skip header line if present
    let header = line_stream.next().await;
    if let Some(Ok(header)) = header {
        if !header.starts_with("type,") {
            return Err(TxProcessorError::CSvParseError(
                "header missing".to_string(),
            ));
        }
    }

    // Convert Lines to Stream and process each line
    let transaction_stream = line_stream
        .map_err(TxProcessorError::IoError)
        .and_then(|line| async move { parse_csv_transaction(line) });

    let mut tx_processor = TxProcessor::new();
    tx_processor.process_input(transaction_stream).await?;

    // Write output
    writeln!(stdout, "client, available, held, total, locked")?;
    let values = tx_processor.clients_balance.values();

    for cb in values {
        let client = cb.client;
        let (available, held, total, locked) = (cb.available, cb.held, cb.total, cb.locked);
        writeln!(stdout, "{client}, {available}, {held}, {total}, {locked}")?;
    }
    Ok(())
}

fn parse_csv_transaction(line: String) -> Result<Transaction, TxProcessorError> {
    let record: Vec<&str> = line.split(',').collect();

    let tx_type: TxType = record[0].parse()?;
    let client: u16 = record[1].trim().parse()?;
    let tx: u32 = record[2].trim().parse()?;
    let amount = record[3].trim();
    let amount: Option<fastnum::D256> = if amount.is_empty() {
        None
    } else {
        Some(fastnum::D256::from_str(amount, Context::default())?)
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
    #[tokio::test]
    async fn test_parse_csv_transaction() {
        let input = r#"type, client,tx, amount
deposit, 1, 2, 3.0
withdrawal, 4, 5, 6.0
dispute, 1, 2,
resolve, 3, 4,
chargeback, 5, 6,
"#
        .as_bytes();

        let  reader = BufReader::new(input);
        let mut stream = LinesStream::new(reader.lines());
        stream.next().await.unwrap().ok();

        let txs = stream
            .map(|res| parse_csv_transaction(res.unwrap()).unwrap())
            .collect::<Vec<Transaction>>().await;

        assert!(txs.len() == 5);

        assert_eq!(
            txs[0],
            Transaction {
                tx_type: Deposit,
                client: 1,
                tx_id: 2,
                amount: Some(3.0.into()),
            }
        );
        assert_eq!(
            txs[1],
            Transaction {
                tx_type: Withdrawal,
                client: 4,
                tx_id: 5,
                amount: Some(6.0.into()),
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
