use strum_macros::EnumString;
use crate::GResult;

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
pub type TxAmount = f64;

#[derive(Debug, PartialEq,  serde::Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TxType,
    pub client: ClientId,
    pub tx_id: TxId,
    pub amount: Option<TxAmount>,
}

#[derive(Debug, PartialEq,  serde::Serialize)]
pub struct ClientBalance {
    pub client: ClientId,
    pub total: TxAmount,
    pub held: TxAmount,
    pub available: TxAmount,
    pub locked: bool,
}

impl ClientBalance {
    pub fn new_empty(client: ClientId) -> ClientBalance {
        ClientBalance {
            client,
            total: 0.0,
            available: 0.0,
            held: 0.0,
            locked: false,
        }
    }

    pub fn add_funds(&mut self, amount: TxAmount) {
        self.available += amount;
        self.total += amount;
    }

    pub fn remove_funds(&mut self, amount: TxAmount) -> GResult<()> {
        if self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            Ok(())
        } else {
            Err("Not enough founds to withdraw".into())
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
        self.total -=amount;
        self.locked = true;
    }
}

#[test]
fn test_client_balance() {
    let mut balance = ClientBalance::new_empty(123);
    assert!(balance.locked == false);

    balance.add_funds(100.0);
    assert!(balance.available == 100.0);
    assert!(balance.total == 100.0);

    balance.hold_funds(60.0);
    // fails:
    balance.remove_funds(60.0).unwrap_err();
    // succeeds:
    balance.remove_funds(40.0).unwrap();

    assert!(balance.available == 0.0);
    assert!(balance.total == 60.0);
    assert!(balance.held == 60.0);
    balance.resolve_funds(60.0);
    assert!(balance.available == 60.0);
    assert!(balance.total == 60.0);
    assert!(balance.held == 00.0);

    balance.hold_funds(20.0);
    balance.chargeback_funds(20.0);
    assert!(balance.available == 40.0);
    assert!(balance.total == 40.0);
    assert!(balance.held == 00.0);
    assert!(balance.locked == true);
}