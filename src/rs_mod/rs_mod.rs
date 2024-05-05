//! A somewhat better simple Rust character device

use core::ffi::{c_char, c_int, c_uint};
use core::ptr;

use kernel::error::to_result;
use kernel::prelude::*;
use kernel::{bindings, c_str, Module};

#[allow(dead_code)]
struct SimpleRsMod {
    chrdev: CharDev,
    device: Device,
}

#[allow(dead_code)]
struct CharDev {
    name: *const c_char,
    fops: Box<bindings::file_operations>,
    major: c_uint,
}

struct Device {
    class: *const bindings::class,
    dev_num: u32,
}

type FopsModCb = fn(&mut bindings::file_operations);

const DRV_NAME: *const c_char = c"rs_mod".as_ptr();

const OUTPUT: &CStr = c_str!("Hello from kernel!\n");
const OUTLEN: usize = OUTPUT.len();

#[allow(clippy::cast_possible_wrap)]
const EINVAL: isize = -(bindings::EINVAL as isize);

#[allow(clippy::cast_possible_wrap)]
const EFAULT: isize = -(bindings::EFAULT as isize);

impl Module for SimpleRsMod {
    fn init(module: &'static ThisModule) -> Result<Self> {
        pr_info!("init\n");

        let chrdev = CharDev::new(DRV_NAME, module, |f| {
            f.read = Some(sdrv_read);
        })?;

        let device = Device::new(DRV_NAME, &chrdev)?;

        Ok(Self { chrdev, device })
    }
}

impl Drop for SimpleRsMod {
    fn drop(&mut self) {
        pr_info!("exit\n");
    }
}

impl CharDev {
    fn new(name: *const c_char, owner: &'static ThisModule, fops_cb: FopsModCb) -> Result<Self> {
        let owner = owner.as_ptr();

        let mut fops = bindings::file_operations {
            owner,
            ..Default::default()
        };

        fops_cb(&mut fops);

        let mut fops = Box::new(fops, kernel::alloc::flags::GFP_KERNEL)?;

        let major = to_res(register_chrdev(0, name, fops.as_mut()))?;
        let major = u32::try_from(major)?;

        pr_info!("registered chardev maj={}\n", major);

        Ok(Self { name, fops, major })
    }

    const fn dev_num(&self) -> u32 {
        mkdev(self.major, 0)
    }
}

unsafe impl Sync for CharDev {}

impl Drop for CharDev {
    fn drop(&mut self) {
        unregister_chrdev(self.major, self.name);
    }
}

impl Device {
    fn new(name: *const c_char, chrdev: &CharDev) -> Result<Self> {
        let class = unsafe { bindings::class_create(name) };
        let class = from_err_ptr(class)?;

        let dev_num = chrdev.dev_num();

        let device = unsafe {
            bindings::device_create(class, ptr::null_mut(), dev_num, ptr::null_mut(), name)
        };
        let _device = from_err_ptr(device)?;

        Ok(Self { class, dev_num })
    }
}

unsafe impl Sync for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            bindings::device_destroy(self.class, self.dev_num);
            bindings::class_destroy(self.class);
        }
    }
}

fn register_chrdev(maj: u32, name: *const c_char, fops: *const bindings::file_operations) -> c_int {
    unsafe { bindings::__register_chrdev(maj, 0, 256, name, fops) }
}

fn unregister_chrdev(maj: u32, name: *const c_char) {
    unsafe { bindings::__unregister_chrdev(maj, 0, 256, name) };
}

const fn mkdev(ma: u32, mi: u32) -> u32 {
    ma << bindings::MINORBITS | mi
}

extern "C" fn sdrv_read(
    _file: *mut bindings::file,
    user_buf: *mut c_char,
    mut count: usize,
    offset: *mut bindings::loff_t,
) -> isize {
    let offset_copy = unsafe { *offset };
    let Ok(offset_u) = usize::try_from(offset_copy) else {
        return EINVAL;
    };

    pr_info!("read, offset={} bytes={}\n", offset_copy, count);

    if offset_u >= OUTLEN {
        return 0;
    }

    if offset_u + count >= OUTLEN {
        count = OUTLEN - offset_u;
    }

    let out_ptr = unsafe { OUTPUT.as_char_ptr().add(offset_u) };

    if copy_to_user(user_buf, out_ptr, count) != 0 {
        return EFAULT;
    }

    let Ok(count_i) = i64::try_from(count) else {
        return EINVAL;
    };

    unsafe {
        *offset += count_i;
    }

    let Ok(count_iz) = isize::try_from(count) else {
        return EINVAL;
    };

    count_iz
}

fn copy_to_user<T>(to: *mut T, from: *const T, n: usize) -> usize {
    let n64 = n as u64;

    let r64 = unsafe { bindings::_copy_to_user(to.cast(), from.cast(), n64) };

    usize::try_from(r64).unwrap()
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

// This needs to use fn from the kernel crate, eventually
fn from_err_ptr<T>(ptr: *mut T) -> Result<*mut T> {
    let void_ptr = ptr.cast();
    let is_err = unsafe { bindings::IS_ERR(void_ptr) };

    if is_err {
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
    name: "rs_mod",
    author: "Ruslan Akbashev",
    description: "Simple open/read/close module",
    license: "GPL",
}
