/* Copyright (c) Fortanix, Inc.
 *
 * Licensed under the GNU General Public License, version 2 <LICENSE-GPL or
 * https://www.gnu.org/licenses/gpl-2.0.html> or the Apache License, Version
 * 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>, at your
 * option. This file may not be copied, modified, or distributed except
 * according to those terms. */

#[cfg(not(feature = "std"))]
use crate::alloc_prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::{Mutex, MutexGuard};
    } else {
        use spin::{Mutex, MutexGuard};
    }
}

use core::ptr;

use mbedtls_sys::types::raw_types::c_int;

pub struct StaticMutex {
    guard: Option<MutexGuard<'static, ()>>,
    mutex: Mutex<()>,
}

#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mbedtls_mutex_init: unsafe extern "C" fn(mutex: *mut *mut StaticMutex) =
    StaticMutex::init;
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mbedtls_mutex_free: unsafe extern "C" fn(mutex: *mut *mut StaticMutex) =
    StaticMutex::free;
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mbedtls_mutex_lock: unsafe extern "C" fn(mutex: *mut *mut StaticMutex) -> c_int =
    StaticMutex::lock;
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static mut mbedtls_mutex_unlock: unsafe extern "C" fn(mutex: *mut *mut StaticMutex) -> c_int =
    StaticMutex::unlock;

// The nightly compiler complains that StaticMutex has no representation hint,
// but this is not an issue because this pointer is opaque to mbedtls
#[allow(improper_ctypes)]
impl StaticMutex {
    unsafe extern "C" fn init(mutex: *mut *mut StaticMutex) {
        if let Some(m) = mutex.as_mut() {
            *m = Box::into_raw(Box::new(StaticMutex {
                guard: None,
                mutex: Mutex::new(()),
            }));
        }
    }

    unsafe extern "C" fn free(mutex: *mut *mut StaticMutex) {
        if let Some(m) = mutex.as_mut() {
            if *m != ptr::null_mut() {
                let mut mutex = Box::from_raw(*m);
                mutex.guard.take();
                *m = ptr::null_mut();
            }
        }
    }

    unsafe extern "C" fn lock(mutex: *mut *mut StaticMutex) -> c_int {
        if let Some(m) = mutex.as_mut().and_then(|p| p.as_mut()) {
            let guard = m.mutex.lock();

            cfg_if::cfg_if! {
                if #[cfg(feature = "std")] {
                    m.guard = Some(guard.unwrap());
                } else {
                    m.guard = Some(guard);
                }
            }

            0
        } else {
            ::mbedtls_sys::MBEDTLS_ERR_THREADING_BAD_INPUT_DATA
        }
    }

    unsafe extern "C" fn unlock(mutex: *mut *mut StaticMutex) -> c_int {
        if let Some(m) = mutex.as_mut().and_then(|p| p.as_mut()) {
            m.guard.take();
            0
        } else {
            ::mbedtls_sys::MBEDTLS_ERR_THREADING_BAD_INPUT_DATA
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_free() {
        unsafe {
            let mut mutex: *mut StaticMutex = ptr::null_mut();
            mbedtls_mutex_init(&mut mutex);
            mbedtls_mutex_free(&mut mutex);
            mbedtls_mutex_free(&mut mutex);
        }
    }
}
