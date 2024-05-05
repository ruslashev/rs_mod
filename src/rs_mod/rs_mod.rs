//! Simple open/read/close module for Linux 6.3.0. That version has most of supporting Rust code
//! for writing drivers.

use core::ffi::{c_char, c_int};
use core::ptr::null_mut;

use kernel::{bindings, chrdev, c_str, to_result};
use kernel::io_buffer::IoBufferWriter;
use kernel::file::{self, File};
use kernel::prelude::*;

const STRDATA: &'static CStr = c_str!("Hello from kernel!\n");

struct SimpleRsMod {
    #[allow(unused)]
    reg: Pin<Box<chrdev::Registration<1>>>,
}

struct ReadStr;

impl kernel::Module for SimpleRsMod {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("init, name = {:?}\n", name);

        let mut reg = chrdev::Registration::new_pinned(name, 0, module)?;

        reg.as_mut().register::<ReadStr>()?;

        let class = class_create(module, name);

        let devnum = mkdev(major, 0);

        let device = bindings::device_create(class, null_mut(), devnum, null_mut(), name);
        let _device = from_err_ptr(device)?;

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
        _this: (),
        _file: &File,
        user_buffer: &mut impl IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {
        let _bytes_left = user_buffer.len();
        let strlen = STRDATA.len();
        let slice = &STRDATA.as_bytes()[..];
        let written = strlen;

        user_buffer.write_slice(slice)?;

        Ok(written)
    }

}

fn class_create(owner: &ThisModule, name: &CStr) -> Result<*mut bindings::class> {
    let mut key = bindings::lock_class_key {};

    unsafe {
        let name_ptr = core::mem::transmute::<*const u8, *const c_char>(name.as_ptr());
        let mod_ptr = core::mem::transmute::<&ThisModule, *mut bindings::module>(owner);
        let ptr = bindings::__class_create(mod_ptr, name_ptr, &mut key);

        from_err_ptr(ptr)
    }
}

// This needs to use fn from the kernel crate, eventually
fn from_err_ptr<T>(ptr: *mut T) -> Result<*mut T> {
    let void_ptr = ptr.cast();

    if unsafe { bindings::IS_ERR(void_ptr) } {
        let err = unsafe { bindings::PTR_ERR(void_ptr) };
        let err_int = c_int::try_from(err)?;
        let Err(e) = to_result(err_int) else {
            unreachable!()
        };
        Err(e)
    } else {
        Ok(ptr)
    }
}

fn mkdev(ma: u32, mi: u32) -> u32 {
    ma << bindings::MINORBITS | mi
}

module! {
    type: SimpleRsMod,
    name: "rs_mod",
    author: "Ruslan Akbashev",
    description: "Simple read() module",
    license: "GPL",
}
