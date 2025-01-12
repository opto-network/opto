#![allow(dead_code)]

use opto_core::*;

#[macro_export]
macro_rules! ensure {
	($e:expr) => {
		if !($e) {
			return false;
		}
	};
}

pub fn is_unlock(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.role, Role::Unlock(_, _))
}

pub fn is_policy(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.role, Role::Policy(_, _))
}

pub fn is_input(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Input)
}

pub fn is_output(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Output)
}

pub fn is_ephemeral(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Ephemeral)
}

pub fn is_only_policy_of_this_type(
	ctx: &Context<'_, impl Environment>,
) -> bool {
	ctx
		.object
		.policies
		.iter()
		.filter(|p| p.id == ctx.predicate_id())
		.count()
		== 1
}

pub fn is_the_only_policy(ctx: &Context<'_, impl Environment>) -> bool {
	is_policy(ctx) && ctx.object.policies.len() == 1
}

#[cfg(all(not(test), target_arch = "wasm32"))]
#[link(wasm_import_module = "env")]
extern "C" {
	#[allow(dead_code)]
	#[link_name = "debug"]
	pub fn debug_syscall(message: u32, len: u32);
}

#[cfg(all(not(test), target_arch = "wasm32"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        {
          let s = alloc::format!($($arg)*);
					let s = alloc::format!("[{}:{}] {s}", file!(), line!());
          unsafe { $crate::utils::debug_syscall(s.as_ptr() as u32, s.len() as u32)
};      }
    };
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        {
          #[cfg(any(test, feature = "std"))]
					let s = format!($($arg)*);
          #[cfg(any(test, feature = "std"))]
					let s = format!("[{}:{}] {s}", file!(), line!());
          #[cfg(any(test, feature = "std"))]
          eprintln!("{s}");
        }
    };
}
