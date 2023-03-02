use ethers::providers::Middleware;

use crate::error::ExecutorError;

pub async fn start_check_in_service<M: Middleware>() -> Result<(), ExecutorError<M>> {
    //Check when the last check in was

    //If the last check in was past the threshold, check in

    loop {}

    //Calc the sleep time

    //Sleep and await

    Ok(())
}
