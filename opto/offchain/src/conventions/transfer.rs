use {
	super::{CoinBalance, CoinOwner},
	crate::{
		conventions::coin::CoinAsset,
		ext::CompactTransitionExt,
		AccountId,
		AssetId,
		Balance,
	},
	derive_more::derive::{Deref, From, Into},
	opto_core::*,
	scale::Encode,
	std::collections::HashMap,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error<'a> {
	/// The input objects list is empty.
	/// No transfer are going to be possible.
	NoInputObjects,

	/// The input object is not recognized as a conventional coin object.
	/// It could be a valid coin object but not a conventional one and we don't
	/// support it. E.g. might have few extra policies or different unlock than
	/// this convention.
	InvalidInputObject(&'a Object),

	/// The input objects have different asset ids.
	/// All inputs must be of the same coin type.
	///
	/// This error will carry the object that caused the error and the asset id
	/// that was expected.
	///
	/// The expected asset id is the asset id of the first object in the list.
	DifferentAssetId(&'a Object, AssetId),

	/// The input objects have different signers.
	/// All inputs must be controlled by the same account.
	///
	/// This error will carry the object that caused the error and the account id
	/// that was expected.
	///
	/// The expected account id is the account id of the first object in the
	/// list.
	DifferentSigners(&'a Object, AccountId),

	/// The output balance is greater than the input balance for a given coin
	/// type.
	InsufficientInputsBalance,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, From, Into, Deref)]
struct HashableAccountId(AccountId);

impl std::hash::Hash for HashableAccountId {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0 .0.hash(state);
	}
}

#[derive(Debug, Clone)]
pub struct CoinTransfer<'a> {
	inputs: Vec<&'a Object>,
	beneficiaries: HashMap<HashableAccountId, u128>,
}

impl<'a> CoinTransfer<'a> {
	pub fn from_inputs(
		objects: impl Iterator<Item = &'a Object>,
	) -> Result<Self, Error<'a>> {
		let mut inputs = Vec::new();

		let mut seen_signer: Option<CoinOwner> = None;
		let mut seen_asset_id: Option<CoinAsset> = None;

		for input in objects {
			// how much
			let _ = CoinBalance::try_from(input)
				.map_err(|_| Error::InvalidInputObject(input))?;

			// of what
			let asset_id = CoinAsset::try_from(input)
				.map_err(|_| Error::InvalidInputObject(input))?;

			// who controls it
			let owner = CoinOwner::try_from(input)
				.map_err(|_| Error::InvalidInputObject(input))?;

			// ensure they are all the same coin type
			if let Some(ref seen_asset_id) = seen_asset_id {
				if *seen_asset_id != asset_id {
					return Err(Error::DifferentAssetId(input, **seen_asset_id));
				}
			} else {
				seen_asset_id = Some(asset_id);
			}

			// ensure they are all controlled by the same account
			if let Some(ref seen_signer) = seen_signer {
				if *seen_signer != owner {
					return Err(Error::DifferentSigners(input, (**seen_signer).clone()));
				}
			} else {
				seen_signer = Some(owner);
			}

			inputs.push(input);
		}

		if inputs.is_empty() {
			return Err(Error::NoInputObjects);
		}

		Ok(Self {
			inputs,
			beneficiaries: HashMap::new(),
		})
	}

	/// Adds a new beneficiary to the transfer.
	///
	/// This function will check if the transfer amount is greater than the
	/// remaining balance of the input objects minus the committed balances of
	/// the other beneficiaries.
	///
	/// Note that the sum of all beneficies' amounts all input balances may
	/// overflow the maximum amount type can hold.
	///
	/// This function may be called multiple times to add multiple beneficiaries.
	/// If called multiple times with the same beneficiary, the amounts will be
	/// summed.
	pub fn add_beneficiary(
		mut self,
		beneficiary: AccountId,
		amount: Balance,
	) -> Result<Self, Error<'a>> {
		let amount = amount as u128;

		let available_balance = self
			.inputs
			.iter()
			.map(|input| {
				*CoinBalance::try_from(*input) //
					.expect("validated at construction") as u128
			})
			.sum::<u128>();

		let commited_balance = self.beneficiaries.values().copied().sum::<u128>();
		let total_balance = available_balance - commited_balance;

		if total_balance < amount {
			return Err(Error::InsufficientInputsBalance);
		}

		*self
			.beneficiaries
			.entry(HashableAccountId(beneficiary))
			.or_default() += amount;

		Ok(self)
	}

	/// Produce a state transition that will transfer coins from input objects
	/// into the beneficiaries, ensuring that there are as few output objects
	/// as possible.
	///
	/// The resulting transition is not signed and needs to be signed by the
	/// owner of the input coins account.
	///
	/// The resulting transition may not consume all input objects.
	pub fn transition(self) -> Result<Transition, Error<'a>> {
		let mut outputs = Vec::new();

		let asset_id = *CoinAsset::try_from(self.inputs[0]) //
			.expect("validated at construction");

		let sender_account = CoinOwner::try_from(self.inputs[0]) //
			.expect("validated at construction");

		let mut total_commitments = 0u128;
		for (account, amount) in &self.beneficiaries {
			// create a new output object for each beneficiary. If the amount is
			// greater than u64::max then break it down into multiple objects.
			let mut amount = *amount;
			total_commitments += amount;

			while amount > 0 {
				let output_amount = amount.min(u64::MAX as u128) as u64;
				amount -= output_amount as u128;

				let output = Object {
					policies: vec![
						opto_stdpred::ids::COIN.params(asset_id),
						opto_stdpred::ids::NONCE.into(),
					],
					unlock: opto_stdpred::ids::SR25519.params(&account.0).into(),
					data: output_amount.encode(),
				};

				outputs.push(output);
			}
		}

		// consume as many input objects as possible to satisfy the output
		// objects. If there are remaining input objects, they will be left
		// unspent and not included in the transition.
		let mut remaining_commitments = total_commitments;

		// drain smaller inputs first
		let mut inputs = self.inputs.into_iter().collect::<Vec<_>>();
		inputs.sort_by_key(|input| {
			*CoinBalance::try_from(*input) //
				.expect("validated at construction")
		});

		let mut consumed = Vec::new();

		while let Some(next) = inputs.pop() {
			let balance = *CoinBalance::try_from(next) //
				.expect("validated at construction") as u128;

			if balance <= remaining_commitments {
				remaining_commitments -= balance;
				consumed.push(next);
				continue;
			} else {
				inputs.push(next);

				let remainder = balance - remaining_commitments;

				// add the remainder as an output that is controlled by the same
				// account as the sender.
				let output = Object {
					policies: vec![
						opto_stdpred::ids::COIN.params(asset_id),
						opto_stdpred::ids::NONCE.into(),
					],
					unlock: opto_stdpred::ids::SR25519.params(sender_account.0).into(),
					data: remainder.encode(),
				};

				outputs.push(output);

				break;
			}
		}

		Ok(
			Transition::<Compact> {
				inputs: consumed.into_iter().map(|o| o.digest()).collect(),
				outputs,
				ephemerals: Vec::new(),
			}
			.set_nonces(),
		)
	}
}

impl<'a> TryFrom<CoinTransfer<'a>> for Transition {
	type Error = Error<'a>;

	fn try_from(value: CoinTransfer<'a>) -> Result<Self, Self::Error> {
		value.transition()
	}
}
