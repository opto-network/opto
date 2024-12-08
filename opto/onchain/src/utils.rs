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
