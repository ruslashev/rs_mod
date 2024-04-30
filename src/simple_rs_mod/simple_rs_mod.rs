//! Simple out-of-tree module

use kernel::prelude::*;

struct SimpleRsMod;

impl kernel::Module for SimpleRsMod {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("init\n");

        Ok(Self)
    }
}

impl Drop for SimpleRsMod {
    fn drop(&mut self) {
        pr_info!("exit\n");
    }
}

module! {
    type: SimpleRsMod,
    name: "simple_rs_mod",
    author: "Ruslan Akbashev",
    description: "Simple open/read/close module",
    license: "GPL",
}
