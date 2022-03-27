use crate::Amount;

/// Possible errors to occur during account operations
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("The account is locked")]
    Locked,
    #[error("The account does not hold enough available funds")]
    InsufficientFunds,
}

/// The unique identifier of an account
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct AccountId(u16);

/// A user account
///
/// The user account consists of two sub accounts:
/// 1. The available funds:
///    Available funds are funds that can withdrawn,
///    or used for other purposes.
/// 2. Held back funds:
///    Funds that are held back are used to cover
///    possible future claims, like chargebacks. The
///    client cannot use these funds until they are
///    either charged back, or freed.
#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Account {
    #[serde(rename = "client")]
    id: AccountId,
    available: Amount,
    held: Amount,
    locked: bool,
}

impl Account {
    /// Creates a new empty user account with the specified id
    pub fn new(id: AccountId) -> Self {
        Self {
            id,
            available: Amount::from_num(0),
            held: Amount::from_num(0),
            locked: false,
        }
    }

    /// The identifier of the account
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// The total funds in the account
    ///
    /// The total funds are the sum of available and held back funds.
    /// See [`Account`] for more info.
    pub fn total(&self) -> Amount {
        self.available + self.held
    }

    /// Deposits the specified amount on the account
    pub fn deposit(&mut self, amount: Amount) -> Result<(), AccountError> {
        self.check_locked()?;
        self.available += amount;

        Ok(())
    }

    /// Withdrawals the specified amount from the account
    pub fn withdrawal(&mut self, amount: Amount) -> Result<(), AccountError> {
        self.check_locked()?;
        self.available = self.available
            .checked_sub(amount)
            .ok_or(AccountError::InsufficientFunds)?;

        Ok(())
    }

    /// Holds the specified amount back from future withdrawals
    /// *To release the funds again, you can use [`Account::set_free`]*
    pub fn hold_back(&mut self, amount: Amount) -> Result<(), AccountError> {
        self.check_locked()?;
        self.available = self.available
            .checked_sub(amount)
            .ok_or(AccountError::InsufficientFunds)?;
        self.held += amount;

        Ok(())
    }

    /// Releases the specified amount for future withdrawals
    /// *To  hold funds back, you can use [`Account::withdrawal`]*
    pub fn set_free(&mut self, amount: Amount) -> Result<(), AccountError> {
        self.check_locked()?;
        self.held = self.held
            .checked_sub(amount)
            .ok_or(AccountError::InsufficientFunds)?;
        self.available += amount;

        Ok(())
    }

    /// Reveres a transaction and returns held back funds
    ///
    /// ### Important
    /// This will leave the account locked. After the account is locked, it can no
    /// longer be used for any purpose until it is unlocked again.
    pub fn charge_back(&mut self, amount: Amount) -> Result<(), AccountError> {
        self.check_locked()?;
        self.held = self.held
            .checked_sub(amount)
            .ok_or(AccountError::InsufficientFunds)?;
        self.locked = true;

        Ok(())
    }

    fn check_locked(&self) -> Result<(), AccountError> {
        match self.locked {
            false => Ok(()),
            true => Err(AccountError::Locked),
        }
    }
}

impl serde::Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        use serde::ser::SerializeStruct;
        let mut map = serializer.serialize_struct("Account", 5)?;

        map.serialize_field("client", &self.id)?;
        map.serialize_field("available", &self.available)?;
        map.serialize_field("held", &self.held)?;
        map.serialize_field("total", &self.total())?;
        map.serialize_field("locked", &self.locked)?;

        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_increases_available() {
        let mut account = Account::new(AccountId(0));

        assert_eq!(account.available, Amount::from_num(0));
        assert_eq!(account.held, Amount::from_num(0));

        account.deposit(Amount::from_num(100)).unwrap();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn deposit_on_locked_fails() {
        let mut account = Account::new(AccountId(0));
        account.locked = true;

        assert_eq!(account.available, Amount::from_num(0));
        assert_eq!(account.held, Amount::from_num(0));

        account.deposit(Amount::from_num(100)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(0));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn withdrawal_decreases_available() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.withdrawal(Amount::from_num(100)).unwrap();

        assert_eq!(account.available, Amount::from_num(0));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn withdrawal_underflow_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.withdrawal(Amount::from_num(200)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn withdrawal_on_locked_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);
        account.locked = true;

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.withdrawal(Amount::from_num(100)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn hold_back_increases_held() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.hold_back(Amount::from_num(50)).unwrap();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));
    }

    #[test]
    fn hold_back_underflow_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.hold_back(Amount::from_num(200)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn hold_back_on_locked_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(100);
        account.locked = true;

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));

        account.hold_back(Amount::from_num(50)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn set_free_decreases_held() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.set_free(Amount::from_num(50)).unwrap();

        assert_eq!(account.available, Amount::from_num(100));
        assert_eq!(account.held, Amount::from_num(0));
    }

    #[test]
    fn set_free_underflow_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.set_free(Amount::from_num(100)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));
    }

    #[test]
    fn set_free_on_locked_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);
        account.locked = true;

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.set_free(Amount::from_num(50)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));
    }

    #[test]
    fn charge_back_decreases_available() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.charge_back(Amount::from_num(50)).unwrap();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(0));
        assert!(account.locked);
    }

    #[test]
    fn charge_back_underflow_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.charge_back(Amount::from_num(100)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));
        assert!(!account.locked);
    }

    #[test]
    fn charge_back_on_locked_fails() {
        let mut account = Account::new(AccountId(0));
        account.available = Amount::from_num(50);
        account.held = Amount::from_num(50);
        account.locked = true;

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));

        account.charge_back(Amount::from_num(50)).unwrap_err();

        assert_eq!(account.available, Amount::from_num(50));
        assert_eq!(account.held, Amount::from_num(50));
        assert!(account.locked);
    }
}
