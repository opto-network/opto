pub enum BuildMode {
	Release,
	Debug,
}
#[cfg(target_arch = "wasm32")]
pub fn build(_: BuildMode) {}

#[cfg(target_arch = "wasm32")]
pub fn build_in_release() {}

#[cfg(not(target_arch = "wasm32"))]
mod bundle;

#[cfg(not(target_arch = "wasm32"))]
mod project;

#[cfg(not(target_arch = "wasm32"))]
mod targets;

#[cfg(not(target_arch = "wasm32"))]
pub fn build(when: BuildMode) {
	use {bundle::Bundle, project::Project};

	if std::env::var("opto_PREDICATE_BUILD").is_ok() {
		return;
	}

	std::env::set_var("opto_PREDICATE_BUILD", "1");

	#[cfg(not(debug_assertions))]
	let is_release = true;

	#[cfg(debug_assertions)]
	let is_release = false;

	if !is_release && matches!(when, BuildMode::Release) {
		return;
	}

	let project = Project::current().unwrap();
	let targets = project.targets().unwrap();
	let output_dir = project.output_dir().unwrap();

	let output_archive =
		output_dir.join(format!("{}.car", project.package_name()));

	let predicate_dir =
		output_dir.join("predicates").join(project.package_name());

	std::fs::remove_file(output_archive.clone()).ok();
	std::fs::remove_dir_all(predicate_dir.clone()).ok();
	std::fs::create_dir_all(predicate_dir.clone()).unwrap();

	let mut bundle = Bundle::default();
	for target in targets {
		println!("Building predicate {}", target.id());
		let built_predicate = target.build().unwrap();
		let pred_bin_path = predicate_dir
			.join(format!("{}.wasm", built_predicate.id))
			.display()
			.to_string();

		std::fs::write(&pred_bin_path, &built_predicate.wasm).unwrap();
		bundle.insert(built_predicate);
	}

	bundle.write(output_archive).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_in_release() {
	build(BuildMode::Release);
}
