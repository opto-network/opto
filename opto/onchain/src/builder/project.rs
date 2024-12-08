use {
	super::targets::BuildTarget,
	core::panic,
	proc_macro2::TokenTree,
	std::{
		collections::HashSet,
		env::var,
		path::{Path, PathBuf},
	},
	syn::{visit::Visit, Meta},
	walkdir::WalkDir,
};

#[derive(Debug)]
pub(crate) struct Project {
	package_name: String,
	package_dir: PathBuf,
	manifest_file: PathBuf,
	cargo_ctx: cargo::GlobalContext,
	features: Vec<String>,
}

impl Project {
	pub fn current() -> anyhow::Result<Self> {
		let manifest_dir = var("CARGO_MANIFEST_DIR")?;
		let package_dir = PathBuf::from(&manifest_dir);
		let package_name = std::env::var("CARGO_PKG_NAME")?;
		let manifest_file = PathBuf::from(manifest_dir).join("Cargo.toml");
		let cargo_ctx = cargo::GlobalContext::default()?;

		let features: Vec<String> = std::env::vars()
			.filter_map(|(key, _)| {
				// Look for environment variables that start with "CARGO_FEATURE_"
				key
					.strip_prefix("CARGO_FEATURE_")
					.map(|feature| feature.replace('_', "-").to_lowercase())
			})
			.collect();

		Ok(Self {
			package_name,
			package_dir,
			manifest_file,
			cargo_ctx,
			features,
		})
	}

	pub fn package_name(&self) -> &str {
		&self.package_name
	}

	pub fn cargo_ctx(&self) -> &cargo::GlobalContext {
		&self.cargo_ctx
	}

	pub fn package_dir(&self) -> &PathBuf {
		&self.package_dir
	}

	pub fn features(&self) -> &[String] {
		&self.features
	}

	pub fn targets(
		&self,
	) -> anyhow::Result<impl Iterator<Item = BuildTarget<'_>>> {
		let mut output = vec![];
		let pred_ids =
			find_all_predicate_ids(self.package_dir.to_string_lossy().as_ref());

		for id in pred_ids {
			output.push(BuildTarget::new(id, self)?);
		}

		Ok(output.into_iter())
	}

	pub fn output_dir(&self) -> anyhow::Result<PathBuf> {
		let ws =
			cargo::core::Workspace::new(&self.manifest_file, self.cargo_ctx())?;
		Ok(ws.target_dir().as_path_unlocked().to_path_buf())
	}
}

fn find_all_predicate_ids(project_dir: &str) -> Vec<u32> {
	let src_dir = Path::new(project_dir).join("src");
	let mut ids = HashSet::new();
	for entry in WalkDir::new(src_dir) {
		let entry = entry.unwrap();
		let path = entry.path();
		if path.is_file() && path.extension().unwrap() == "rs" {
			let file_contents = std::fs::read_to_string(path).unwrap();
			let ast = syn::parse_file(&file_contents).unwrap();
			let mut v = AttribVisitor::default();
			v.visit_file(&ast);
			for id in v.ids.into_iter() {
				if !ids.insert(id) {
					panic!("Duplicate predicate id '{id}' in {}", path.display());
				}
			}
		}
	}

	ids.into_iter().collect()
}

#[derive(Debug, Default)]
struct AttribVisitor {
	ids: Vec<u32>,
}

impl<'ast> Visit<'ast> for AttribVisitor {
	fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
		if attr
			.path()
			.segments
			.last()
			.map(|ident| ident.ident == "predicate")
			.unwrap_or_default()
		{
			let meta = attr.meta.clone();
			if let Meta::List(list) = meta {
				let list = list.tokens.into_iter().collect::<Vec<TokenTree>>();
				if let Some(
					&[TokenTree::Ident(ref ident), TokenTree::Punct(ref punct), TokenTree::Literal(ref lit)],
				) = list.windows(3).take(1).next()
				{
					if ident == "id" && punct.as_char() == '=' {
						let id = lit.to_string().parse::<u32>().unwrap();
						self.ids.push(id);
					}
				}
			}
		}
	}
}
