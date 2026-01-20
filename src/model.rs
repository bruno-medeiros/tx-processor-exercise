use crate::TxProcessorError;
use fastnum::dec256;
use strum_macros::EnumString;

#[derive(Debug, Eq, PartialEq, serde::Deserialize, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

pub type ClientId = u16;
pub type TxId = u32;
pub type TxAmount = fastnum::D256;

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub tx_type: TxType,
    pub client: ClientId,
    pub tx_id: TxId,
    pub amount: Option<TxAmount>,
}

#[derive(Debug, PartialEq)]
pub struct ClientBalance {
    pub client: ClientId,
    pub held: TxAmount,
    pub available: TxAmount,
    pub locked: bool,
}

impl ClientBalance {
    pub fn new_empty(client: ClientId) -> ClientBalance {
        ClientBalance {
            client,
            available: dec256!(0.0),
            held: dec256!(0.0),
            locked: false,
        }
    }

    pub fn total(&self) -> TxAmount {
        self.available + self.held
    }

    pub fn add_funds(&mut self, amount: TxAmount) {
        self.available += amount;
    }

    pub fn remove_funds(&mut self, amount: TxAmount) -> Result<(), TxProcessorError> {
        if self.available >= amount {
            self.available -= amount;
            Ok(())
        } else {
            Err(TxProcessorError::WithdrawalError(self.available, amount))
        }
    }

    pub fn hold_funds(&mut self, amount: TxAmount) {
        self.held += amount;
        self.available -= amount;
    }

    pub fn resolve_funds(&mut self, amount: TxAmount) {
        self.held -= amount;
        self.available += amount;
    }

    pub fn chargeback_funds(&mut self, amount: TxAmount) {
        // TODO: validate held >= amount
        self.held -= amount;
        self.locked = true;
    }
}

#[test]
fn test_client_balance() {
    let mut balance = ClientBalance::new_empty(123);
    assert!(balance.locked == false);

    balance.add_funds(100.0.into());
    assert!(balance.available == 100.0.into());
    assert!(balance.total() == 100.0.into());

    balance.hold_funds(60.0.into());
    // fails:
    balance.remove_funds(60.0.into()).unwrap_err();
    // succeeds:
    balance.remove_funds(40.0.into()).unwrap();

    assert!(balance.available == 0.0.into());
    assert!(balance.total() == 60.0.into());
    assert!(balance.held == 60.0.into());
    balance.resolve_funds(60.0.into());
    assert!(balance.available == 60.0.into());
    assert!(balance.total() == 60.0.into());
    assert!(balance.held == 00.0.into());

    balance.hold_funds(20.0.into());
    balance.chargeback_funds(20.0.into());
    assert!(balance.available == 40.0.into());
    assert!(balance.total() == 40.0.into());
    assert!(balance.held == 00.0.into());
    assert!(balance.locked == true);
}
