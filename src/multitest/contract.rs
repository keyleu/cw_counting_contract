use cosmwasm_std::{Addr, Coin, StdResult, Empty, StdError};
use cw_multi_test::{App, Executor, ContractWrapper};

use crate::{
    error::ContractError,
    msg::{ExecMsg, InstantiateMsg, QueryMsg, ValueResp, Parent}, execute, instantiate, query, migrate,
};

pub struct CountingContract(Addr);

impl CountingContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn store_code(app: &mut App) -> u64 {
        let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
        app.store_code(Box::new(contract))
    }
    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        code_id: u64,
        sender: &Addr,
        counter: u64,
        minimal_donation: Coin,
        label: &str,
        admin: Option<&Addr>,
        parent: Option<Parent>
    ) -> StdResult<CountingContract> {
        app.instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                counter,
                minimal_donation,
                parent,
            },
            &[],
            label,
            admin.map(Addr::to_string),
        )
        .map_err(|err| err.downcast().unwrap())
        .map(CountingContract)
    }

    #[track_caller]
    pub fn donate(
        &self,
        app: &mut App,
        sender: &Addr,
        funds: &[Coin],
    ) -> Result<(), ContractError> {
        app.execute_contract(sender.clone(), self.0.clone(), &ExecMsg::Donate {}, funds)
            .map_err(|err| err.downcast::<ContractError>().unwrap())?;
        Ok(())
    }

    #[track_caller]
    pub fn withdraw(&self, app: &mut App, sender: &Addr) -> Result<(), ContractError> {
        app.execute_contract(sender.clone(), self.0.clone(), &ExecMsg::Withdraw {}, &[])
            .map_err(|err| err.downcast::<ContractError>().unwrap())?;
        Ok(())
    }

    #[track_caller]
    pub fn reset(&self, app: &mut App, sender: &Addr, counter: u64) -> Result<(), ContractError> {
        app.execute_contract(
            sender.clone(),
            self.0.clone(),
            &ExecMsg::Reset { counter: counter },
            &[],
        )
        .map_err(|err| err.downcast::<ContractError>().unwrap())?;
        Ok(())
    }

    #[track_caller]
    pub fn query_value(&self, app: &App) -> StdResult<ValueResp> {
        app.wrap()
            .query_wasm_smart(self.0.clone(), &QueryMsg::Value {})
    }

    #[track_caller]
    pub fn migrate(app: &mut App, contract: Addr, code_id: u64, sender: &Addr) -> StdResult<Self> {
        app.migrate_contract(sender.clone(), contract.clone(), &Empty {}, code_id)
            .map_err(|err| err.downcast::<StdError>().unwrap())?;

        Ok(CountingContract(contract.clone()))
    }
}
