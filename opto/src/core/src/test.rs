use {
	super::{predicate::PredicateId, Expression, Object, Transition},
	crate::{
		repr::{AtRest, Expanded},
		Context,
		Repr,
	},
	alloc::{collections::BTreeMap, vec::Vec},
};

pub type PredicateFn = fn(Context<'_>, &Transition<Expanded>, &[u8]) -> bool;

#[derive(Default)]
pub struct MockMachine {
	pub predicates: BTreeMap<PredicateId, PredicateFn>,
}

/// In the mock machine we can add predicates ourselves and map them to native
/// rust functions that can be called to evaluate the predicate.
impl MockMachine {
	pub fn add_predicate(&mut self, id: PredicateId, func: PredicateFn) {
		self.predicates.insert(id, func);
	}

	/// Returns a factory function that can be used to create a functor that can
	/// be used to create executable instances of at-rest predicates.
	///
	/// In this setting it will essentially return a closure that will look up
	/// the predicate in the mock machine and return a boxed function pointer to
	/// the native rust function that can be used to evaluate the predicate.
	pub fn factory_fn<'a>(
		&'a self,
	) -> impl Fn(
		&'a AtRest,
	) -> Result<
		alloc::boxed::Box<
			dyn FnOnce(Context, &Transition<Expanded>, &[u8]) -> bool,
		>,
		(),
	> {
		move |predicate: &'a AtRest| {
			let predicate = self.predicates.get(&predicate.id).cloned().ok_or(())?;

			Ok(alloc::boxed::Box::new(
				move |context: Context<'_>,
				      transition: &Transition<Expanded>,
				      params: &[u8]| { predicate(context, transition, params) },
			))
		}
	}
}

pub fn predicate(id: u32, params: &[u8]) -> AtRest {
	AtRest {
		id: PredicateId(id),
		params: params.to_vec(),
	}
}

pub fn object(
	policies: Vec<AtRest>,
	unlock: Expression<AtRest>,
	data: Vec<u8>,
) -> Object<AtRest, Vec<u8>> {
	Object {
		policies,
		unlock,
		data,
	}
}

pub struct ObjectBuilder<R: Repr> {
	object: Object<R::Predicate, R::Data>,
}

impl ObjectBuilder<Expanded> {
	pub fn new() -> Self {
		Self {
			object: Object {
				policies: alloc::vec![],
				unlock: AtRest {
					id: PredicateId(100), // const true
					params: alloc::vec![1],
				}
				.into(),
				data: alloc::vec![],
			},
		}
	}

	pub fn with_policy(mut self, policy: <Expanded as Repr>::Predicate) -> Self {
		self.object.policies.push(policy);
		self
	}

	pub fn with_unlock(mut self, unlock: Expression<AtRest>) -> Self {
		self.object.unlock = unlock;
		self
	}

	pub fn with_data(mut self, data: Vec<u8>) -> Self {
		self.object.data = data;
		self
	}

	pub fn build(self) -> Object<AtRest, Vec<u8>> {
		self.object
	}
}

impl Default for ObjectBuilder<Expanded> {
	fn default() -> Self {
		Self::new()
	}
}
