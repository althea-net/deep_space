use failure::Error;
use futures::{future, Future};

pub fn encode() -> impl Future<Item = (), Error = Error> {
    future::ok(())
}
