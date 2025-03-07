#[macro_use]
mod primitive;

mod arith;
pub mod gfp;
pub mod curve;
pub mod gfn;

// Only LIMB=64 supported now.
pub type LIMB = u64;
pub const LIMB_BITS:usize = 64;
pub const NLIMBS:usize = 256/LIMB_BITS;
pub type DoubleLIMB = u128;


// #[cfg(target_pointer_width = "64")]

// #[cfg(target_pointer_width = "64")]

// #[cfg(target_pointer_width = "64")]

// #[cfg(target_pointer_width = "32")]
// pub type LIMB = u32;

// #[cfg(target_pointer_width = "32")]
// pub const LIMB_SIZE:usize = 8;

// #[cfg(target_pointer_width = "32")]
// pub type DoubleLIMB = u64;