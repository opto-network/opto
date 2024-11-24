#![allow(clippy::just_underscores_and_digits)]

//! Based on macro expansion from `subxt codegen` cli command.
use crate::{repr::{Compact, Expanded}, Expression, Object, Op, Transition};

const _: () = {
	pub struct Visitor<_0, _1, ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		::core::marker::PhantomData<(_0, _1, ScaleDecodeTypeResolver)>,
	);
	use scale_decode::ToString;
	impl<_0, _1> scale_decode::IntoVisitor for Object<_0, _1>
	where
		_0: scale_decode::IntoVisitor,
		_1: scale_decode::IntoVisitor,
	{
		type AnyVisitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver> =
			Visitor<_0, _1, ScaleDecodeTypeResolver>;

		fn into_visitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		) -> Self::AnyVisitor<ScaleDecodeTypeResolver> {
			Visitor(::core::marker::PhantomData)
		}
	}
	impl<_0, _1, ScaleDecodeTypeResolver: scale_decode::TypeResolver>
		scale_decode::Visitor for Visitor<_0, _1, ScaleDecodeTypeResolver>
	where
		_0: scale_decode::IntoVisitor,
		_1: scale_decode::IntoVisitor,
	{
		type Error = scale_decode::Error;
		type TypeResolver = ScaleDecodeTypeResolver;
		type Value<'scale, 'info> = Object<_0, _1>;

		fn visit_composite<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Composite<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.has_unnamed_fields() {
				return self.visit_tuple(&mut value.as_tuple(), type_id);
			}
			let vals: scale_decode::BTreeMap<Option<&str>, _> = value
				.map(|res| res.map(|item| (item.name(), item)))
				.collect::<Result<_, _>>()?;
			Ok(Object {
				policies: {
					let val = vals
						.get(&Some("policies"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "policies".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("policies"))?
				},
				unlock: {
					let val = vals
						.get(&Some("unlock"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "unlock".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("unlock"))?
				},
				data: {
					let val = vals
						.get(&Some("data"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "data".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("data"))?
				},
			})
		}

		fn visit_tuple<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Tuple<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 3usize {
				return Err(scale_decode::Error::new(
					scale_decode::error::ErrorKind::WrongLength {
						actual_len: value.remaining(),
						expected_len: 3usize,
					},
				));
			}
			let vals = value;
			Ok(Object {
				policies: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("policies"))?
				},
				unlock: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("unlock"))?
				},
				data: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("data"))?
				},
			})
		}
	}
	impl<_0, _1> scale_decode::DecodeAsFields for Object<_0, _1>
	where
		_0: scale_decode::IntoVisitor,
		_1: scale_decode::IntoVisitor,
	{
		fn decode_as_fields<'info, R: scale_decode::TypeResolver>(
			input: &mut &[u8],
			fields: &mut dyn scale_decode::FieldIter<'info, R::TypeId>,
			types: &'info R,
		) -> Result<Self, scale_decode::Error> {
			let mut composite = scale_decode::visitor::types::Composite::new(
				core::iter::empty(),
				input,
				fields,
				types,
				false,
			);
			use scale_decode::{IntoVisitor, Visitor};
			let val = <Object<_0, _1>>::into_visitor()
				.visit_composite(&mut composite, Default::default());
			composite.skip_decoding()?;
			*input = composite.bytes_from_undecoded();
			val.map_err(From::from)
		}
	}
};
impl<_0, _1> scale_encode::EncodeAsType for Object<_0, _1>
where
	_0: scale_encode::EncodeAsType,
	_1: scale_encode::EncodeAsType,
{
	#[allow(unused_variables)]
	fn encode_as_type_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_type_id: ScaleEncodeResolver::TypeId,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Object {
			policies,
			unlock,
			data,
		} = self;
		scale_encode::Composite::new(
			[
				(
					Some("policies"),
					scale_encode::CompositeField::new(policies),
				),
				(Some("unlock"), scale_encode::CompositeField::new(unlock)),
				(Some("data"), scale_encode::CompositeField::new(data)),
			]
			.into_iter(),
		)
		.encode_composite_as_type_to(
			__encode_as_type_type_id,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}
impl<_0, _1> scale_encode::EncodeAsFields for Object<_0, _1>
where
	_0: scale_encode::EncodeAsType,
	_1: scale_encode::EncodeAsType,
{
	#[allow(unused_variables)]
	fn encode_as_fields_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_fields: &mut dyn scale_encode::FieldIter<
			'_,
			ScaleEncodeResolver::TypeId,
		>,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Object {
			policies,
			unlock,
			data,
		} = self;
		scale_encode::Composite::new(
			[
				(
					Some("policies"),
					scale_encode::CompositeField::new(policies),
				),
				(Some("unlock"), scale_encode::CompositeField::new(unlock)),
				(Some("data"), scale_encode::CompositeField::new(data)),
			]
			.into_iter(),
		)
		.encode_composite_fields_to(
			__encode_as_type_fields,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}

const _: () = {
	pub struct Visitor<_0, ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		::core::marker::PhantomData<(_0, ScaleDecodeTypeResolver)>,
	);

	impl<_0> scale_decode::IntoVisitor for Expression<_0>
	where
		_0: scale_decode::IntoVisitor,
	{
		type AnyVisitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver> =
			Visitor<_0, ScaleDecodeTypeResolver>;

		fn into_visitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		) -> Self::AnyVisitor<ScaleDecodeTypeResolver> {
			Visitor(::core::marker::PhantomData)
		}
	}
	impl<_0, ScaleDecodeTypeResolver: scale_decode::TypeResolver>
		scale_decode::Visitor for Visitor<_0, ScaleDecodeTypeResolver>
	where
		_0: scale_decode::IntoVisitor,
	{
		type Error = scale_decode::Error;
		type TypeResolver = ScaleDecodeTypeResolver;
		type Value<'scale, 'info> = Expression<_0>;

		fn visit_composite<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Composite<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			self.visit_tuple(&mut value.as_tuple(), type_id)
		}

		fn visit_tuple<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Tuple<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 1usize {
				return Err(scale_decode::Error::new(
					scale_decode::error::ErrorKind::WrongLength {
						actual_len: value.remaining(),
						expected_len: 1usize,
					},
				));
			}
			let vals = value;
			Ok(Expression({
				let val = vals.next().expect(
					"field count should have been checked already on tuple type; please \
					 file a bug report",
				)?;
				val.decode_as_type().map_err(|e| e.at_idx(0usize))?
			}))
		}
	}
	impl<_0> scale_decode::DecodeAsFields for Expression<_0>
	where
		_0: scale_decode::IntoVisitor,
	{
		fn decode_as_fields<'info, R: scale_decode::TypeResolver>(
			input: &mut &[u8],
			fields: &mut dyn scale_decode::FieldIter<'info, R::TypeId>,
			types: &'info R,
		) -> Result<Self, scale_decode::Error> {
			let mut composite = scale_decode::visitor::types::Composite::new(
				core::iter::empty(),
				input,
				fields,
				types,
				false,
			);
			use scale_decode::{IntoVisitor, Visitor};
			let val = <Expression<_0>>::into_visitor()
				.visit_composite(&mut composite, Default::default());
			composite.skip_decoding()?;
			*input = composite.bytes_from_undecoded();
			val.map_err(From::from)
		}
	}
};
impl<_0> scale_encode::EncodeAsType for Expression<_0>
where
	_0: scale_encode::EncodeAsType,
{
	#[allow(unused_variables)]
	fn encode_as_type_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_type_id: ScaleEncodeResolver::TypeId,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Expression(_0) = self;
		scale_encode::Composite::new(
			[(
				None as Option<&'static str>,
				scale_encode::CompositeField::new(_0),
			)]
			.into_iter(),
		)
		.encode_composite_as_type_to(
			__encode_as_type_type_id,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}
impl<_0> scale_encode::EncodeAsFields for Expression<_0>
where
	_0: scale_encode::EncodeAsType,
{
	#[allow(unused_variables)]
	fn encode_as_fields_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_fields: &mut dyn scale_encode::FieldIter<
			'_,
			ScaleEncodeResolver::TypeId,
		>,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Expression(_0) = self;
		scale_encode::Composite::new(
			[(
				None as Option<&'static str>,
				scale_encode::CompositeField::new(_0),
			)]
			.into_iter(),
		)
		.encode_composite_fields_to(
			__encode_as_type_fields,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}

const _: () = {
	pub struct Visitor<_0, ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		::core::marker::PhantomData<(_0, ScaleDecodeTypeResolver)>,
	);
	use scale_decode::ToString;
	impl<_0> scale_decode::IntoVisitor for Op<_0>
	where
		_0: scale_decode::IntoVisitor,
	{
		type AnyVisitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver> =
			Visitor<_0, ScaleDecodeTypeResolver>;

		fn into_visitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		) -> Self::AnyVisitor<ScaleDecodeTypeResolver> {
			Visitor(::core::marker::PhantomData)
		}
	}
	impl<_0, ScaleDecodeTypeResolver: scale_decode::TypeResolver>
		scale_decode::Visitor for Visitor<_0, ScaleDecodeTypeResolver>
	where
		_0: scale_decode::IntoVisitor,
	{
		type Error = scale_decode::Error;
		type TypeResolver = ScaleDecodeTypeResolver;
		type Value<'scale, 'info> = Op<_0>;

		fn visit_variant<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Variant<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.name() == "Predicate" {
				let fields = value.fields();
				if fields.remaining() != 1usize {
					return Err(scale_decode::Error::new(
						scale_decode::error::ErrorKind::WrongLength {
							actual_len: fields.remaining(),
							expected_len: 1usize,
						},
					));
				}
				let vals = fields;
				return Ok(Op::Predicate({
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_idx(0usize))?
				}));
			}
			if value.name() == "And" {
				return Ok(Op::And);
			}
			if value.name() == "Or" {
				return Ok(Op::Or);
			}
			if value.name() == "Not" {
				return Ok(Op::Not);
			}
			Err(scale_decode::Error::new(
				scale_decode::error::ErrorKind::CannotFindVariant {
					got: value.name().to_string(),
					expected: <[_]>::into_vec(alloc::boxed::Box::new([
						"Predicate",
						"And",
						"Or",
						"Not",
					])),
				},
			))
		}

		fn visit_composite<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Composite<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 1 {
				return self
					.visit_unexpected(scale_decode::visitor::Unexpected::Composite);
			}
			value.decode_item(self).unwrap()
		}

		fn visit_tuple<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Tuple<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 1 {
				return self.visit_unexpected(scale_decode::visitor::Unexpected::Tuple);
			}
			value.decode_item(self).unwrap()
		}
	}
};
impl<_0> scale_encode::EncodeAsType for Op<_0>
where
	_0: scale_encode::EncodeAsType,
{
	#[allow(unused_variables)]
	fn encode_as_type_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_type_id: ScaleEncodeResolver::TypeId,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		match self {
			Self::Predicate(_0) => scale_encode::Variant {
				name: "Predicate",
				fields: scale_encode::Composite::new(
					[(
						None as Option<&'static str>,
						scale_encode::CompositeField::new(_0),
					)]
					.into_iter(),
				),
			}
			.encode_variant_as_type_to(
				__encode_as_type_type_id,
				__encode_as_type_types,
				__encode_as_type_out,
			),
			Self::And => scale_encode::Variant {
				name: "And",
				fields: scale_encode::Composite::new(
					([] as [(Option<&'static str>, scale_encode::CompositeField<_>); 0])
						.into_iter(),
				),
			}
			.encode_variant_as_type_to(
				__encode_as_type_type_id,
				__encode_as_type_types,
				__encode_as_type_out,
			),
			Self::Or => scale_encode::Variant {
				name: "Or",
				fields: scale_encode::Composite::new(
					([] as [(Option<&'static str>, scale_encode::CompositeField<_>); 0])
						.into_iter(),
				),
			}
			.encode_variant_as_type_to(
				__encode_as_type_type_id,
				__encode_as_type_types,
				__encode_as_type_out,
			),
			Self::Not => scale_encode::Variant {
				name: "Not",
				fields: scale_encode::Composite::new(
					([] as [(Option<&'static str>, scale_encode::CompositeField<_>); 0])
						.into_iter(),
				),
			}
			.encode_variant_as_type_to(
				__encode_as_type_type_id,
				__encode_as_type_types,
				__encode_as_type_out,
			),
		}
	}
}

const _: () = {
	pub struct Visitor<
		ScaleDecodeTypeResolver: scale_decode::TypeResolver,
	>(::core::marker::PhantomData<ScaleDecodeTypeResolver>);
	use scale_decode::ToString;
	impl scale_decode::IntoVisitor for Transition<Compact>
	{
		type AnyVisitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver> =
			Visitor<ScaleDecodeTypeResolver>;

		fn into_visitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		) -> Self::AnyVisitor<ScaleDecodeTypeResolver> {
			Visitor(::core::marker::PhantomData)
		}
	}
	impl<
			ScaleDecodeTypeResolver: scale_decode::TypeResolver,
		> scale_decode::Visitor for Visitor<ScaleDecodeTypeResolver>
	{
		type Error = scale_decode::Error;
		type TypeResolver = ScaleDecodeTypeResolver;
		type Value<'scale, 'info> = Transition<Compact>;

		fn visit_composite<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Composite<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.has_unnamed_fields() {
				return self.visit_tuple(&mut value.as_tuple(), type_id);
			}
			let vals: scale_decode::BTreeMap<Option<&str>, _> = value
				.map(|res| res.map(|item| (item.name(), item)))
				.collect::<Result<_, _>>()?;
			Ok(Transition {
				inputs: {
					let val = vals
						.get(&Some("inputs"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "inputs".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("inputs"))?
				},
				ephemerals: {
					let val = vals
						.get(&Some("ephemerals"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "ephemerals".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("ephemerals"))?
				},
				outputs: {
					let val = vals
						.get(&Some("outputs"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "outputs".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("outputs"))?
				},
			})
		}

		fn visit_tuple<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Tuple<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 3usize {
				return Err(scale_decode::Error::new(
					scale_decode::error::ErrorKind::WrongLength {
						actual_len: value.remaining(),
						expected_len: 3usize,
					},
				));
			}
			let vals = value;
			Ok(Transition::<Compact>{
				inputs: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("inputs"))?
				},
				ephemerals: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("ephemerals"))?
				},
				outputs: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("outputs"))?
				},
			})
		}
	}
	impl scale_decode::DecodeAsFields for Transition<Compact> {
		fn decode_as_fields<'info, R: scale_decode::TypeResolver>(
			input: &mut &[u8],
			fields: &mut dyn scale_decode::FieldIter<'info, R::TypeId>,
			types: &'info R,
		) -> Result<Self, scale_decode::Error> {
			let mut composite = scale_decode::visitor::types::Composite::new(
				core::iter::empty(),
				input,
				fields,
				types,
				false,
			);
			use scale_decode::{IntoVisitor, Visitor};
			let val = <Transition<Compact>>::into_visitor()
				.visit_composite(&mut composite, Default::default());
			composite.skip_decoding()?;
			*input = composite.bytes_from_undecoded();
			val.map_err(From::from)
		}
	}
};
impl scale_encode::EncodeAsType for Transition<Compact> {
	#[allow(unused_variables)]
	fn encode_as_type_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_type_id: ScaleEncodeResolver::TypeId,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Transition {
			inputs,
			ephemerals,
			outputs,
		} = self;
		scale_encode::Composite::new(
			[
				(Some("inputs"), scale_encode::CompositeField::new(inputs)),
				(
					Some("ephemerals"),
					scale_encode::CompositeField::new(ephemerals),
				),
				(Some("outputs"), scale_encode::CompositeField::new(outputs)),
			]
			.into_iter(),
		)
		.encode_composite_as_type_to(
			__encode_as_type_type_id,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}
impl scale_encode::EncodeAsFields for Transition<Compact> {
	#[allow(unused_variables)]
	fn encode_as_fields_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_fields: &mut dyn scale_encode::FieldIter<
			'_,
			ScaleEncodeResolver::TypeId,
		>,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Transition {
			inputs,
			ephemerals,
			outputs,
		} = self;
		scale_encode::Composite::new(
			[
				(Some("inputs"), scale_encode::CompositeField::new(inputs)),
				(
					Some("ephemerals"),
					scale_encode::CompositeField::new(ephemerals),
				),
				(Some("outputs"), scale_encode::CompositeField::new(outputs)),
			]
			.into_iter(),
		)
		.encode_composite_fields_to(
			__encode_as_type_fields,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}


const _: () = {
	pub struct Visitor<
		ScaleDecodeTypeResolver: scale_decode::TypeResolver,
	>(::core::marker::PhantomData<ScaleDecodeTypeResolver>);
	use scale_decode::ToString;
	impl scale_decode::IntoVisitor for Transition<Expanded>
	{
		type AnyVisitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver> =
			Visitor<ScaleDecodeTypeResolver>;

		fn into_visitor<ScaleDecodeTypeResolver: scale_decode::TypeResolver>(
		) -> Self::AnyVisitor<ScaleDecodeTypeResolver> {
			Visitor(::core::marker::PhantomData)
		}
	}
	impl<
			ScaleDecodeTypeResolver: scale_decode::TypeResolver,
		> scale_decode::Visitor for Visitor<ScaleDecodeTypeResolver>
	{
		type Error = scale_decode::Error;
		type TypeResolver = ScaleDecodeTypeResolver;
		type Value<'scale, 'info> = Transition<Expanded>;

		fn visit_composite<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Composite<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.has_unnamed_fields() {
				return self.visit_tuple(&mut value.as_tuple(), type_id);
			}
			let vals: scale_decode::BTreeMap<Option<&str>, _> = value
				.map(|res| res.map(|item| (item.name(), item)))
				.collect::<Result<_, _>>()?;
			Ok(Transition {
				inputs: {
					let val = vals
						.get(&Some("inputs"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "inputs".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("inputs"))?
				},
				ephemerals: {
					let val = vals
						.get(&Some("ephemerals"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "ephemerals".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("ephemerals"))?
				},
				outputs: {
					let val = vals
						.get(&Some("outputs"))
						.ok_or_else(|| {
							scale_decode::Error::new(
								scale_decode::error::ErrorKind::CannotFindField {
									name: "outputs".to_string(),
								},
							)
						})?
						.clone();
					val.decode_as_type().map_err(|e| e.at_field("outputs"))?
				},
			})
		}

		fn visit_tuple<'scale, 'info>(
			self,
			value: &mut scale_decode::visitor::types::Tuple<
				'scale,
				'info,
				Self::TypeResolver,
			>,
			_type_id: <Self::TypeResolver as scale_decode::TypeResolver>::TypeId,
		) -> Result<Self::Value<'scale, 'info>, Self::Error> {
			if value.remaining() != 3usize {
				return Err(scale_decode::Error::new(
					scale_decode::error::ErrorKind::WrongLength {
						actual_len: value.remaining(),
						expected_len: 3usize,
					},
				));
			}
			let vals = value;
			Ok(Transition::<Expanded>{
				inputs: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("inputs"))?
				},
				ephemerals: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("ephemerals"))?
				},
				outputs: {
					let val = vals.next().expect(
						"field count should have been checked already on tuple type; \
						 please file a bug report",
					)?;
					val.decode_as_type().map_err(|e| e.at_field("outputs"))?
				},
			})
		}
	}
	impl scale_decode::DecodeAsFields for Transition<Expanded> {
		fn decode_as_fields<'info, R: scale_decode::TypeResolver>(
			input: &mut &[u8],
			fields: &mut dyn scale_decode::FieldIter<'info, R::TypeId>,
			types: &'info R,
		) -> Result<Self, scale_decode::Error> {
			let mut composite = scale_decode::visitor::types::Composite::new(
				core::iter::empty(),
				input,
				fields,
				types,
				false,
			);
			use scale_decode::{IntoVisitor, Visitor};
			let val = <Transition<Expanded>>::into_visitor()
				.visit_composite(&mut composite, Default::default());
			composite.skip_decoding()?;
			*input = composite.bytes_from_undecoded();
			val.map_err(From::from)
		}
	}
};
impl scale_encode::EncodeAsType for Transition<Expanded> {
	#[allow(unused_variables)]
	fn encode_as_type_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_type_id: ScaleEncodeResolver::TypeId,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Transition {
			inputs,
			ephemerals,
			outputs,
		} = self;
		scale_encode::Composite::new(
			[
				(Some("inputs"), scale_encode::CompositeField::new(inputs)),
				(
					Some("ephemerals"),
					scale_encode::CompositeField::new(ephemerals),
				),
				(Some("outputs"), scale_encode::CompositeField::new(outputs)),
			]
			.into_iter(),
		)
		.encode_composite_as_type_to(
			__encode_as_type_type_id,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}
impl scale_encode::EncodeAsFields for Transition<Expanded> {
	#[allow(unused_variables)]
	fn encode_as_fields_to<ScaleEncodeResolver: scale_encode::TypeResolver>(
		&self,
		__encode_as_type_fields: &mut dyn scale_encode::FieldIter<
			'_,
			ScaleEncodeResolver::TypeId,
		>,
		__encode_as_type_types: &ScaleEncodeResolver,
		__encode_as_type_out: &mut scale_encode::Vec<u8>,
	) -> Result<(), scale_encode::Error> {
		let Transition {
			inputs,
			ephemerals,
			outputs,
		} = self;
		scale_encode::Composite::new(
			[
				(Some("inputs"), scale_encode::CompositeField::new(inputs)),
				(
					Some("ephemerals"),
					scale_encode::CompositeField::new(ephemerals),
				),
				(Some("outputs"), scale_encode::CompositeField::new(outputs)),
			]
			.into_iter(),
		)
		.encode_composite_fields_to(
			__encode_as_type_fields,
			__encode_as_type_types,
			__encode_as_type_out,
		)
	}
}
