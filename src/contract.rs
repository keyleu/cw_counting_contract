use cosmwasm_std::{Coin, DepsMut, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    msg::Parent,
    state::{ParentDonation, State, OWNER, PARENT_DONATION, STATE},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    info: MessageInfo,
    counter: u64,
    minimal_donation: Coin,
    parent: Option<Parent>,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(
        deps.storage,
        &State {
            counter: counter,
            minimal_donation: minimal_donation,
            donating_parent: parent.as_ref().map(|p| p.donating_period),
        },
    )?;

    OWNER.save(deps.storage, &info.sender)?;

    if let Some(parent) = parent {
        PARENT_DONATION.save(
            deps.storage,
            &ParentDonation {
                address: deps.api.addr_validate(&parent.addr)?,
                donating_parent_period: parent.donating_period,
                part: parent.part,
            },
        )?;
    }

    Ok(Response::new())
}

pub fn migrate(mut deps: DepsMut) -> Result<Response, ContractError> {
    let contract = get_contract_version(deps.storage)?;
    if contract.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidName(contract.contract));
    }

    let resp = match contract.version.as_str() {
        "0.1.0" => migrate_0_1_0(deps.branch())?,
        "0.2.0" => migrate_0_2_0(deps.branch())?,
        CONTRACT_VERSION => return Ok(Response::new()),
        _ => return Err(ContractError::InvalidVersion(contract.version.to_string())),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(resp)
}

pub fn migrate_0_1_0(deps: DepsMut) -> StdResult<Response> {
    const COUNTER: Item<u64> = Item::new("counter");
    const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");

    let counter = COUNTER.load(deps.storage)?;
    let minimal_donation = MINIMAL_DONATION.load(deps.storage)?;

    STATE.save(
        deps.storage,
        &State {
            counter,
            minimal_donation,
            donating_parent: None,
        },
    )?;

    Ok(Response::new())
}

pub fn migrate_0_2_0(deps: DepsMut) -> StdResult<Response> {
    #[derive(Serialize, Deserialize)]
    struct OldState {
        counter: u64,
        minimal_donation: Coin,
    }

    const OLD_STATE: Item<OldState> = Item::new("state");

    let OldState {
        counter,
        minimal_donation,
    } = OLD_STATE.load(deps.storage)?;

    STATE.save(
        deps.storage,
        &State {
            counter,
            minimal_donation,
            donating_parent: None,
        },
    )?;

    Ok(Response::new())
}

pub mod query {
    use crate::{msg::ValueResp, state::STATE};
    use cosmwasm_std::{Deps, StdResult};

    pub fn value(deps: Deps) -> StdResult<ValueResp> {
        let value = STATE.load(deps.storage)?.counter;
        Ok(ValueResp { value })
    }
}

pub mod exec {
    use cosmwasm_std::{BankMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg, to_binary};

    use crate::{
        error::ContractError,
        state::{OWNER, STATE, PARENT_DONATION}, msg::ExecMsg,
    };

    pub fn donate(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
        let mut state = STATE.load(deps.storage)?;
        let mut resp = Response::new();

        if state.minimal_donation.amount.is_zero()
            || info.funds.iter().any(|coin| {
                coin.denom == state.minimal_donation.denom
                    && coin.amount >= state.minimal_donation.amount
            })
        {
            state.counter += 1;
            if let Some(parent) = &mut state.donating_parent {
                *parent -= 1;
    
                if *parent == 0 {
                    let parent_donation = PARENT_DONATION.load(deps.storage)?;
                    *parent = parent_donation.donating_parent_period;
    
                    let funds: Vec<_> = deps
                        .querier
                        .query_all_balances(env.contract.address)?
                        .into_iter()
                        .map(|mut coin| {
                            coin.amount = coin.amount * parent_donation.part;
                            coin
                        })
                        .collect();
    
                    let msg = WasmMsg::Execute {
                        contract_addr: parent_donation.address.to_string(),
                        msg: to_binary(&ExecMsg::Donate {})?,
                        funds,
                    };
    
                    resp = resp
                        .add_message(msg)
                        .add_attribute("donated_to_parent", parent_donation.address.to_string());
                }
            }
            
            STATE.save(deps.storage, &state)?;
        }
        //COUNTER.update(deps.storage, |counter| -> StdResult<_> { Ok(counter + 1) })?;

        resp = resp
            .add_attribute("action", "donate")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", state.counter.to_string());

        Ok(resp)
    }

    pub fn reset(
        deps: DepsMut,
        info: MessageInfo,
        counter: u64,
    ) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {
                owner: owner.to_string(),
            });
        }

        STATE.update(deps.storage, |mut state| -> StdResult<_> {
            state.counter = counter;
            Ok(state)
        })?;

        let resp = Response::new()
            .add_attribute("action", "reset")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", counter.to_string());
        Ok(resp)
    }

    pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {
                owner: owner.to_string(),
            });
        }

        let balance = deps.querier.query_all_balances(&env.contract.address)?;
        let bank_msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: balance,
        };
        let resp = Response::new()
            .add_message(bank_msg)
            .add_attribute("action", "withdraw")
            .add_attribute("sender", info.sender.as_str());
        Ok(resp)
    }
}
