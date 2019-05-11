extern crate num256;

pub mod coin;
pub mod msg;
pub mod stdfee;
pub mod stdsignmsg;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
