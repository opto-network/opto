use {
	crate::{
		alloc::vec::Vec,
		digest::DigestBuilder,
		env::Environment,
		eval::{Context, InUse},
		repr::{AsInput, AsObject, Compact, Executable, Expanded, Repr},
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

impl<R: Repr> Eq for Transition<R>
where
	R::InputObject: Eq,
	R::Predicate: Eq,
	R::Data: Eq,
{
}

impl<R: Repr> core::hash::Hash for Transition<R>
where
	R::InputObject: core::hash::Hash,
	R::Predicate: core::hash::Hash,
	R::Data: core::hash::Hash,
	Object<<R as Repr>::Predicate, <R as Repr>::Data>: core::hash::Hash,
{
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.inputs.hash(state);
		self.ephemerals.hash(state);
		self.outputs.hash(state);
	}
}

impl<R: Repr> Debug for Transition<R>
where
	R::InputObject: Debug,
	R::Predicate: Debug,
	R::Data: AsRef<[u8]> + Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Transition")
			.field("inputs", &self.inputs)
			.field("ephemerals", &self.ephemerals)
			.field("outputs", &self.outputs)
			.finish()
	}
}

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

impl Transition<Expanded> {
	/// Given an expanded at-rest transition graph and a function that is able to
	/// create executable versions of the at-rest predicates this function will
	/// create an executable version of the transition that can be invoked and
	/// evaluated at runtime.
	pub fn instantiate<'a, B, F, E, Env: Environment + 'a>(
		&'a self,
		builder: B,
	) -> Result<Transition<Executable<'a, F, Env>>, E>
	where
		B: Fn(&'a AtRest) -> Result<F, E> + 'a,
		F: FnOnce(Context<'a, Env>, &'a Transition<Expanded>, &'a [u8]) -> bool,
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
							.map(|p| builder(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(builder(u)?)))?,
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
							.map(|p| builder(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(builder(u)?)))?,
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
							.map(|p| builder(p).map(|e| InUse::new(e)))
							.collect::<Result<_, _>>()?,
						unlock: obj
							.unlock
							.as_ref()
							.try_map(|u| Ok(InUse::new(builder(u)?)))?,
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
