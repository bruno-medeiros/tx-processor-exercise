use crate::model::{ClientBalance, ClientId, Transaction, TxAmount, TxId, TxType};
use std::collections::{HashMap, HashSet};
use futures::{Stream, StreamExt};
use tokio::pin;
use crate::TxProcessorError;

#[derive(Debug, Default)]
pub struct TxProcessor {
    pub account_transactions: HashMap<TxId, TxAmount>,
    pub disputed_transactions: HashSet<TxId>,
    pub clients_balance: HashMap<ClientId, ClientBalance>,
}

impl TxProcessor {
    pub fn new() -> TxProcessor {
        Self::default()
    }

    pub async fn process_input(
        &mut self,
         tx_iter: impl Stream<Item = Result<Transaction, TxProcessorError>> + Send + 'static,
    ) -> Result<&HashMap<ClientId, ClientBalance>, TxProcessorError> {
        pin!(tx_iter);
        while let Some(tx_res) = tx_iter.next().await {
            let tx = tx_res?;

            let client_entry = self
                .clients_balance
                .entry(tx.client)
                .or_insert_with(|| ClientBalance::new_empty(tx.client));

            match tx.tx_type {
                TxType::Deposit => {
                    let amount = tx.amount.ok_or(TxProcessorError::AmountMissing)?;
                    client_entry.add_funds(amount);
                }
                TxType::Withdrawal => {
                    let amount = tx.amount.ok_or(TxProcessorError::AmountMissing)?;
                    client_entry.remove_funds(amount).unwrap_or(
                        // withdrawal denied due to no funds
                        (),
                    );
                }
                TxType::Dispute => {
                    if let Some(amount) = self.account_transactions.get(&tx.tx_id) {
                        if self.disputed_transactions.contains(&tx.tx_id) {
                            // Already disputed, ignore
                        } else {
                            self.disputed_transactions.insert(tx.tx_id);
                            client_entry.hold_funds(*amount);
                        }
                    }
                }
                TxType::Resolve => {
                    if let Some(amount) = self.account_transactions.get(&tx.tx_id) {
                        if self.disputed_transactions.contains(&tx.tx_id) {
                            client_entry.resolve_funds(*amount);
                            // self.disputed_transactions.remove(&tx.tx_id);
                        } else {
                            // Not disputed, ignore
                        }
                    }
                }
                TxType::Chargeback => {
                    // Check tx_id is under dispute
                    if let Some(amount) = self.account_transactions.get(&tx.tx_id) {
                        if self.disputed_transactions.contains(&tx.tx_id) {
                            client_entry.chargeback_funds(*amount);
                            // self.disputed_transactions.remove(&tx.tx_id);
                        } else {
                            // Not disputed, ignore
                        }
                    }
                }
            }

            // Store Deposits in account_transactions record for referencing during Disputes/Resolves.
            // The spec doesn't clarify if it's Deposits or Withdrawals that can be disputed,
            // but from the description:
            // "This means that the clients available funds should decrease by the amount disputed,
            // their held funds should increase by the amount disputed"
            // only Deposits make sense for disputes.
            // (Maybe Withdrawals too if amount was inverted)
            #[allow(clippy::single_match)]
            match tx.tx_type {
                TxType::Deposit => {
                    let amount = tx.amount.ok_or(TxProcessorError::AmountMissing)?;
                    self.account_transactions.insert(tx.tx_id, amount);
                }
                _ => {}
            }
        }

        Ok(&self.clients_balance)
    }
}

#[cfg(test)]
mod tests {
    use futures::stream;
    use super::*;

    // Some helper functions:

    fn deposit(client: ClientId, tx_id: TxId, amount: TxAmount) -> Transaction {
        Transaction {
            tx_type: TxType::Deposit,
            client,
            tx_id,
            amount: Some(amount),
        }
    }
    fn withdrawal(client: ClientId, tx_id: TxId, amount: TxAmount) -> Transaction {
        Transaction {
            tx_type: TxType::Withdrawal,
            client,
            tx_id,
            amount: Some(amount),
        }
    }
    async fn process_tx(tx_processor: &mut TxProcessor, transaction: Transaction) -> Result<(), TxProcessorError> {
        let stream = stream::iter(vec![transaction]).map(Ok);
        tx_processor.process_input(stream).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_deposit() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();
        assert!(tx_processor.clients_balance.is_empty());

        // Test a single deposit.
        process_tx(&mut tx_processor, deposit(1, 1, 100.0.into())).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        let mut expected_balance = ClientBalance {
            client: 1,
            held: 0.0.into(),
            available: 100.0.into(),
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        // Test a second deposit.
        process_tx(&mut tx_processor, deposit(1, 2, 50.0.into())).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        expected_balance.available = 150.0.into();
        assert_eq!(c1_balance, &expected_balance);

        // Test another deposit with different client.
        let client = 2;
        process_tx(&mut tx_processor, deposit(client, 3, 50.0.into())).await?;

        let c1_balance = tx_processor.clients_balance.get(&client).unwrap();
        let expected_balance = ClientBalance {
            client,
            held: 0.0.into(),
            available: 50.0.into(),
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        Ok(())
    }

    #[tokio::test]
    async fn test_withdrawal() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 1000.0.into())).await?;

        // Test a withdrawal.
        process_tx(&mut tx_processor, withdrawal(1, 2, 600.0.into())).await?;
        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        let mut expected_balance = ClientBalance {
            client: 1,
            held: 0.0.into(),
            available: 400.0.into(),
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        // Test a second withdrawal with not enough funds.
        process_tx(&mut tx_processor, withdrawal(1, 3, 600.0.into())).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        // Expect balance doesn't change
        assert_eq!(c1_balance, &expected_balance);

        // Test a 3rd withdrawal
        process_tx(&mut tx_processor, withdrawal(1, 4, 400.0.into())).await?;
        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        expected_balance.available = 0.0.into();
        assert_eq!(c1_balance, &expected_balance);

        Ok(())
    }

    fn dispute(tx_type: TxType, client: ClientId, tx_id: TxId) -> Transaction {
        Transaction {
            tx_type,
            client,
            tx_id,
            amount: None,
        }
    }

    #[tokio::test]
    async fn test_error_references() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 1000.0.into())).await?;
        process_tx(&mut tx_processor, deposit(1, 2, 500.0.into())).await?;

        // Test bad references.
        process_tx(&mut tx_processor, dispute(TxType::Dispute, 1, 666)).await?;
        process_tx(&mut tx_processor, dispute(TxType::Resolve, 1, 666)).await?;
        process_tx(&mut tx_processor, dispute(TxType::Chargeback, 1, 666)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: 0.0.into(),
                available: 1500.0.into(),
                locked: false,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_dispute_resolve() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 1000.0.into())).await?;
        process_tx(&mut tx_processor, deposit(1, 2, 500.0.into())).await?;

        // Test a dispute.
        process_tx(&mut tx_processor, dispute(TxType::Dispute, 1, 2)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: 500.0.into(),
                available: (1500.0 - 500.0).into(),
                locked: false,
            }
        );

        // Test a resolve.
        process_tx(&mut tx_processor, dispute(TxType::Resolve, 1, 2)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: 0.0.into(),
                available: 1500.0.into(),
                locked: false,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_dispute_resolve_multiple() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 50.0.into())).await?;
        process_tx(&mut tx_processor, deposit(1, 2, 60.0.into())).await?;
        process_tx(&mut tx_processor, deposit(1, 3, 80.0.into())).await?;

        // Test two pending disputes.
        process_tx(&mut tx_processor, dispute(TxType::Dispute, 1, 2)).await?;
        process_tx(&mut tx_processor, dispute(TxType::Dispute, 1, 3)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: (60.0 + 80.0).into(),
                available: 50.0.into(),
                locked: false,
            }
        );

        // Test a resolve.
        process_tx(&mut tx_processor, dispute(TxType::Resolve, 1, 2)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: 80.0.into(),
                available: (50.0 + 60.0).into(),
                locked: false,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_chargeback() -> Result<(), TxProcessorError> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 1000.0.into())).await?;
        process_tx(&mut tx_processor, deposit(1, 2, 500.0.into())).await?;

        process_tx(&mut tx_processor, dispute(TxType::Dispute, 1, 2)).await?;

        // Test chargeback
        process_tx(&mut tx_processor, dispute(TxType::Chargeback, 1, 2)).await?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        assert_eq!(
            c1_balance,
            &ClientBalance {
                client: 1,
                held: 0.0.into(),
                available: 1000.0.into(),
                locked: true,
            }
        );

        Ok(())
    }
}
