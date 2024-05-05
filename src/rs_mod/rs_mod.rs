//! Simple open/read/close module for Linux 6.3.0. That version has most of supporting Rust code
//! for writing drivers.

use kernel::file::{self, File};
use kernel::io_buffer::IoBufferWriter;
use kernel::prelude::*;
use kernel::{c_str, miscdev};

const STRDATA: &CStr = c_str!("Hello from kernel!\n");

struct SimpleRsMod {
    #[allow(unused)]
    reg: Pin<Box<miscdev::Registration<ReadStr>>>,
}

struct ReadStr;

impl kernel::Module for SimpleRsMod {
    fn init(name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("init\n");

        let reg = miscdev::Registration::new_pinned(fmt!("{name}"), ())?;

        Ok(Self { reg })
    }
}

impl Drop for SimpleRsMod {
    fn drop(&mut self) {
        pr_info!("exit\n");
    }
}

#[vtable]
impl file::Operations for ReadStr {
    fn open(_context: &(), _file: &File) -> Result<()> {
        Ok(())
    }

    fn read(
        _data: (),
        _file: &File,
        user_buffer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        if user_buffer.is_empty() || offset != 0 {
            return Ok(0);
        }

        user_buffer.write_slice(STRDATA.as_bytes())?;

        Ok(STRDATA.len())
    }
}

module! {
    type: SimpleRsMod,
    name: "rs_mod",
    author: "Ruslan Akbashev",
    description: "Simple read() module",
    license: "GPL",
}
