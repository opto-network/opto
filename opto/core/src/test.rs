use {
	super::{predicate::PredicateId, Expression, Object, Transition},
	crate::{
		eval::Context,
		repr::{Expanded, Repr},
		Predicate,
	},
	alloc::{collections::BTreeMap, vec::Vec},
};

pub use crate::env::StaticEnvironment;

pub type PredicateFn =
	fn(Context<'_, StaticEnvironment>, &Transition<Expanded>, &[u8]) -> bool;

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
		&'a Predicate,
	) -> Result<
		alloc::boxed::Box<
			dyn FnOnce(
				Context<'_, StaticEnvironment>,
				&Transition<Expanded>,
				&[u8],
			) -> bool,
		>,
		(),
	> {
		move |predicate: &'a Predicate| {
			let predicate = self.predicates.get(&predicate.id).cloned().ok_or(())?;

			Ok(alloc::boxed::Box::new(
				move |context: Context<'_, StaticEnvironment>,
				      transition: &Transition<Expanded>,
				      params: &[u8]| { predicate(context, transition, params) },
			))
		}
	}
}

pub fn predicate(id: u32, params: &[u8]) -> Predicate {
	Predicate {
		id: PredicateId(id),
		params: params.to_vec(),
	}
}

pub fn object(
	policies: Vec<Predicate>,
	unlock: Expression<Predicate>,
	data: Vec<u8>,
) -> Object<Predicate, Vec<u8>> {
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
				unlock: Predicate {
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

	pub fn with_unlock(mut self, unlock: Expression<Predicate>) -> Self {
		self.object.unlock = unlock;
		self
	}

	pub fn with_data(mut self, data: Vec<u8>) -> Self {
		self.object.data = data;
		self
	}

	pub fn build(self) -> Object<Predicate, Vec<u8>> {
		self.object
	}
}

impl Default for ObjectBuilder<Expanded> {
	fn default() -> Self {
		Self::new()
	}
}
