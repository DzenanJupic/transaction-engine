use fixed::types::U51F13;

use crate::account::AccountId;

/// The unique identifier of a transaction
#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
pub struct TransactionId(u32);

/// The different types of transactions supported by the transaction engine
#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// A credit to the client's asset account
    Deposit,
    /// A debit to the client's asset account
    Withdrawal,
    /// A client's claim that a transaction was erroneous and should be reversed
    Dispute,
    /// A resolution to a dispute
    Resolve,
    /// The final step of a dispute and the client reversing a transaction
    Chargeback,
}

/// A transactions
///
/// Transactions are orders to the transaction engine to modify the funds and
/// the state of a clients account.
#[derive(Debug, serde::Deserialize)]
pub struct Transaction {
    #[serde(rename = "tx")]
    id: TransactionId,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    client: AccountId,
    amount: Option<U51F13>,
}

impl Transaction {
    /// The unique id of a transaction
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// The type of the transaction
    pub fn transaction_type(&self) -> TransactionType {
        self.transaction_type
    }

    /// The account id this transaction is for
    pub fn client(&self) -> AccountId {
        self.client
    }

    /// The amount
    /// Will only be populated for deposits and withdrawals
    pub fn amount(&self) -> Option<U51F13> {
        self.amount
    }
}
