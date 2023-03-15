// use std::sync::Arc;

// use ethers::{
//     abi::{ParamType, Token},
//     providers::Middleware,
//     types::{Bytes, H160},
// };

// use crate::{abi::GetTokenBalanceBatchRequest, error::ExecutorError};

// pub async fn get_token_balance_batch_request<M: Middleware>(
//     request_data: &mut [(H160, H160)],
//     middleware: Arc<M>,
// ) -> Result<(), ExecutorError<M>> {
//     let mut request_data_tokens = vec![];

//     for data in request_data.iter() {}

//     let constructor_args = Token::Tuple(vec![Token::Tuple()]);
//     let deployer =
//         GetTokenBalanceBatchRequest::deploy(middleware.clone(), constructor_args).unwrap();

//     let return_data: Bytes = deployer.call_raw().await?;

//     let return_data_tokens = ethers::abi::decode(
//         &[ParamType::Array(Box::new(ParamType::Uint(256)))],
//         &return_data,
//     )?;

//     let mut balances = vec![];
//     //Update pool data
//     for tokens in return_data_tokens {
//         if let Some(tokens_arr) = tokens.into_array() {
//             for token in tokens_arr {
//                 if let Some(balance) = token.into_uint() {
//                     balances.push(balance);
//                 }
//             }
//         }
//     }
//     Ok(())
// }
