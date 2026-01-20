use crate::model::TxDetails;
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
        let locked = cb.locked;
        // Convert to float just for printing and avoid trailing zeros
        let available = cb.available.to_f64();
        let held = cb.held.to_f64();
        let total = cb.total().to_f64();
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
        client,
        tx_id: tx,
        tx_details: match tx_type {
            TxType::Deposit => TxDetails::Deposit {
                amount: amount.ok_or(TxProcessorError::AmountMissing)?,
            },
            TxType::Withdrawal => TxDetails::Withdrawal {
                amount: amount.ok_or(TxProcessorError::AmountMissing)?,
            },
            TxType::Dispute => TxDetails::Dispute,
            TxType::Resolve => TxDetails::Resolve,
            TxType::Chargeback => TxDetails::Chargeback,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let reader = BufReader::new(input);
        let mut stream = LinesStream::new(reader.lines());
        stream.next().await.unwrap().ok();

        let txs = stream
            .map(|res| parse_csv_transaction(res.unwrap()).unwrap())
            .collect::<Vec<Transaction>>()
            .await;

        assert!(txs.len() == 5);

        assert_eq!(
            txs[0],
            Transaction {
                client: 1,
                tx_id: 2,
                tx_details: TxDetails::Deposit { amount: 3.0.into() },
            }
        );
        assert_eq!(
            txs[1],
            Transaction {
                client: 4,
                tx_id: 5,
                tx_details: TxDetails::Withdrawal { amount: 6.0.into() },
            }
        );
        assert_eq!(
            txs[2],
            Transaction {
                client: 1,
                tx_id: 2,
                tx_details: TxDetails::Dispute
            }
        );
        assert_eq!(
            txs[3],
            Transaction {
                client: 3,
                tx_id: 4,
                tx_details: TxDetails::Resolve
            }
        );
        assert_eq!(
            txs[4],
            Transaction {
                client: 5,
                tx_id: 6,
                tx_details: TxDetails::Chargeback
            }
        );
    }
}
