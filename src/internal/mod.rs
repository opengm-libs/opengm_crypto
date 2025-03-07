pub mod cpuid;

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! once {
    ($b: expr) => {{
        use core::sync::atomic::{AtomicI32, Ordering};
        const STATUS_INIT: i32 = -1;
        const STATUS_PENDING: i32 = 1;
        const STATUS_DONE: i32 = 0;
        static STATUS: AtomicI32 = AtomicI32::new(STATUS_INIT);
        if STATUS.load(Ordering::Acquire) != STATUS_DONE {
            loop {
                if let Ok(_) = STATUS.compare_exchange(STATUS_INIT, STATUS_PENDING, Ordering::Acquire, Ordering::Relaxed) {
                    $b
                    STATUS.store(STATUS_DONE, Ordering::Release);
                    break;
                }
            }
        }
    }};
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! once {
    ($b: block) => {{
        static START: std::sync::Once = std::sync::Once::new();
        START.call_once(|| $b);
    }};
}

// do $init once, during $init, do $pending.
// do $done when $init done.
#[macro_export]
macro_rules! once_or {
    ($init: block, $pending: block, $done: block) => {{
        use core::sync::atomic::{AtomicI32, Ordering};
        const STATUS_INIT: i32 = 0;
        const STATUS_PENDING: i32 = 1;
        const STATUS_DONE: i32 = 2;
        static STATUS: AtomicI32 = AtomicI32::new(STATUS_INIT);
        match STATUS.compare_exchange(STATUS_INIT, STATUS_PENDING, Ordering::Acquire, Ordering::Relaxed) {
            Err(STATUS_DONE) => {
                return $done
            },
            Ok(_) => {
                #[cfg(not(feature = "std"))]
                {
                    $init
                    STATUS.store(STATUS_DONE, Ordering::Release);
                }
                #[cfg(feature = "std")]
                thread::spawn(|| {
                    $init
                    STATUS.store(STATUS_DONE, Ordering::Release);
                });
            },
            Err(_) => {},
        }
        return $pending
    }};
}

// alignX, X bytes align
#[repr(align(8))]
pub(crate) struct Aligned8<T: core::any::Any, const N: usize>(pub [T; N]);

#[repr(align(16))]
pub(crate) struct Aligned16<T: core::any::Any, const N: usize>(pub [T; N]);

#[repr(align(32))]
pub(crate) struct Aligned32<T: core::any::Any, const N: usize>(pub [T; N]);

#[repr(align(64))]
pub(crate) struct Aligned64<T: core::any::Any, const N: usize>(pub [T; N]);


#[macro_export]
macro_rules! aligned64 {
    ([$a: expr; $N: expr]) => {
        &($crate::internal::Aligned64::<_, $N>([$a; $N]).0)
    };
}

#[macro_export]
macro_rules! aligned64_mut {
    ([$a: expr; $N: expr]) => {
        &mut ($crate::internal::Aligned64::<_, $N>([$a; $N]).0)
    };
}


#[macro_export]
macro_rules! aligned32 {
    ([$a: expr; $N: expr]) => {
        &($crate::internal::Aligned32::<_, $N>([$a; $N]).0)
    };
}

#[macro_export]
macro_rules! aligned32_mut {
    ([$a: expr; $N: expr]) => {
        &mut ($crate::internal::Aligned32::<_, $N>([$a; $N]).0)
    };
}


#[macro_export]
macro_rules! aligned16 {
    ([$a: expr; $N: expr]) => {
        &($crate::internal::Aligned16::<_, $N>([$a; $N]).0)
    };
}

#[macro_export]
macro_rules! aligned16_mut {
    ([$a: expr; $N: expr]) => {
        &mut ($crate::internal::Aligned16::<_, $N>([$a; $N]).0)
    };
}


#[macro_export]
macro_rules! aligned8 {
    ([$a: expr; $N: expr]) => {
        &($crate::internal::Aligned8::<_, $N>([$a; $N]).0)
    };
}

#[macro_export]
macro_rules! aligned8_mut {
    ([$a: expr; $N: expr]) => {
        &mut ($crate::internal::Aligned8::<_, $N>([$a; $N]).0)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned() {
        let a = aligned64!([0u8; 16]);
        println!("{:?}", a.as_ptr());
        assert_eq!((a.as_ptr() as usize) % 64, 0);
    }
    #[test]
    fn test_aligned_bug() {
        let a = &mut {Aligned64([0u8; 40]).0};
        let b = &mut Aligned64([0u8; 11]).0;
        if a.as_ptr() as usize % 64 != 0{
            println!("a unaligned:{:x}", a.as_ptr() as usize);
        }else{
            println!("a aligned:{:x}", a.as_ptr() as usize);
        };

        if b.as_ptr() as usize % 64 != 0{
            println!("b unaligned:{:x}", a.as_ptr() as usize);
        }else{
            println!("b aligned:{:x}", a.as_ptr() as usize);
        };
        
    }
}
