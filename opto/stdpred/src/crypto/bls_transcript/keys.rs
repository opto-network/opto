use {
	super::hash_to_g2,
	blstrs::{G1Affine, G1Projective, G2Projective, Scalar},
	core::str::FromStr,
	group::prime::PrimeCurveAffine,
	scale::{Decode, Encode, Error, Input, Output},
};

#[derive(Clone, PartialEq, Eq)]
pub struct SecretKey(pub(crate) Scalar);

impl SecretKey {
	pub const SIZE: usize = 32;

	#[cfg(any(test, feature = "std"))]
	pub fn generate(mut rng: impl rand::RngCore) -> Self {
		use group::ff::PrimeField;
		/// The number of bits we should "shave" from a randomly sampled reputation.
		const REPR_SHAVE_BITS: usize = 256 - Scalar::NUM_BITS as usize;

		loop {
			let mut raw = [0u64; 4];
			for int in raw.iter_mut() {
				*int = rng.next_u64();
			}

			// Mask away the unused most-significant bits.
			raw[3] &= 0xffffffffffffffff >> REPR_SHAVE_BITS;

			if let Some(scalar) = Scalar::from_u64s_le(&raw).into_option() {
				return SecretKey(scalar);
			}
		}
	}

	pub fn public_key(&self) -> PublicKey {
		PublicKey(G1Affine::generator() * self.0)
	}

	pub fn sign(&self, message: &[u8]) -> Signature {
		Signature(hash_to_g2(message) * self.0)
	}
}

impl FromStr for SecretKey {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.strip_prefix("0x").unwrap_or(s);

		let mut buffer = [0; Self::SIZE];
		let bytes = hex::decode(s).map_err(FromStrError::InvalidHex)?;

		if bytes.len() != Self::SIZE {
			return Err(FromStrError::InvalidLength(Self::SIZE, bytes.len()));
		}

		buffer.copy_from_slice(&bytes);
		Scalar::from_bytes_le(&buffer)
			.into_option()
			.map(Self)
			.ok_or(FromStrError::InvalidValue)
	}
}

impl core::fmt::Debug for SecretKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let bytes = self.0.to_bytes_le();
		write!(
			f,
			"Secret(0x{}..{})",
			hex::encode(&bytes[..1]),
			hex::encode(&bytes[31..])
		)
	}
}

impl core::fmt::Display for SecretKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let bytes = self.0.to_bytes_le();
		write!(
			f,
			"0x{}..{}",
			hex::encode(&bytes[..1]),
			hex::encode(&bytes[31..])
		)
	}
}

impl TryFrom<&[u8]> for SecretKey {
	type Error = Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		Self::decode(&mut &bytes[..])
	}
}

impl From<&SecretKey> for PublicKey {
	fn from(sk: &SecretKey) -> PublicKey {
		sk.public_key()
	}
}

impl From<&SecretKey> for [u8; 32] {
	fn from(sk: &SecretKey) -> [u8; 32] {
		sk.0.to_bytes_le()
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum FromStrError {
	// (expected, actual)
	InvalidLength(usize, usize),

	InvalidHex(hex::FromHexError),

	InvalidValue,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PublicKey(pub(crate) G1Projective);

impl PublicKey {
	pub const SIZE: usize = 48;
}

impl From<&PublicKey> for [u8; 48] {
	fn from(value: &PublicKey) -> Self {
		value.0.to_compressed()
	}
}

impl core::fmt::Debug for PublicKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Public(0x{})", hex::encode(self.0.to_compressed()),)
	}
}

impl core::fmt::Display for PublicKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "0x{}", hex::encode(self.0.to_compressed()),)
	}
}

impl TryFrom<&[u8]> for PublicKey {
	type Error = Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		Self::decode(&mut &bytes[..])
	}
}

impl FromStr for PublicKey {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.strip_prefix("0x").unwrap_or(s);

		let mut buffer = [0; Self::SIZE];
		let bytes = hex::decode(s).map_err(FromStrError::InvalidHex)?;

		if bytes.len() != Self::SIZE {
			return Err(FromStrError::InvalidLength(Self::SIZE, bytes.len()));
		}

		buffer.copy_from_slice(&bytes);
		G1Projective::from_compressed(&buffer)
			.into_option()
			.map(Self)
			.ok_or(FromStrError::InvalidValue)
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct Signature(pub(crate) G2Projective);

impl Signature {
	pub const SIZE: usize = 96;
}

impl FromStr for Signature {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.strip_prefix("0x").unwrap_or(s);

		let mut buffer = [0; Self::SIZE];
		let bytes = hex::decode(s).map_err(FromStrError::InvalidHex)?;

		if bytes.len() != Self::SIZE {
			return Err(FromStrError::InvalidLength(Self::SIZE, bytes.len()));
		}

		buffer.copy_from_slice(&bytes);
		G2Projective::from_compressed(&buffer)
			.into_option()
			.map(Self)
			.ok_or(FromStrError::InvalidValue)
	}
}

impl TryFrom<&[u8]> for Signature {
	type Error = Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		Self::decode(&mut &bytes[..])
	}
}

impl From<&Signature> for [u8; 96] {
	fn from(value: &Signature) -> Self {
		value.0.to_compressed()
	}
}

impl core::fmt::Debug for Signature {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Signature(0x{})", hex::encode(self.0.to_compressed()),)
	}
}

impl core::fmt::Display for Signature {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "0x{}", hex::encode(self.0.to_compressed()),)
	}
}

impl Encode for SecretKey {
	fn encoded_size(&self) -> usize {
		32 // BLS Secret Key size
	}

	fn size_hint(&self) -> usize {
		32
	}

	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		dest.write(&self.0.to_bytes_le());
	}
}

impl Decode for SecretKey {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let mut bytes = [0u8; 32];
		input.read(&mut bytes)?;
		Ok(Self(
			Scalar::from_bytes_le(&bytes)
				.into_option()
				.ok_or("invalid secret key")?,
		))
	}
}

impl Encode for PublicKey {
	fn encoded_size(&self) -> usize {
		48 // BLS Public Key size
	}

	fn size_hint(&self) -> usize {
		48
	}

	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		dest.write(&self.0.to_compressed());
	}
}

impl Decode for PublicKey {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let mut bytes = [0u8; 48];
		input.read(&mut bytes)?;
		Ok(Self(
			G1Projective::from_compressed(&bytes)
				.into_option()
				.ok_or_else(|| Error::from("invalid public key"))?,
		))
	}
}

impl Encode for Signature {
	fn encoded_size(&self) -> usize {
		96 // BLS Signature size
	}

	fn size_hint(&self) -> usize {
		96 // BLS Signature size
	}

	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		dest.write(&self.0.to_compressed());
	}
}

impl Decode for Signature {
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		let mut bytes = [0u8; 96];
		input.read(&mut bytes)?;
		Ok(Self(
			G2Projective::from_compressed(&bytes)
				.into_option()
				.ok_or("invalid signature")?,
		))
	}
}
