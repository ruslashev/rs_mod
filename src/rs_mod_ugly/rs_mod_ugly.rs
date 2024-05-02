//! This module is insanely ugly and it's intentional. This is because it's written for Linux
//! 6.9.0, which doesn't have all the pretty layers of abstractions for writing Rust drivers
//! upstreamed. Therefore, it uses bindings, and is effectively an inferior translation from C.

use core::ffi::{c_char, c_int};
use core::ptr::{addr_of, null_mut};

use kernel::bindings::{self, file_operations};
use kernel::error::to_result;
use kernel::prelude::*;

const DRV_NAME: *const c_char = c"rs_mod_ugly".as_ptr();

static mut FOPS: file_operations = file_operations {
    owner: null_mut(),
    read: Some(sdrv_read),
    llseek: None,
    write: None,
    read_iter: None,
    write_iter: None,
    iopoll: None,
    iterate_shared: None,
    poll: None,
    unlocked_ioctl: None,
    compat_ioctl: None,
    mmap: None,
    mmap_supported_flags: 0,
    flush: None,
    release: None,
    fsync: None,
    fasync: None,
    lock: None,
    get_unmapped_area: None,
    check_flags: None,
    splice_write: None,
    splice_read: None,
    splice_eof: None,
    fallocate: None,
    show_fdinfo: None,
    copy_file_range: None,
    flock: None,
    open: None,
    setlease: None,
    remap_file_range: None,
    fadvise: None,
    uring_cmd: None,
    uring_cmd_iopoll: None,
};

struct SimpleRsMod {
    major: u32,
    class: *mut bindings::class,
    devnum: u32,
}

unsafe impl Sync for SimpleRsMod {}

impl kernel::Module for SimpleRsMod {
    fn init(module: &'static ThisModule) -> Result<Self> {
        pr_info!("init\n");

        let inst = unsafe {
            FOPS.owner = module.as_ptr();

            let major = to_res(register_chrdev(0, DRV_NAME, addr_of!(FOPS)))? as u32;

            let class = bindings::class_create(DRV_NAME);
            let class = from_err_ptr(class)?;

            let devnum = mkdev(major, 0);

            let device = bindings::device_create(class, null_mut(), devnum, null_mut(), DRV_NAME);
            let _device = from_err_ptr(device)?;

            Self {
                major,
                class,
                devnum,
            }
        };

        Ok(inst)
    }
}

impl Drop for SimpleRsMod {
    fn drop(&mut self) {
        pr_info!("exit\n");

        unsafe {
            bindings::device_destroy(self.class, self.devnum);
            bindings::class_destroy(self.class);
        }

        unregister_chrdev(self.major, DRV_NAME);
    }
}

extern "C" fn sdrv_read(
    _file: *mut bindings::file,
    user_buf: *mut c_char,
    mut count: usize,
    pos: *mut bindings::loff_t,
) -> isize {
    let output = "Hello, world!";
    let outlen = output.len();

    unsafe {
        pr_info!("read, offset={} bytes={}", *pos, count);

        let Ok(pos_u) = usize::try_from(*pos) else {
            return -(bindings::EINVAL as isize);
        };
        let Ok(count_u) = u64::try_from(count) else {
            return -(bindings::EINVAL as isize);
        };
        let Ok(count_iz) = isize::try_from(count) else {
            return -(bindings::EINVAL as isize);
        };

        if pos_u >= outlen {
            return 0;
        }

        if pos_u + count >= outlen {
            count = outlen - pos_u - 1;
        }

        let outptr = output.as_ptr().add(pos_u);

        if bindings::_copy_to_user(user_buf.cast(), outptr.cast(), count_u) != 0 {
            return -(bindings::EFAULT as isize);
        }

        let Ok(count_i) = i64::try_from(count) else {
            return -(bindings::EINVAL as isize);
        };

        *pos += count_i;

        count_iz
    }
}

fn register_chrdev(maj: u32, name: *const c_char, fops: *const file_operations) -> c_int {
    unsafe { bindings::__register_chrdev(maj, 0, 256, name, fops) }
}

fn unregister_chrdev(maj: u32, name: *const c_char) {
    unsafe { bindings::__unregister_chrdev(maj, 0, 256, name) };
}

// Don't really understand why [`kernel::error::to_result`] has () as success type
fn to_res(val: c_int) -> Result<c_int, Error> {
    if val < 0 {
        let Err(e) = to_result(val) else {
            unreachable!()
        };
        Err(e)
    } else {
        Ok(val)
    }
}

fn mkdev(ma: u32, mi: u32) -> u32 {
    ma << bindings::MINORBITS | mi
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

module! {
    type: SimpleRsMod,
    name: "rs_mod_ugly",
    author: "Ruslan Akbashev",
    description: "Simple open/read/close module",
    license: "GPL",
}
