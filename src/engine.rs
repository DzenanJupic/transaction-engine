use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

use crate::{Account, AccountError, AccountId, Transaction, TransactionId, TransactionType};

/// Possible errors to occur during the processing of a transaction
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error(transparent)]
    Account(#[from] AccountError),
    #[error("The referenced transaction was not found")]
    TransactionNotFound,
    #[error("The transaction is missing an amount")]
    TransactionAmountNotSpecified,
    #[error("There's already a dispute for this transaction")]
    DuplicateDispute,
    #[error("There's no dispute for this transaction to resolve")]
    UnknownDispute,
    #[error("There's already a transaction with the same id")]
    DuplicateTransaction,
    #[error("The transaction is not of type deposit and cannot be disputed")]
    ImpossibleDispute,
}

/// The central transaction engine used for processing all transactions
///
/// This will automatically create use accounts on the fly, in case transactions
/// reference new or unknown user accounts.
#[derive(Debug, Default)]
pub struct TransactionEngine {
    /// A map of all user accounts
    accounts: HashMap<AccountId, Account>,
    /// A map of all deposit and withdrawal transactions
    /// Other types of transactions cannot be referenced, and therefore don't have to be saved
    transactions: HashMap<TransactionId, Transaction>,
    /// A set of all currently disputed transactions
    disputes: HashSet<TransactionId>,
}

impl TransactionEngine {
    /// Creates a new, empty transaction engine
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
            disputes: HashSet::new(),
        }
    }

    /// The map of all current accounts
    pub fn accounts(&self) -> &HashMap<AccountId, Account> {
        &self.accounts
    }

    /// Processes one transaction and applies possible effects to user accounts
    pub fn handle_transaction(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let transaction_id = transaction.id();
        let transaction_type = transaction.transaction_type();
        self.save_transaction(transaction)?;

        let transaction = self.transactions
            .get(&transaction_id)
            .ok_or(TransactionError::TransactionNotFound)?;
        let amount = transaction
            .amount()
            .ok_or(TransactionError::TransactionAmountNotSpecified)?;
        let account = self.accounts
            .entry(transaction.client())
            .or_insert_with(|| Account::new(transaction.client()));

        match transaction_type {
            TransactionType::Deposit => account.deposit(amount)?,
            TransactionType::Withdrawal => account.withdrawal(amount)?,
            // the specs state
            // > A dispute represents a client's claim that a transaction was erroneous and should be reversed.
            // [...]. This means that the clients available funds should decrease by the amount disputed, their
            // held funds should increase by the amount disputed, while their total funds should remain the same.
            //
            // Since the specs don't say anything about disputing withdrawals / increasing funds, disputes
            // are, for now, only allowed for deposits.
            TransactionType::Dispute if transaction.transaction_type() != TransactionType::Deposit => {
                return Err(TransactionError::ImpossibleDispute);
            }
            TransactionType::Dispute => {
                self.disputes
                    .insert(transaction.id())
                    .then(|| ())
                    .ok_or(TransactionError::DuplicateDispute)?;
                account.hold_back(amount)?;
            },
            TransactionType::Resolve => {
                self.disputes
                    .remove(&transaction.id())
                    .then(|| ())
                    .ok_or(TransactionError::UnknownDispute)?;
                account.set_free(amount)?;
            },
            TransactionType::Chargeback => {
                self.disputes
                    .remove(&transaction.id())
                    .then(|| ())
                    .ok_or(TransactionError::UnknownDispute)?;
                account.charge_back(amount)?;

                let id = transaction.id();
                self.transactions.remove(&id);
            },
        }

        Ok(())
    }

    fn save_transaction(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        match transaction.transaction_type() {
            TransactionType::Deposit | TransactionType::Withdrawal => {},
            // we don't have to save other transaction types here, since they cannot
            // be referenced later on
            _ => return Ok(())
        }

        match self.transactions.entry(transaction.id()) {
            Entry::Vacant(v) => {
                v.insert(transaction);
                Ok(())
            }
            Entry::Occupied(_) => Err(TransactionError::DuplicateTransaction),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    macro_rules! engine_test {
        (
            $name:ident
            $transactions:literal
            $solution:literal
        ) => {
            #[test]
            fn $name() {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .trim(csv::Trim::All)
                    .from_reader($transactions.as_bytes());
                let mut engine = TransactionEngine::new();

                for transaction in reader.deserialize() {
                    let _ = engine.handle_transaction(transaction.unwrap());
                }

                let mut accounts = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .trim(csv::Trim::All)
                    .from_reader($solution.as_bytes());
                let accounts = accounts
                    .deserialize::<Account>()
                    .map(Result::unwrap)
                    .map(|account| (account.id(), account))
                    .collect::<HashMap<_, _>>();
                assert_eq!(
                    engine.accounts(),
                    &accounts,
                );
            }
        };
    }

    engine_test!(pdf_example
        r#"type, client, tx, amount
           deposit,   1,  1,    1.0
           deposit,   2,  2,    2.0
           deposit,   1,  3,    2.0
           withdrawal,1,  4,    1.5
           withdrawal,2,  5,    3.0"#
        r#"client,available,held,total,locked
                1,      1.5,   0,  1.5, false
                2,        2,   0,    2, false"#
    );

    engine_test!(deposit
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20"#
        r#"client,available,held,total,locked
                1,       70,   0,   70, false"#
    );
    engine_test!(duplicate_transaction
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  1,     20"#
        r#"client,available,held,total,locked
                1,       50,   0,   50, false"#
    );
    engine_test!(withdrawal
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           withdrawal,1,  3,     30
           withdrawal,1,  4,     10"#
        r#"client,available,held,total,locked
                1,       30,   0,   30, false"#
    );
    engine_test!(withdrawal_underflow
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           withdrawal,1,  3,     80"#
        r#"client,available,held,total,locked
                1,       70,   0,   70, false"#
    );
    engine_test!(dispute
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,     "#
        r#"client,available,held,total,locked
                1,       20,  50,   70, false"#
    );
    engine_test!(duplicate_dispute
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           dispute,   1,  1,       "#
        r#"client,available,held,total,locked
                1,       20,  50,   70, false"#
    );
    engine_test!(dispute_underflow
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           withdrawal,1,  3,     60
           dispute,   1,  1,       "#
        r#"client,available,held,total,locked
                1,       10,   0,   10, false"#
    );
    engine_test!(dispute_unknown
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1, 42,       "#
        r#"client,available,held,total,locked
                1,       70,   0,   70, false"#
    );
    engine_test!(resolve
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           resolve,   1,  1,       "#
        r#"client,available,held,total,locked
                1,       70,   0,   70, false"#
    );
    engine_test!(resolve_unknown
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           resolve,   1, 42,       "#
        r#"client,available,held,total,locked
                1,       20,  50,   70, false"#
    );
    engine_test!(duplicate_resolve
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           resolve,   1,  1,
           resolve,   1,  1,       "#
        r#"client,available,held,total,locked
                1,       70,  0,   70, false"#
    );
    engine_test!(chargeback
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           chargeback,1,  1,       "#
        r#"client,available,held,total,locked
                1,       20,  0,   20, true"#
    );
    engine_test!(duplicate_chargeback
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           chargeback,1,  1,
           dispute,   1,  1,
           chargeback,1,  1,       "#
        r#"client,available,held,total,locked
                1,       20,  0,   20, true"#
    );
    engine_test!(deposit_after_chargeback
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           chargeback,1,  1,
           deposit,   1,  3,     10"#
        r#"client,available,held,total,locked
                1,       20,  0,   20, true"#
    );
    engine_test!(withdrawal_after_chargeback
        r#"type, client, tx, amount
           deposit,   1,  1,     50
           deposit,   1,  2,     20
           dispute,   1,  1,
           chargeback,1,  1,
           withdrawal,1,  3,     10"#
        r#"client,available,held,total,locked
                1,       20,  0,   20, true"#
    );
}
