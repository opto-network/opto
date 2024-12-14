use {
	crate::NftIdentity,
	opto::{
		ext::{CompactTransitionExt, ExpressionExt},
		stdpred,
		AccountId32,
		Client,
		Digest,
		Expression,
		Hashable,
		Object,
		Predicate,
		ReadOnlyClient,
		Transition,
	},
	scale::Encode,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("client error: {0}")]
	Client(#[from] opto::client::Error),

	#[error("Mint object has invalid tag")]
	InvalidMintTag,

	#[error("Mint not found for the given unique tag")]
	MintNotFound,

	#[error("Missing uniqueness policy on mint object")]
	MissingUniqueness,

	#[error("Invalid uniqueness policy on mint object")]
	InvalidUniqueness,

	#[error("Not a mint object")]
	NotAMintObject,

	#[error("Mint already exists")]
	MintAlreadyExists,
}

#[derive(Debug, Clone)]
pub struct Mint {
	object: Object,
	tag: Digest,
}

impl Mint {
	pub async fn fetch(client: &Client, tag: &Digest) -> Result<Self, Error> {
		if let Some(object) = client.unique(tag).await? {
			object.try_into()
		} else {
			Err(Error::MintNotFound)
		}
	}

	pub fn issue(&self, tag: Digest) -> NftBuilder {
		NftBuilder {
			mint: self,
			tag,
			nonce: true,
			unlock: self.object.unlock.clone(),
			mutable: false,
			data: Vec::new(),
		}
	}
}

impl TryFrom<Object> for Mint {
	type Error = Error;

	fn try_from(object: Object) -> Result<Self, Self::Error> {
		if let Some(mint_predicate) = object
			.policies
			.iter()
			.find(|p| p.id == crate::policies::ids::NFT_MINT)
		{
			if mint_predicate.params.len() != Digest::SIZE {
				return Err(Error::InvalidMintTag);
			}

			if let Some(uniqueness) = object
				.policies
				.iter()
				.find(|p| p.id == stdpred::ids::UNIQUE)
			{
				if uniqueness.params != mint_predicate.params {
					return Err(Error::InvalidUniqueness);
				}
			} else {
				return Err(Error::MissingUniqueness);
			}

			let tag: Digest = mint_predicate
				.params
				.as_slice()
				.try_into()
				.map_err(|_| Error::InvalidUniqueness)?;

			Ok(Self { object, tag })
		} else {
			Err(Error::NotAMintObject)
		}
	}
}

pub struct MintBuilder(Object);

impl MintBuilder {
	pub fn new(identity: Digest) -> Self {
		let policies = vec![
			Predicate {
				id: crate::policies::ids::NFT_MINT,
				params: identity.to_vec(),
			},
			Predicate {
				id: stdpred::ids::UNIQUE,
				params: identity.to_vec(),
			},
		];

		Self(Object {
			policies,
			unlock: Expression::constant(true),
			data: Vec::new(),
		})
	}

	pub fn with_owner(mut self, owner: AccountId32) -> Self {
		self.0.unlock = Expression::signature(&owner);
		self
	}

	pub fn with_unlock(mut self, expression: Expression) -> Self {
		self.0.unlock = expression;
		self
	}

	pub fn with_data(mut self, data: Vec<u8>) -> Self {
		self.0.data = data;
		self
	}

	pub fn immutable(mut self, value: bool) -> Self {
		if let Some(policy) = self
			.0
			.policies
			.iter_mut()
			.find(|p| p.id == stdpred::ids::CONSTANT)
		{
			policy.params = vec![value as u8];
		} else {
			self.0.policies.push(Predicate {
				id: stdpred::ids::CONSTANT,
				params: vec![value as u8],
			});
		}

		self
	}

	pub fn build(self) -> Object {
		self.0
	}
}

pub struct NftBuilder<'m> {
	mint: &'m Mint,
	tag: Digest,
	unlock: Expression,
	mutable: bool,
	nonce: bool,
	data: Vec<u8>,
}

impl NftBuilder<'_> {
	pub fn mutable(mut self, value: bool) -> Self {
		self.mutable = value;
		self
	}

	pub fn unlock(mut self, expression: Expression) -> Self {
		self.unlock = expression;
		self
	}

	pub fn recipient(mut self, recipient: &AccountId32) -> Self {
		self.unlock = Expression::signature(recipient);
		self
	}

	pub fn password(mut self, password: &[u8]) -> Self {
		self.unlock = Expression::preimage(password);
		self
	}

	pub fn data(mut self, data: Vec<u8>) -> Self {
		self.data = data;
		self
	}

	pub fn nonce(mut self, value: bool) -> Self {
		self.nonce = value;
		self
	}

	pub fn object(self) -> Object {
		let mut object = Object {
			policies: vec![
				Predicate {
					id: crate::policies::ids::NFT,
					params: NftIdentity {
						mint: self.mint.tag,
						tag: self.tag,
						mutable: self.mutable,
					}
					.encode()
					.to_vec(),
				},
				Predicate {
					id: stdpred::ids::UNIQUE,
					params: Digest::compute_concat(&[
						self.mint.tag.as_slice(),
						self.tag.as_slice(),
					])
					.to_vec(),
				},
			],
			unlock: self.unlock,
			data: self.data,
		};

		if self.nonce {
			object.policies.push(Predicate {
				id: stdpred::ids::NONCE,
				params: 0u64.to_le_bytes().to_vec(),
			});
		}

		object
	}

	pub fn transition(self) -> Transition {
		let has_nonce = self.nonce;
		let mut transition = Transition {
			inputs: vec![self.mint.object.digest()],
			ephemerals: vec![],
			outputs: vec![self.mint.object.clone(), self.object()],
		};

		if has_nonce {
			transition = transition.set_nonces();
		}

		transition
	}
}
