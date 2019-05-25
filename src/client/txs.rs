use failure::Error;
use futures::{future, Future};

pub fn txs_encode() -> impl Future<Item = (), Error = Error> {
    future::ok(())
}
