use {
	super::{Filter, Hot, IntoFilter},
	alloc::{
		string::{String, ToString},
		vec::Vec,
	},
	scale::{Decode, Encode},
};

#[derive(Clone, Debug)]
pub struct DataPattern<F: Filter = Hot> {
	filter: F,
	capture: Option<String>,
}

impl<F: Filter + PartialEq> PartialEq for DataPattern<F> {
	fn eq(&self, other: &Self) -> bool {
		self.filter == other.filter && self.capture == other.capture
	}
}

impl<F: Filter + Encode> Encode for DataPattern<F> {
	fn encode(&self) -> Vec<u8> {
		let mut result = alloc::vec![];
		result.extend_from_slice(&self.filter.encode());
		result.extend_from_slice(&self.capture.encode());
		result
	}
}

impl<F: Filter + Decode> scale::Decode for DataPattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let filter = F::decode(input)?;
		let capture = Option::<String>::decode(input)?;
		Ok(Self { filter, capture })
	}
}

impl<F: Filter> DataPattern<F> {
	pub fn new<T>(filter: impl IntoFilter<F, T>) -> Self {
		Self {
			filter: filter.into_filter(),
			capture: None,
		}
	}

	pub fn named<T>(
		name: impl AsRef<str>,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		Self {
			filter: filter.into_filter(),
			capture: Some(name.as_ref().to_string()),
		}
	}

	pub fn matches(&self, data: impl AsRef<[u8]>) -> bool {
		self.filter.matches(data.as_ref())
	}

	pub fn capture(&self, data: impl AsRef<[u8]>) -> Option<&str> {
		if let Some(ref capture) = self.capture {
			if self.matches(data.as_ref()) {
				return Some(capture.as_str());
			}
		}

		None
	}
}

pub trait IntoDataPattern<F: Filter> {
	fn into_data_pattern(self) -> DataPattern<F>;
}

impl<F: Filter> IntoDataPattern<F> for DataPattern<F> {
	fn into_data_pattern(self) -> DataPattern<F> {
		self
	}
}
