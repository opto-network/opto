use {
	crate::{Expression, Object, Op, Predicate, PredicateId},
	scale_decode::ToString,
	serde::{
		de::VariantAccess,
		ser::SerializeStruct,
		Deserialize,
		Deserializer,
		Serialize,
		Serializer,
	},
};

impl Serialize for Predicate {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let human_readable = serializer.is_human_readable();
		let mut at_rest = serializer.serialize_struct("Predicate", 2)?;
		at_rest.serialize_field("id", &self.id.0)?;
		if human_readable {
			at_rest.serialize_field(
				"params",
				&alloc::format!("0x{}", hex::encode(&self.params)),
			)?;
		} else {
			at_rest.serialize_field("params", &self.params)?;
		}
		at_rest.end()
	}
}

impl<'de> Deserialize<'de> for Predicate {
	fn deserialize<D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "snake_case")]
		enum Field {
			Id,
			Params,
		}

		struct Visitor {
			human_readable: bool,
		}

		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Predicate;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				formatter.write_str("struct Predicate")
			}

			fn visit_map<A: serde::de::MapAccess<'de>>(
				self,
				mut map: A,
			) -> Result<Self::Value, A::Error> {
				let mut id = None;
				let mut params = None;
				while let Some(key) = map.next_key()? {
					match key {
						Field::Id => {
							if id.is_some() {
								return Err(serde::de::Error::duplicate_field("id"));
							}
							id = Some(map.next_value()?);
						}
						Field::Params => {
							if params.is_some() {
								return Err(serde::de::Error::duplicate_field("params"));
							}
							if self.human_readable {
								let mut hexstr = map.next_value::<alloc::string::String>()?;
								if hexstr.starts_with("0x") {
									hexstr = hexstr[2..].to_string();
								}

								params =
									Some(hex::decode(hexstr).map_err(serde::de::Error::custom)?);
							} else {
								params = Some(map.next_value()?);
							}
						}
					}
				}
				let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
				let params =
					params.ok_or_else(|| serde::de::Error::missing_field("params"))?;

				Ok(Predicate {
					id: PredicateId(id),
					params,
				})
			}
		}

		let human_readable = deserializer.is_human_readable();
		deserializer.deserialize_struct("Predicate", &["id", "params"], Visitor {
			human_readable,
		})
	}
}

impl<P: Serialize, D: Serialize + AsRef<[u8]>> Serialize for Object<P, D> {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let is_human_readable = serializer.is_human_readable();
		let mut object = serializer.serialize_struct("Object", 3)?;
		object.serialize_field("policies", &self.policies)?;
		object.serialize_field("unlock", &self.unlock)?;
		if is_human_readable {
			object.serialize_field(
				"data",
				&alloc::format!("0x{}", hex::encode(&self.data)),
			)?;
		} else {
			object.serialize_field("data", &self.data)?;
		}
		object.end()
	}
}

impl<'de, P: Deserialize<'de>> Deserialize<'de>
	for Object<P, alloc::vec::Vec<u8>>
{
	fn deserialize<De: Deserializer<'de>>(
		deserializer: De,
	) -> Result<Self, De::Error> {
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "snake_case")]
		enum Field {
			Policies,
			Unlock,
			Data,
		}

		struct Visitor<P> {
			human_readable: bool,
			_phantom: core::marker::PhantomData<P>,
		}

		impl<'de, P: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<P> {
			type Value = Object<P, alloc::vec::Vec<u8>>;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				formatter.write_str("struct Object")
			}

			fn visit_map<A: serde::de::MapAccess<'de>>(
				self,
				mut map: A,
			) -> Result<Self::Value, A::Error> {
				let mut policies = None;
				let mut unlock = None;
				let mut data = None;
				while let Some(key) = map.next_key()? {
					match key {
						Field::Policies => {
							if policies.is_some() {
								return Err(serde::de::Error::duplicate_field("policies"));
							}
							policies = Some(map.next_value()?);
						}
						Field::Unlock => {
							if unlock.is_some() {
								return Err(serde::de::Error::duplicate_field("unlock"));
							}
							unlock = Some(map.next_value()?);
						}
						Field::Data => {
							if data.is_some() {
								return Err(serde::de::Error::duplicate_field("data"));
							}
							if self.human_readable {
								let mut hexstr = map.next_value::<alloc::string::String>()?;
								if hexstr.starts_with("0x") {
									hexstr = hexstr[2..].to_string();
								}

								data =
									Some(hex::decode(hexstr).map_err(serde::de::Error::custom)?);
							} else {
								data = Some(map.next_value()?);
							}
						}
					}
				}
				let policies = policies
					.ok_or_else(|| serde::de::Error::missing_field("policies"))?;
				let unlock =
					unlock.ok_or_else(|| serde::de::Error::missing_field("unlock"))?;
				let data =
					data.ok_or_else(|| serde::de::Error::missing_field("data"))?;

				Ok(Object {
					policies,
					unlock,
					data,
				})
			}
		}

		let human_readable = deserializer.is_human_readable();
		deserializer.deserialize_struct(
			"Object",
			&["policies", "unlock", "data"],
			Visitor {
				human_readable,
				_phantom: core::marker::PhantomData,
			},
		)
	}
}

impl<P: Serialize> Serialize for Op<P> {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		match self {
			Self::Predicate(p) => {
				serializer.serialize_newtype_variant("Op", 0, "predicate", p)
			}
			Self::And => serializer.serialize_unit_variant("Op", 1, "and"),
			Self::Or => serializer.serialize_unit_variant("Op", 2, "or"),
			Self::Not => serializer.serialize_unit_variant("Op", 3, "not"),
		}
	}
}

impl<'de, P: Deserialize<'de>> Deserialize<'de> for Op<P> {
	fn deserialize<D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "snake_case")]
		enum Field {
			Predicate,
			And,
			Or,
			Not,
		}

		struct Visitor<P> {
			_phantom: core::marker::PhantomData<P>,
		}

		impl<'de, P: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<P> {
			type Value = Op<P>;

			fn expecting(
				&self,
				formatter: &mut core::fmt::Formatter,
			) -> core::fmt::Result {
				formatter.write_str("enum Op")
			}

			fn visit_enum<A: serde::de::EnumAccess<'de>>(
				self,
				data: A,
			) -> Result<Self::Value, A::Error> {
				let op = match data.variant()? {
					(Field::Predicate, variant) => {
						let p = variant.newtype_variant()?;
						Op::Predicate(p)
					}
					(Field::And, _) => Op::And,
					(Field::Or, _) => Op::Or,
					(Field::Not, _) => Op::Not,
				};
				Ok(op)
			}
		}

		deserializer.deserialize_enum(
			"Op",
			&["Predicate", "And", "Or", "Not"],
			Visitor {
				_phantom: core::marker::PhantomData,
			},
		)
	}
}

impl<P: Serialize> Serialize for Expression<P> {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_seq(self.0.iter())
	}
}

impl<'de, P: Deserialize<'de>> Deserialize<'de> for Expression<P> {
	fn deserialize<D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		Ok(Expression(serde::de::Deserialize::deserialize(
			deserializer,
		)?))
	}
}

#[cfg(test)]
mod test {
	use {
		super::*,
		crate::{Predicate, PredicateId},
	};

	#[test]
	fn test_serde() {
		let pred1 = Predicate {
			id: PredicateId(1),
			params: vec![1, 2, 3],
		};

		let pred2: Expression<_> = Predicate {
			id: PredicateId(2),
			params: vec![4, 5, 6],
		}
		.into();

		let pred3: Expression<_> = Predicate {
			id: PredicateId(3),
			params: vec![7, 8, 9],
		}
		.into();

		let expr = pred2 & pred3;

		let object = Object {
			policies: vec![pred1],
			unlock: expr,
			data: vec![10u8, 20, 30],
		};
		let serialized = serde_json::to_string(&object).unwrap();

		assert_eq!(
			serialized,
			r#"{"policies":[{"id":1,"params":"0x010203"}],"unlock":["and",{"predicate":{"id":2,"params":"0x040506"}},{"predicate":{"id":3,"params":"0x070809"}}],"data":"0x0a141e"}"#
		);

		let deserialized: Object = serde_json::from_str(&serialized).unwrap();
		assert_eq!(object, deserialized);
	}
}
