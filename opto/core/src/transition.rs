use {
	crate::{
		alloc::{boxed::Box, collections::VecDeque, vec::Vec},
		digest::DigestBuilder,
		eval::{Context, InUse, Location, Role},
		expression::{self, Op},
		repr::{
			AsExpression,
			AsInput,
			AsObject,
			AsPredicate,
			Compact,
			Expanded,
			Repr,
		},
		AtRest,
		Digest,
		Hashable,
		Object,
	},
	core::fmt::Debug,
	scale::{Decode, Encode},
	scale_info::TypeInfo,
};

/// Represents a state transition.
///
/// This is the basic unit of execution in the system. It is computed
/// off-chain and validated on chain. For a state transition to be
/// valid all policy and unlock predicates on inputes and ephemerals
/// need to be satisfied and all policies on outputs need to be satisfied.
///
/// Input objects are consumed by the transition and are no longer
/// available after the transition is executed. Ephemeral objects are
/// created by the transition and are only available during the execution
/// of the transition. Output objects are created by the transition and
/// are available after the transition is executed.
pub struct Transition<R: Repr = Expanded> {
	pub inputs: Vec<R::InputObject>,
	pub ephemerals: Vec<AsObject<R>>,
	pub outputs: Vec<AsObject<R>>,
}

/// This is a representation of a state transition where all input objects
/// are fully expanded and available in the transition object. This
/// representation is used when predicates are evaluated, and is always tied
/// to an underlying machine that is executing the transition.
///
/// This representation requires a reference to the expanded representation
/// during evaluation. This representation is not serializable, clonable,
/// copiable or comparable.
pub struct Executable<'a, F>(core::marker::PhantomData<&'a F>)
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool;

impl<'a, F> Repr for Executable<'a, F>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	type Data = &'a [u8];
	type InputObject = AsObject<Self>;
	type Predicate = InUse<'a, F>;
}

impl<R: Repr> Encode for Transition<R>
where
	R::InputObject: Encode,
	R::Predicate: Encode,
	R::Data: Encode,
{
	fn size_hint(&self) -> usize {
		self.inputs.size_hint()
			+ self.ephemerals.size_hint()
			+ self.outputs.size_hint()
	}

	fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
		self.inputs.encode_to(dest);
		self.ephemerals.encode_to(dest);
		self.outputs.encode_to(dest);
	}
}

impl<R: Repr> Decode for Transition<R>
where
	R::InputObject: Decode,
	R::Predicate: Decode,
	R::Data: Decode,
{
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let inputs = Vec::<AsInput<R>>::decode(input)?;
		let ephemerals = Vec::<AsObject<R>>::decode(input)?;
		let outputs = Vec::<AsObject<R>>::decode(input)?;

		Ok(Transition {
			inputs,
			ephemerals,
			outputs,
		})
	}
}

impl<R: Repr> PartialEq for Transition<R>
where
	R::InputObject: PartialEq,
	R::Predicate: PartialEq,
	R::Data: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.inputs == other.inputs
			&& self.ephemerals == other.ephemerals
			&& self.outputs == other.outputs
	}
}

impl<R: Repr> Debug for Transition<R>
where
	R::InputObject: Debug,
	R::Predicate: Debug,
	R::Data: Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Transition")
			.field("inputs", &self.inputs)
			.field("ephemerals", &self.ephemerals)
			.field("outputs", &self.outputs)
			.finish()
	}
}

impl Eq for Transition<Expanded> {}

impl<P: Repr + Clone> Clone for Transition<P>
where
	P::InputObject: Clone,
	P::Predicate: Clone,
	P::Data: Clone,
{
	fn clone(&self) -> Self {
		Transition {
			inputs: self.inputs.clone(),
			ephemerals: self.ephemerals.clone(),
			outputs: self.outputs.clone(),
		}
	}
}

impl<P: Repr + TypeInfo + 'static> TypeInfo for Transition<P>
where
	P::InputObject: TypeInfo,
	P::Predicate: TypeInfo,
	P::Data: TypeInfo,
{
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("Transition", module_path!()))
			.composite(
				scale_info::build::Fields::named()
					.field(|f| f.ty::<Vec<P::InputObject>>().name("inputs"))
					.field(|f| f.ty::<Vec<AsObject<P>>>().name("ephemerals"))
					.field(|f| f.ty::<Vec<AsObject<P>>>().name("outputs")),
			)
	}
}

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

/// Evaluates the transition in the context of a state transition.
///
/// The `'m` lifetime is the lifetime of the Machine that is evaluating the
/// transition. The `'d` lifetime is the lifetime of the transition Data that is
/// being evaluated.
impl<'a, F> Transition<Executable<'a, F>>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	/// Evaluates the transition in the context of a state transition.
	pub fn evaluate(
		self,
		source: &'a Transition<Expanded>,
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
			eval_object(object, Location::Input, instance, source)?;
		}

		// Check that all ephemeral objects have their predicates satisfied.
		for (instance, object) in
			self.ephemerals.into_iter().zip(source.ephemerals.iter())
		{
			eval_object(object, Location::Ephemeral, instance, source)?;
		}

		// Check that all output objects have their predicates satisfied. For output
		// object we don't need to evaluate unlock conditions, only policies.
		for (instance, object) in
			self.outputs.into_iter().zip(source.outputs.iter())
		{
			eval_policies(source, object, instance, Location::Output)?;
		}

		// all checks passed, transition is valid
		Ok(())
	}
}

impl Transition<Expanded> {
	/// Given an expanded at-rest transition graph and a function that is able to
	/// create executable versions of the at-rest predicates this function will
	/// create an executable version of the transition that can be invoked and
	/// evaluated at runtime.
	pub fn instantiate<'a, B, F, E>(
		&'a self,
		factory: B,
	) -> Result<Transition<Executable<'a, F>>, E>
	where
		B: Fn(&'a AtRest) -> Result<F, E> + 'a,
		F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
	{
		Ok(Transition {
			inputs: self
				.inputs
				.iter()
				.map(|obj| {
					Ok(Object {
						policies: obj
							.policies
							.iter()
							.map(|p| factory(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(factory(u)?)))?,
						data: obj.data.as_slice(),
					})
				})
				.collect::<Result<_, _>>()?,
			ephemerals: self
				.ephemerals
				.iter()
				.map(|obj| {
					Ok(Object {
						policies: obj
							.policies
							.iter()
							.map(|p| factory(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(factory(u)?)))?,
						data: obj.data.as_slice(),
					})
				})
				.collect::<Result<_, _>>()?,
			outputs: self
				.outputs
				.iter()
				.map(|obj| {
					Ok(Object {
						policies: obj
							.policies
							.iter()
							.map(|p| factory(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(factory(u)?)))?,
						data: obj.data.as_slice(),
					})
				})
				.collect::<Result<_, _>>()?,
		})
	}

	/// Returns a hash of the state transition without ephemeral objects.
	pub fn digest_for_signing(&self) -> Digest {
		let mut hasher = Digest::hasher();

		// hash all inputs's hashes
		for obj in self.inputs.iter() {
			hasher.update(obj.digest());
		}

		// hash all outputs
		for obj in self.outputs.iter() {
			hasher.update(obj.digest());
		}

		// this is the message that will be used to verify the signature
		hasher.finalize().into()
	}
}

impl Transition<Compact> {
	/// Returns a hash of the state transition without ephemeral objects.
	pub fn digest_for_signing(&self) -> Digest {
		let mut hasher = Digest::hasher();

		// hash all inputs
		for obj in self.inputs.iter() {
			hasher.update(obj);
		}

		// hash all outputs
		for obj in self.outputs.iter() {
			hasher.update(obj.digest());
		}

		// this is the message that will be used to verify the signature
		hasher.finalize().into()
	}
}

fn eval_unlocks<'a, F>(
	source: &'a Transition<Expanded>,
	object: &'a AsObject<Expanded>,
	expression: AsExpression<Executable<'a, F>>,
	location: Location,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
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

fn eval_object<'a, F>(
	object: &'a AsObject<Expanded>,
	location: Location,
	instance: AsObject<Executable<'a, F>>,
	transition: &'a Transition<Expanded>,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	for (j, (instance, policy)) in instance
		.policies
		.into_iter()
		.zip(object.policies.iter())
		.enumerate()
	{
		let role = Role::Policy(policy, j);
		if !eval_predicate(policy, instance, location, role, object, transition) {
			return Err(Error::PolicyNotSatisfied(object, location, j));
		}
	}

	eval_unlocks(transition, object, instance.unlock, location)
}

/// Checks wheter the policies of an object are satisfied.
///
/// It takes the transition where the object is located, the object that
/// contains the policy predicate, the instantiated object that has an
/// executable instance of the predicate and the location where the object is
/// located.
fn eval_policies<'a, F>(
	transition: &'a Transition<Expanded>,
	object: &'a AsObject<Expanded>,
	instance: AsObject<Executable<'a, F>>,
	location: Location,
) -> Result<(), Error<'a>>
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	for (j, (instance, policy)) in instance
		.policies
		.into_iter()
		.zip(object.policies.iter())
		.enumerate()
	{
		let role = Role::Policy(policy, j);
		if !eval_predicate(policy, instance, location, role, object, transition) {
			return Err(Error::PolicyNotSatisfied(object, location, j));
		}
	}

	Ok(())
}

fn eval_predicate<'a, F>(
	predicate: &'a AtRest,
	instance: InUse<'a, F>,
	location: Location,
	role: Role<'a, AsPredicate<Expanded>>,
	object: &'a AsObject<Expanded>,
	transition: &'a Transition<Expanded>,
) -> bool
where
	F: FnOnce(Context<'a>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	instance.eval(
		Context {
			location,
			role,
			object,
		},
		transition,
		&predicate.params,
	)
}

fn index_of_unchecked<T>(slice: &[T], item: &T) -> usize {
	if core::mem::size_of::<T>() == 0 {
		return 0; // do what you will with this case
	}
	(item as *const T as *const () as usize
		- slice.as_ptr() as *const () as usize)
		/ core::mem::size_of::<T>()
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

#[cfg(test)]
mod tests {
	use {
		super::super::{object::tests::test_object, *},
		crate::transition::Expanded,
		alloc::vec::Vec,
		scale::{Decode, Encode},
	};

	#[test]
	fn transition_encode_decode_smoke() {
		let transition: Transition<Expanded> = Transition {
			inputs: vec![test_object(3), test_object(4)],
			ephemerals: vec![],
			outputs: vec![test_object(1), test_object(2)],
		};

		let size_hint = transition.size_hint();
		let mut encoded = Vec::with_capacity(size_hint);
		transition.encode_to(&mut encoded);
		assert!(encoded.len() <= size_hint);

		let decoded =
			Transition::<Expanded>::decode(&mut encoded.as_slice()).unwrap();
		assert_eq!(transition, decoded);
	}
}
