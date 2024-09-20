use {
	super::{Object, Transition},
	crate::{repr::Expanded, Predicate, PredicateId, Repr},
	core::fmt::Debug,
};

/// An instance of an executable predicate.
///
/// Describes a predicate that is instantiated and ready to be invoked and
/// evaluated in context of a machine and a state transition. When a predicate
/// is in this state it has already fetched the underlying WASM code, was able
/// to instantiate it using a WASM virtual machine and is ready to be invoked.
///
/// Predicates in this form cannot be persisted, cloned, or transported or
/// compared and do not have a universal representation across all machines.
/// They are tied to the machine that instantiated them and are only valid in
/// the context of that machine.
pub struct InUse<'a, F>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	eval: F,
	_p: core::marker::PhantomData<&'a F>,
}

impl<'a, F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool>
	Predicate for InUse<'a, F>
{
}

impl<'a, F> InUse<'a, F>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	pub fn eval(
		self,
		context: Context<'a>,
		transition: &'a Transition<Expanded>,
		params: &'a [u8],
	) -> bool {
		(self.eval)(context, transition, params)
	}
}

/// A functor that can be used to evaluate a predicate in the context of a
/// state. This is a boxed version of the function that abstracts the underlying
/// callable type.
pub type PredicateFunctor =
	alloc::boxed::Box<dyn FnOnce(Context, &Transition<Expanded>, &[u8]) -> bool>;

impl<'a, F> InUse<'a, F>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	pub fn new(eval: F) -> Self {
		Self {
			eval,
			_p: core::marker::PhantomData,
		}
	}
}

/// Identifies the exact predicate that is being evaluated in the context of a
/// state transition.
#[derive(Debug, Clone, PartialEq)]
pub struct Context<'a> {
	pub location: Location,
	pub role: Role<'a, <Expanded as Repr>::Predicate>,
	pub object:
		&'a Object<<Expanded as Repr>::Predicate, <Expanded as Repr>::Data>,
}

impl<'a> Context<'a> {
	/// Index of the object that contains the predicate being evaluated in the
	/// state transition objects list.
	///
	/// This is the index starting from 0 of the object in either inputs,
	/// ephemerals or outputs list of the state transition.
	pub fn object_index(
		&self,
		transition: &Transition<Expanded>,
	) -> Option<usize> {
		let slice = match self.location {
			Location::Input => transition.inputs.as_slice(),
			Location::Ephemeral => transition.ephemerals.as_slice(),
			Location::Output => transition.outputs.as_slice(),
		};

		index_of(slice, self.object)
	}

	/// Predicate ID of the predicate that is being evaluated.
	pub fn predicate_id(&self) -> PredicateId {
		match self.role {
			Role::Policy(p, _) => p.id,
			Role::Unlock(p, _) => p.id,
		}
	}
}

/// Specifies where in the state transition the object that has the predicate
/// is located.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Location {
	Input,
	Ephemeral,
	Output,
}

/// Specifies the location of the predicate in an object that is part of
/// a state transition.
pub enum Role<'a, P> {
	/// The predicate is part of the object's policy.
	/// The usize is the index of the policy in the object's policy list.
	Policy(&'a P, usize),

	/// The predicate is part of the object's unlock expression.
	/// The usize is the index of the predicate in the unlock expression in its
	/// prefix traversal order as it appears in the `as_ops` method.
	Unlock(&'a P, usize),
}

impl<'a, P> Clone for Role<'a, P> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<'a, P> Copy for Role<'a, P> {}

impl<'a, P> Debug for Role<'a, P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Role::Policy(_, i) => write!(f, "Policy({})", i),
			Role::Unlock(_, i) => write!(f, "Unlock({})", i),
		}
	}
}

impl<'a, P> PartialEq for Role<'a, P> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Role::Policy(_, i), Role::Policy(_, j)) => i == j,
			(Role::Unlock(_, i), Role::Unlock(_, j)) => i == j,
			_ => false,
		}
	}
}

impl<'a, P> Role<'a, P> {
	pub const fn is_policy(&self) -> bool {
		matches!(self, Role::Policy(_, _))
	}

	pub const fn is_unlock(&self) -> bool {
		matches!(self, Role::Unlock(_, _))
	}
}

fn index_of<T>(slice: &[T], item: &T) -> Option<usize> {
	let ptr = item as *const T;
	let len = slice.len() as isize;
	// SAFETY: `ptr` is a valid pointer to `T` and `len` is the length of `slice`.
	// We are not dereferencing `ptr` and we are not accessing any element of
	// `slice` beyond the end of the slice. We're only doing pointer arithmetic.
	let slice_end = unsafe { slice.as_ptr().offset(len) };
	if slice.as_ptr() <= ptr && slice_end > ptr {
		Some(index_of_unchecked(slice, item))
	} else {
		None
	}
}

fn index_of_unchecked<T>(slice: &[T], item: &T) -> usize {
	if size_of::<T>() == 0 {
		return 0; // do what you will with this case
	}
	(item as *const T as *const () as usize
		- slice.as_ptr() as *const () as usize)
		/ size_of::<T>()
}
