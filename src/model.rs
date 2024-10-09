use strum_macros::EnumString;
use crate::GResult;

#[derive(Debug, serde::Deserialize, EnumString)]
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

#[derive(Debug, serde::Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TxType,
    pub client: ClientId,
    pub tx_id: TxId,
    pub amount: TxAmount,
}

#[derive(Debug, PartialEq,  serde::Serialize)]
pub struct ClientBalance {
    pub client: ClientId,
    pub available: TxAmount,
    pub held: TxAmount,
    pub total: TxAmount,
    pub locked: bool,
}

impl ClientBalance {
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
}

impl ClientBalance {
    pub fn new_empty(client: ClientId) -> ClientBalance {
        ClientBalance {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }
}
