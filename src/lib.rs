pub use self::{
    account::{Account, AccountError, AccountId},
    engine::{TransactionEngine, TransactionError},
    transaction::{Transaction, TransactionId, TransactionType},
};

mod account;
mod engine;
mod transaction;

/// An amount of money with a maximal precision of at least four decimals.
///
/// The maximum amount that can be represented is [`fixed::types::U50F14::MAX`].
pub type Amount = fixed::types::U50F14;
