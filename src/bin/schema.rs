use cosmwasm_schema::write_api;
use counting_contract::msg::{InstantiateMsg, ExecMsg, QueryMsg};

fn main(){
    write_api!{
        instantiate: InstantiateMsg,
        execute: ExecMsg,
        query: QueryMsg,
    }
}