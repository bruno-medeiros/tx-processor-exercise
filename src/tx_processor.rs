use crate::model::{ClientBalance, ClientId, Transaction, TxAmount, TxId, TxType};
use crate::GResult;
use std::collections::HashMap;

pub struct TxProcessor {
    referenced_transactions: HashMap<TxId, TxAmount>,
    clients_balance: HashMap<ClientId, ClientBalance>,
}

impl TxProcessor {
    pub fn new() -> TxProcessor {
        Self {
            referenced_transactions: HashMap::new(),
            clients_balance: HashMap::new(),
        }
    }

    pub fn process_input<ITER: Iterator<Item = GResult<Transaction>>>(
        &mut self,
        tx_iter: ITER,
    ) -> GResult<&HashMap<ClientId, ClientBalance>> {
        for tx in tx_iter {
            let tx = tx?;

            let client_entry = self
                .clients_balance
                .entry(tx.client)
                .or_insert_with(|| ClientBalance::new_empty(tx.client));

            match tx.tx_type {
                TxType::Deposit => {
                    client_entry.add_funds(tx.amount);
                }
                TxType::Withdrawal => {
                    client_entry.remove_funds(tx.amount).unwrap_or_else(|_err|{
                        // log error somewhere
                    });
                }
                TxType::Dispute | TxType::Resolve | TxType::Chargeback => {
                    self.referenced_transactions.insert(tx.tx_id, tx.amount);
                }
            }
            match tx.tx_type {
                TxType::Dispute | TxType::Resolve | TxType::Chargeback => {
                    self.referenced_transactions.insert(tx.tx_id, tx.amount);
                }
                _ => {}
            }
        }

        Ok(&self.clients_balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Some helper functions:

    fn deposit(client: ClientId, tx_id: TxId, amount: TxAmount) -> Transaction {
        Transaction {
            tx_type: TxType::Deposit,
            client,
            tx_id,
            amount,
        }
    }
    fn withdrawal(client: ClientId, tx_id: TxId, amount: TxAmount) -> Transaction {
        Transaction {
            tx_type: TxType::Withdrawal,
            client,
            tx_id,
            amount,
        }
    }
    fn process_tx(tx_processor: &mut TxProcessor, transaction: Transaction) -> GResult<()> {
        tx_processor.process_input(vec![transaction].into_iter().map(|tx| Ok(tx)))?;
        Ok(())
    }

    #[test]
    fn test_deposit() -> GResult<()> {
        let mut tx_processor = TxProcessor::new();
        assert!(tx_processor.clients_balance.is_empty());

        // Test a single deposit.
        process_tx(&mut tx_processor, deposit(1, 1, 100.0))?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        let mut expected_balance = ClientBalance {
            client: 1,
            total: 100.0,
            held: 0.0,
            available: 100.0,
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        // Test a second deposit.
        process_tx(&mut tx_processor, deposit(1, 2, 50.0))?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        expected_balance.total = 150.0;
        expected_balance.available = 150.0;
        assert_eq!(c1_balance, &expected_balance);

        // Test another deposit with different client.
        let client = 2;
        process_tx(&mut tx_processor, deposit(client, 3, 50.0))?;

        let c1_balance = tx_processor.clients_balance.get(&client).unwrap();
        let expected_balance = ClientBalance {
            client,
            total: 50.0,
            held: 0.0,
            available: 50.0,
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        Ok(())
    }

    #[test]
    fn test_withdrawal() -> GResult<()> {
        let mut tx_processor = TxProcessor::new();

        process_tx(&mut tx_processor, deposit(1, 1, 1000.0))?;

        // Test a withdrawal.
        process_tx(&mut tx_processor, withdrawal(1, 2, 600.0))?;
        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        let mut expected_balance = ClientBalance {
            client: 1,
            total: 400.0,
            held: 0.0,
            available: 400.0,
            locked: false,
        };
        assert_eq!(c1_balance, &expected_balance);

        // Test a second withdrawal with not enough funds.
        process_tx(&mut tx_processor, withdrawal(1, 3, 600.0))?;

        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        // Expect balance doesn't change
        assert_eq!(c1_balance, &expected_balance);

        // Test a 3rd withdrawal
        process_tx(&mut tx_processor, withdrawal(1, 4, 400.0))?;
        let c1_balance = tx_processor.clients_balance.get(&1).unwrap();
        expected_balance.total = 0.0;
        expected_balance.available = 0.0;
        assert_eq!(c1_balance, &expected_balance);

        Ok(())
    }
}
