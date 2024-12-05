use {
	super::{Object, Transition},
	crate::{
		env::Environment,
		expression,
		repr::{AsExpression, AsObject, AsPredicate, Executable, Expanded, Repr},
		AtRest,
		Op,
		PredicateId,
	},
	alloc::{boxed::Box, collections::VecDeque},
	core::fmt::Debug,
};

/// Errors that can occur during evaluation of a state transition
#[derive(Debug, Clone, PartialEq)]
pub enum Error<'a> {
	/// Predicate not satisfied in the context of a state transition.
	PolicyNotSatisfied(&'a AsObject<Expanded>, Location, usize),

	/// Unlock expression of an object not satisfied in the
	/// context of a state transition.
	UnlockNotSatisfied(&'a AsObject<Expanded>, Location),

	/// Inconsistent transition instantiation.
	///
	/// The `Machine` implementation that was used to convert an
	/// in-rest transition to an in-use transition returned a transition
	/// that has a different unlock shape than the original transition.
	InvalidInstance,

	/// Invalid unlock tree.
	///
	/// The unlock tree of an object is not a valid boolean expression tree.
	InvalidUnlockTree(expression::Error),
}

impl From<expression::Error> for Error<'_> {
	fn from(err: expression::Error) -> Self {
		Error::InvalidUnlockTree(err)
	}
}

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
pub struct InUse<'a, F, E: Environment + 'a>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	eval: F,
	_p: core::marker::PhantomData<(&'a F, E)>,
}

impl<'a, F, E: Environment> InUse<'a, F, E>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	pub fn eval(
		self,
		context: Context<'a, E>,
		transition: &'a Transition<Expanded>,
		params: &'a [u8],
	) -> bool {
		(self.eval)(context, transition, params)
	}
}

/// A functor that can be used to evaluate a predicate in the context of a
/// state. This is a boxed version of the function that abstracts the underlying
/// callable type.
pub type PredicateFunctor<E> = alloc::boxed::Box<
	dyn FnOnce(Context<E>, &Transition<Expanded>, &[u8]) -> bool,
>;

impl<'a, F, E: Environment + 'a> InUse<'a, F, E>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
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
pub struct Context<'a, E: Environment> {
	pub location: Location,
	pub role: Role<'a, <Expanded as Repr>::Predicate>,
	pub object:
		&'a Object<<Expanded as Repr>::Predicate, <Expanded as Repr>::Data>,
	pub env: &'a E,
}

impl<'a, E: Environment> Context<'a, E> {
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

/// Evaluates the transition in the context of a state transition.
///
/// The `'m` lifetime is the lifetime of the Machine that is evaluating the
/// transition. The `'d` lifetime is the lifetime of the transition Data that is
/// being evaluated.
impl<'a, F, E: Environment + 'a> Transition<Executable<'a, F, E>>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	/// Evaluates the transition in the context of a state transition.
	pub fn evaluate(
		self,
		source: &'a Transition<Expanded>,
		env: &'a E,
	) -> Result<(), Error<'a>> {
		// Check that the transition has the same shape as the source transition.
		if self.inputs.len() != source.inputs.len()
			|| self.ephemerals.len() != source.ephemerals.len()
			|| self.outputs.len() != source.outputs.len()
		{
			return Err(Error::InvalidInstance);
		}

		// Check that all input objects have their predicates satisfied.
		for (instance, object) in self.inputs.into_iter().zip(source.inputs.iter())
		{
			eval_object(object, Location::Input, instance, source, env)?;
		}

		// Check that all ephemeral objects have their predicates satisfied.
		for (instance, object) in
			self.ephemerals.into_iter().zip(source.ephemerals.iter())
		{
			eval_object(object, Location::Ephemeral, instance, source, env)?;
		}

		// Check that all output objects have their predicates satisfied. For output
		// object we don't need to evaluate unlock conditions, only policies.
		for (instance, object) in
			self.outputs.into_iter().zip(source.outputs.iter())
		{
			eval_policies(source, object, instance, Location::Output, env)?;
		}

		// all checks passed, transition is valid
		Ok(())
	}
}

fn eval_unlocks<'a, F, E: Environment>(
	source: &'a Transition<Expanded>,
	object: &'a AsObject<Expanded>,
	expression: AsExpression<Executable<'a, F, E>>,
	location: Location,
	env: &'a E,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	let mut object_ops = object.unlock.as_ops().iter().rev();
	let mut instance_ops = expression.to_vec();
	instance_ops.reverse();
	let mut instance_ops = instance_ops.into_iter();

	let mut stack = VecDeque::<Box<dyn FnOnce() -> bool>>::new();

	match (object_ops.next(), instance_ops.next()) {
		(Some(object_op), Some(instance_op)) => match (object_op, instance_op) {
			(op_pred @ Op::Predicate(pred), Op::Predicate(inst)) => {
				let index = index_of(object.unlock.as_ops(), op_pred)
					.expect("predicate not found in object unlock ops");
				stack.push_back(Box::new(move || {
					eval_predicate(
						pred,
						inst,
						location,
						Role::Unlock(pred, index),
						object,
						source,
						env,
					)
				}));
			}
			(Op::Not, Op::Not) => match stack.pop_back() {
				Some(operand) => stack.push_back(Box::new(move || !operand())),
				None => {
					return Err(Error::InvalidUnlockTree(
						expression::Error::MalformedExpression,
					))
				}
			},

			(Op::And, Op::And) => {
				let left = stack.pop_back().ok_or(Error::InvalidUnlockTree(
					expression::Error::MalformedExpression,
				))?;
				let right = stack.pop_back().ok_or(Error::InvalidUnlockTree(
					expression::Error::MalformedExpression,
				))?;

				stack.push_back(Box::new(move || {
					let left_result = left();
					if !left_result {
						return false; // short-circuit
					}
					left_result && right()
				}));
			}
			(Op::Or, Op::Or) => {
				let left = stack.pop_back().ok_or(Error::InvalidUnlockTree(
					expression::Error::MalformedExpression,
				))?;
				let right = stack.pop_back().ok_or(Error::InvalidUnlockTree(
					expression::Error::MalformedExpression,
				))?;

				stack.push_back(Box::new(move || {
					let left_result = left();
					if left_result {
						return true; // short-circuit
					}
					left_result || right()
				}));
			}
			_ => {
				return Err(Error::InvalidUnlockTree(
					expression::Error::MalformedExpression,
				))
			}
		},
		(None, None) => return Ok(()),
		_ => return Err(Error::InvalidInstance),
	}

	if let Some(result_func) = stack.pop_back() {
		if result_func() {
			Ok(())
		} else {
			Err(Error::UnlockNotSatisfied(object, location))
		}
	} else {
		Err(Error::InvalidUnlockTree(
			expression::Error::MalformedExpression,
		))
	}
}

fn eval_object<'a, F, E: Environment>(
	object: &'a AsObject<Expanded>,
	location: Location,
	instance: AsObject<Executable<'a, F, E>>,
	transition: &'a Transition<Expanded>,
	env: &'a E,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	for (j, (instance, policy)) in instance
		.policies
		.into_iter()
		.zip(object.policies.iter())
		.enumerate()
	{
		let role = Role::Policy(policy, j);
		if !eval_predicate(
			policy, instance, location, role, object, transition, env,
		) {
			return Err(Error::PolicyNotSatisfied(object, location, j));
		}
	}

	eval_unlocks(transition, object, instance.unlock, location, env)
}

/// Checks wheter the policies of an object are satisfied.
///
/// It takes the transition where the object is located, the object that
/// contains the policy predicate, the instantiated object that has an
/// executable instance of the predicate and the location where the object is
/// located.
fn eval_policies<'a, F, E: Environment>(
	transition: &'a Transition<Expanded>,
	object: &'a AsObject<Expanded>,
	instance: AsObject<Executable<'a, F, E>>,
	location: Location,
	env: &'a E,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	for (j, (instance, policy)) in instance
		.policies
		.into_iter()
		.zip(object.policies.iter())
		.enumerate()
	{
		let role = Role::Policy(policy, j);
		if !eval_predicate(
			policy, instance, location, role, object, transition, env,
		) {
			return Err(Error::PolicyNotSatisfied(object, location, j));
		}
	}

	Ok(())
}

fn eval_predicate<'a, F, E: Environment>(
	predicate: &'a AtRest,
	instance: InUse<'a, F, E>,
	location: Location,
	role: Role<'a, AsPredicate<Expanded>>,
	object: &'a AsObject<Expanded>,
	transition: &'a Transition<Expanded>,
	env: &'a E,
) -> bool
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	instance.eval(
		Context {
			location,
			role,
			object,
			env,
		},
		transition,
		&predicate.params,
	)
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
