use crate::{
    account,
    core::OperationKind,
    error::{CreateAccountsApiError, CreateTransfersApiError},
    Account, Transfer,
};

#[derive(Debug)]
pub enum Reply {
    CreateAccounts(Result<(), CreateAccountsApiError>),
    CreateTransfers(Result<(), CreateTransfersApiError>),
    GetAccountBalances(Vec<account::Balance>),
    GetAccountTransfers(Vec<Transfer>),
    LookupAccounts(Vec<Account>),
    LookupTransfers(Vec<Transfer>),
}

impl Reply {
    pub fn copy_from_reply(operation: OperationKind, payload: &[u8]) -> Self {
        match operation {
            OperationKind::CreateAccounts => {
                let results = bytemuck::pod_collect_to_vec(payload);
                let e = CreateAccountsApiError::from_raw_results(results);
                Reply::CreateAccounts(e.map_or(Ok(()), Err))
            }
            OperationKind::CreateTransfers => {
                let results = bytemuck::pod_collect_to_vec(payload);
                let e = CreateTransfersApiError::from_raw_results(results);
                Reply::CreateTransfers(e.map_or(Ok(()), Err))
            }
            OperationKind::GetAccountBalances => {
                Reply::GetAccountBalances(bytemuck::pod_collect_to_vec(payload))
            }
            OperationKind::GetAccountTransfers => {
                Reply::GetAccountTransfers(bytemuck::pod_collect_to_vec(payload))
            }
            OperationKind::LookupAccounts => {
                Reply::LookupAccounts(bytemuck::pod_collect_to_vec(payload))
            }
            OperationKind::LookupTransfers => {
                Reply::LookupTransfers(bytemuck::pod_collect_to_vec(payload))
            }
            _ => unimplemented!("unknown operation kind"),
        }
    }

    pub fn into_create_accounts(self) -> Result<(), CreateAccountsApiError> {
        if let Reply::CreateAccounts(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected CreateAccounts but found: {self:?}")
        }
    }

    pub fn into_create_transfers(self) -> Result<(), CreateTransfersApiError> {
        if let Reply::CreateTransfers(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected CreateTransfers but found: {self:?}")
        }
    }

    pub fn into_get_account_balances(self) -> Vec<account::Balance> {
        if let Reply::GetAccountBalances(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected GetAccountBalances but found: {self:?}")
        }
    }

    pub fn into_get_account_transfers(self) -> Vec<Transfer> {
        if let Reply::GetAccountTransfers(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected GetAccountTransfers but found: {self:?}")
        }
    }

    pub fn into_lookup_accounts(self) -> Vec<Account> {
        if let Reply::LookupAccounts(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected LookupAccounts but found: {self:?}")
        }
    }

    pub fn into_lookup_transfers(self) -> Vec<Transfer> {
        if let Reply::LookupTransfers(out) = self {
            out
        } else {
            panic!("wrong reply variant, expected LookupTransfers but found: {self:?}")
        }
    }
}
