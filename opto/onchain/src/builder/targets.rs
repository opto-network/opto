use {
	super::project::Project,
	cargo::{
		core::{
			compiler::{CompileKind, CompileMode, CompileTarget, Executor},
			PackageId,
			Target,
		},
		ops::{CompileFilter, CompileOptions, Packages},
		util::Filesystem,
		CargoResult,
	},
	cargo_util::ProcessBuilder,
	nanoid::nanoid,
	std::{env::temp_dir, path::PathBuf, sync::Arc},
};

pub struct PredicateWasmBinary {
	pub id: u32,
	pub wasm: Vec<u8>,
}

pub struct BuildTarget<'p> {
	predicate_id: u32,
	target_dir: PathBuf,
	project: &'p Project,

	wrapper_dir: PathBuf,
	wrapper_manifest: PathBuf,
	wrapper_package_name: String,
	wrapper_target_dir: PathBuf,
}

impl<'p> BuildTarget<'p> {
	pub fn new(predicate_id: u32, project: &'p Project) -> anyhow::Result<Self> {
		let random = nanoid!(6);
		let target_dir =
			temp_dir().join(format!("predicate_{predicate_id}_{random}"));
		std::fs::create_dir_all(&target_dir)?;

		let wrapper_dir = target_dir.join("wrapper");
		let wrapper_manifest = wrapper_dir.join("Cargo.toml");
		let wrapper_package_name = format!("predicate-{}", predicate_id);
		let wrapper_target_dir = target_dir.join("target");

		Ok(Self {
			predicate_id,
			target_dir,
			project,
			wrapper_dir,
			wrapper_manifest,
			wrapper_package_name,
			wrapper_target_dir,
		})
	}

	pub const fn id(&self) -> u32 {
		self.predicate_id
	}

	fn create_wrapper_project(
		&self,
	) -> anyhow::Result<cargo::core::Workspace<'_>> {
		std::fs::create_dir_all(&self.wrapper_dir)?;

		let cargo_toml = format!(
			r###"
      [package]
      name = "{}"
      version = "1.0.0"
      edition = "2021"

			[lib]
			crate-type = ["cdylib"]

      [dependencies]
      {} = {{ path = "{}" }}
      wee_alloc = "0.4.5"
       
      [profile.release]
      opt-level = "z"
      lto = true
      codegen-units = 1
      panic = "abort"
      debug = false
			overflow-checks = false
			debug-assertions = false

      "###,
			self.wrapper_package_name,
			self.project.package_name(),
			self.project.package_dir().to_string_lossy(),
		);

		std::fs::write(&self.wrapper_manifest, cargo_toml)?;
		std::fs::create_dir_all(self.wrapper_dir.join("src"))?;

		let lib_rs = format!(
			r###"
			extern crate alloc;

      #[global_allocator]
      static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

			const _: () = {{
				#[export_name="_alloc"]
				extern "C" fn alloc(size: usize) -> *mut u8 {{
					let mut buf = alloc::vec::Vec::with_capacity(size as usize);
					let ptr = buf.as_mut_ptr();
					core::mem::forget(buf);
					ptr
				}}

				#[export_name="_dealloc"]
				extern "C" fn dealloc(ptr: *mut u8, size: usize) {{
					let _ = unsafe {{ alloc::vec::Vec::from_raw_parts(ptr, 0, size as usize) }};
				}}
			}};

      pub use {}::*;
      "###,
			self.project.package_name().replace("-", "_"),
		);
		std::fs::write(self.wrapper_dir.join("src/lib.rs"), lib_rs)?;

		let ws = cargo::core::Workspace::new(
			self.wrapper_manifest.as_path(),
			self.project.cargo_ctx(),
		)?;

		let package = ws
			.members()
			.find(|p| p.name().as_str() == self.wrapper_package_name)
			.ok_or(anyhow::anyhow!(
				"Package {} not found",
				self.project.package_name()
			))?;

		cargo::core::Workspace::ephemeral(
			package.clone(),
			self.project.cargo_ctx(),
			Some(Filesystem::new(self.wrapper_target_dir.clone())),
			false,
		)
	}

	pub fn build(self) -> anyhow::Result<PredicateWasmBinary> {
		let ws = self.create_wrapper_project()?;
		let target_name = format!("pred_{}", self.predicate_id);
		let mut compile_options =
			CompileOptions::new(self.project.cargo_ctx(), CompileMode::Build)?;
		compile_options.spec =
			Packages::Packages(vec![self.wrapper_package_name.clone()]);

		compile_options.build_config.requested_kinds = vec![CompileKind::Target(
			CompileTarget::new("wasm32-unknown-unknown")?,
		)];

		compile_options.target_rustc_args = Some(vec![
			format!("--cfg={target_name}"),
			"-Clink-arg=--export=__heap_base".into(),
		]);

		compile_options.build_config.requested_profile = "release".into();
		compile_options.filter = CompileFilter::lib_only();

		let exec: Arc<dyn Executor> =
			Arc::new(TargetedExecutor(target_name.clone()));
		cargo::ops::compile_with_exec(&ws, &compile_options, &exec)?;

		let wasm_binary = std::fs::read(self.target_dir.join(format!(
			"target/wasm32-unknown-unknown/release/{}.wasm",
			self.wrapper_package_name.replace("-", "_")
		)))?;

		#[cfg(feature = "optimize")]
		let optimizer_config = binaryen::CodegenConfig {
			optimization_level: 2,
			shrink_level: 2,
			debug_info: false,
		};

		#[cfg(feature = "optimize")]
		let mut module = binaryen::Module::read(&wasm_binary)
			.expect("rustc generated invalid wasm");

		#[cfg(feature = "optimize")]
		module.optimize(&optimizer_config);

		#[cfg(feature = "optimize")]
		let output = PredicateWasmBinary {
			id: self.predicate_id,
			wasm: module.write(),
		};

		#[cfg(not(feature = "optimize"))]
		let output = PredicateWasmBinary {
			id: self.predicate_id,
			wasm: wasm_binary,
		};

		std::fs::remove_dir_all(&self.target_dir)?;

		Ok(output)
	}
}

#[derive(Clone)]
pub struct TargetedExecutor(pub String);

impl Executor for TargetedExecutor {
	fn exec(
		&self,
		cmd: &ProcessBuilder,
		_id: PackageId,
		_target: &Target,
		_mode: CompileMode,
		on_stdout_line: &mut dyn FnMut(&str) -> CargoResult<()>,
		on_stderr_line: &mut dyn FnMut(&str) -> CargoResult<()>,
	) -> CargoResult<()> {
		cmd
			.clone()
			.arg(format!("--cfg={}", self.0))
			.exec_with_streaming(on_stdout_line, on_stderr_line, false)
			.map(drop)
	}
}
