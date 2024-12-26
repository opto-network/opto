#[cfg(feature = "graph")]
use petgraph::prelude::*;
use {
	crate::predicate::Predicate,
	alloc::{string::String, vec::Vec},
	core::{
		fmt::{Debug, Display},
		hash::Hash,
		ops::{Index, Not},
	},
	scale::{Decode, Encode, EncodeLike, Output},
	scale_info::{
		build::{Fields, Variants},
		TypeInfo,
	},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
	/// The expression tree is malformed.
	MalformedExpression,

	/// Attempted to zip two expression trees with different shapes.
	NonIsomorphic,

	/// A cycle was detected in the graph and it cannot be converted to an
	/// expression tree.
	CycleDetected,

	/// The graph is invalid and cannot be converted to an expression tree.
	InvalidGraph,
}

/// Represents the basic operators supported by the expression tree.
///
/// Those operators are stored in prefix (polish notation) format
/// and their list is used to construct the expression tree.
pub enum Op<P> {
	Predicate(P),
	And,
	Or,
	Not,
}

impl<P: Hash> Hash for Op<P> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		match self {
			Self::Predicate(p) => {
				state.write_u8(0);
				p.hash(state);
			}
			Self::And => state.write_u8(1),
			Self::Or => state.write_u8(2),
			Self::Not => state.write_u8(3),
		}
	}
}

impl<P: Encode> Encode for Op<P> {
	fn size_hint(&self) -> usize {
		match self {
			Self::Predicate(p) => 1 + p.size_hint(),
			Self::And => 1,
			Self::Or => 1,
			Self::Not => 1,
		}
	}

	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		match self {
			Self::Predicate(p) => {
				dest.push_byte(0);
				p.encode_to(dest);
			}
			Self::And => dest.push_byte(1),
			Self::Or => dest.push_byte(2),
			Self::Not => dest.push_byte(3),
		}
	}
}
impl<P: Encode + EncodeLike> EncodeLike for Op<P> {}

impl<P: Decode> Decode for Op<P> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let tag = input.read_byte()?;

		Ok(match tag {
			0 => Self::Predicate(P::decode(input)?),
			1 => Self::And,
			2 => Self::Or,
			3 => Self::Not,
			_ => return Err(scale::Error::from("Invalid tag")),
		})
	}
}

impl<P> Op<P> {
	/// True for anything that is not a predicate (leaf) node.
	pub const fn is_operator(&self) -> bool {
		matches!(self, Self::And | Self::Or | Self::Not)
	}

	/// True for leaf (predicate) nodes.
	pub const fn is_leaf(&self) -> bool {
		!self.is_operator()
	}

	/// True for binary operators that have two children (AND, OR).
	pub const fn is_binary(&self) -> bool {
		matches!(self, Self::And | Self::Or)
	}

	/// True for unary operators that have one child. (NOT)
	pub const fn is_unary(&self) -> bool {
		matches!(self, Self::Not)
	}

	pub const fn as_predicate(&self) -> Option<&P> {
		match self {
			Self::Predicate(p) => Some(p),
			_ => None,
		}
	}
}

impl<P: Clone> Clone for Op<P> {
	fn clone(&self) -> Self {
		match self {
			Self::Predicate(p) => Self::Predicate(p.clone()),
			Self::And => Self::And,
			Self::Or => Self::Or,
			Self::Not => Self::Not,
		}
	}
}

impl<P: PartialEq> PartialEq for Op<P> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Predicate(p1), Self::Predicate(p2)) => p1 == p2,
			(Self::And, Self::And) => true,
			(Self::Or, Self::Or) => true,
			(Self::Not, Self::Not) => true,
			_ => false,
		}
	}
}

impl<P: Display> core::fmt::Display for Op<P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Predicate(p) => write!(f, "{}", p),
			Self::And => write!(f, "AND"),
			Self::Or => write!(f, "OR"),
			Self::Not => write!(f, "NOT"),
		}
	}
}

impl<P: Debug> Debug for Op<P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Predicate(p) => write!(f, "{:?}", p),
			Self::And => write!(f, "AND"),
			Self::Or => write!(f, "OR"),
			Self::Not => write!(f, "NOT"),
		}
	}
}

impl<P: TypeInfo + 'static> TypeInfo for Op<P> {
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("Op", module_path!()))
			.type_params(<[_]>::into_vec(alloc::boxed::Box::new([
				scale_info::TypeParameter::new("P", Some(scale_info::meta_type::<P>())),
			])))
			.variant(
				Variants::new()
					.variant("Predicate", |v| {
						v.index(0)
							.discriminant(0)
							.fields(Fields::unnamed().field(|f| f.ty::<P>().type_name("P")))
					})
					.variant("And", |v| v.index(1).discriminant(1))
					.variant("Or", |v| v.index(2).discriminant(2))
					.variant("Not", |v| v.index(3).discriminant(3)),
			)
	}
}

/// An expression tree that represents a boolean expression of predicates used
/// in unlock conditions of an object.
///
/// The expression must be evaluated to true for the object to be consumed in an
/// input of a state transition
///
/// The expression tree is stored in the prefix (polish) notation.
pub struct Expression<P = Predicate>(pub(crate) Vec<Op<P>>);

impl<P: PartialEq> PartialEq for Expression<P> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<P: Hash> Hash for Expression<P> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.0.hash(state)
	}
}

impl<P> Index<usize> for Expression<P> {
	type Output = Op<P>;

	fn index(&self, index: usize) -> &Self::Output {
		&self.0[index]
	}
}

impl<P> Expression<P> {
	/// Returns the same expression tree with all predicates replaced by a
	/// reference to the predicate in this tree.
	pub fn as_ref(&self) -> Expression<&P> {
		Expression(
			self
				.0
				.iter()
				.map(|op| match op {
					Op::Predicate(p) => Op::Predicate(p),
					Op::And => Op::And,
					Op::Or => Op::Or,
					Op::Not => Op::Not,
				})
				.collect(),
		)
	}
}

impl<P: Encode> Encode for Expression<P> {
	fn size_hint(&self) -> usize {
		self.0.size_hint()
	}

	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		self.0.encode_to(dest)
	}
}
impl<P: Encode + EncodeLike> EncodeLike for Expression<P> {}

impl<P: Decode> Decode for Expression<P> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		Ok(Self(Decode::decode(input)?))
	}
}

/// Converts a list of operators in prefix notation to an expression tree.
impl<P> TryFrom<Vec<Op<P>>> for Expression<P> {
	type Error = Error;

	fn try_from(ops: Vec<Op<P>>) -> Result<Self, Self::Error> {
		if ops.is_empty() {
			return Err(Error::MalformedExpression);
		}

		fn validate_preorder<P>(ops: &[Op<P>]) -> bool {
			let mut stack = 0;

			for (i, op) in ops.iter().enumerate() {
				match op {
					Op::Predicate(_) => stack += 1,
					Op::And | Op::Or => {
						if i == ops.len() - 2 {
							return false;
						}
						stack -= 1;
					}
					Op::Not => {
						if i == ops.len() - 1 {
							return false;
						}
					}
				}
			}

			stack == 1
		}

		if !validate_preorder(&ops) {
			return Err(Error::MalformedExpression);
		}

		Ok(Self(ops))
	}
}

impl<P: TypeInfo + 'static> TypeInfo for Expression<P> {
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("Expression", module_path!()))
			.type_params(<[_]>::into_vec(alloc::boxed::Box::new([
				scale_info::TypeParameter::new("P", Some(scale_info::meta_type::<P>())),
			])))
			.composite(Fields::unnamed().field(|f| f.ty::<Vec<Op<P>>>()))
	}
}

impl<P> IntoIterator for Expression<P> {
	type IntoIter = alloc::vec::IntoIter<Op<P>>;
	type Item = Op<P>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<P> Expression<P> {
	/// Returns a reference to the list of operators in the expression tree in
	/// prefix (parent, left, right) order.
	pub fn as_ops(&self) -> &[Op<P>] {
		&self.0
	}

	/// Converts the expression tree to a vector of operators in prefix notation.
	pub fn to_vec(self) -> Vec<Op<P>> {
		self.0
	}

	/// Checks if the expression tree is isomorphic to another expression tree,
	/// meaning that the trees have the same structure and the same operators,
	/// but the leaf predicates can be different.
	pub fn is_isomorphic<V>(&self, other: &Expression<V>) -> bool {
		self.0.len() == other.0.len()
			&& self.as_ops().iter().zip(other.0.iter()).all(|(op1, op2)| {
				matches!(
					(op1, op2),
					(Op::Predicate(_), Op::Predicate(_))
						| (Op::And, Op::And)
						| (Op::Or, Op::Or)
						| (Op::Not, Op::Not)
				)
			})
	}

	/// Applies a function to all predicates in the tree and returns a new
	/// tree with the same structure and transformed predicates.
	pub fn map<V>(self, f: impl Fn(P) -> V) -> Expression<V> {
		Expression(
			self
				.into_iter()
				.map(|op| match op {
					Op::Predicate(p) => Op::Predicate(f(p)),
					Op::And => Op::And,
					Op::Or => Op::Or,
					Op::Not => Op::Not,
				})
				.collect(),
		)
	}

	/// Applies a falliable function to all predicates in the tree and returns a
	/// new tree with the same structure and transformed predicates. If any of
	/// the individual mappings fail then the whole operation fails.
	pub fn try_map<V, E>(
		self,
		f: impl Fn(P) -> Result<V, E>,
	) -> Result<Expression<V>, E> {
		Ok(Expression(
			self
				.into_iter()
				.map(|op| match op {
					Op::Predicate(p) => f(p).map(|r| Op::Predicate(r)),
					Op::And => Ok(Op::And),
					Op::Or => Ok(Op::Or),
					Op::Not => Ok(Op::Not),
				})
				.collect::<Result<_, _>>()?,
		))
	}

	/// Joins two isomorphic expression tree by producing a new tree with each
	/// predicate node being a tuple of the corresponding predicates in the input
	/// trees.
	pub fn zip<V>(
		self,
		other: Expression<V>,
	) -> Result<Expression<(P, V)>, Error> {
		if !self.is_isomorphic(&other) {
			return Err(Error::NonIsomorphic);
		}

		Ok(Expression(
			self
				.into_iter()
				.zip(other)
				.map(|(op1, op2)| match (op1, op2) {
					(Op::Predicate(p1), Op::Predicate(p2)) => Op::Predicate((p1, p2)),
					(Op::And, Op::And) => Op::And,
					(Op::Or, Op::Or) => Op::Or,
					(Op::Not, Op::Not) => Op::Not,
					_ => unreachable!("is_isomorphic should have caught this"),
				})
				.collect(),
		))
	}

	/// Zips two expression trees together and applies a function to the
	/// predicates at the same path in the trees.
	///
	/// The function is applied to the predicates at the same path in the trees
	/// and the result is used to construct a new tree of the same shape.
	///
	/// This function fails if the trees have different shapes.
	pub fn zip_with<V, F, P2>(
		self,
		other: Expression<P2>,
		f: F,
	) -> Result<Expression<V>, Error>
	where
		F: Fn(P, P2) -> V,
	{
		if !self.is_isomorphic(&other) {
			return Err(Error::NonIsomorphic);
		}

		Ok(Expression(
			self
				.into_iter()
				.zip(other)
				.map(|(op1, op2)| match (op1, op2) {
					(Op::Predicate(p1), Op::Predicate(p2)) => Op::Predicate(f(p1, p2)),
					(Op::And, Op::And) => Op::And,
					(Op::Or, Op::Or) => Op::Or,
					(Op::Not, Op::Not) => Op::Not,
					_ => unreachable!("is_isomorphic should have caught this"),
				})
				.collect(),
		))
	}

	/// Returns the Op at the root of the expression tree.
	pub fn root(&self) -> &Op<P> {
		&self.0[0]
	}

	/// Returns a new expression tree that is the left subree of the current tree.
	pub fn left(self) -> Option<Self> {
		let mut ops = self.0.into_iter();
		let op = ops.next()?;
		if op.is_operator() {
			let mut left = alloc::vec![];
			let mut stack = 0;

			while stack <= 0 {
				let op = ops.next()?;

				match op {
					Op::Predicate(_) => stack += 1,
					Op::And | Op::Or => stack -= 1,
					Op::Not => {}
				};

				left.push(op);
			}

			Some(Self(left))
		} else {
			None
		}
	}

	/// Returns a new expression tree that is the right subtree of the current
	/// tree.
	pub fn right(self) -> Option<Self> {
		let mut ops = self.0.into_iter();
		let op = ops.next()?;

		if op.is_operator() {
			let mut stack = 0;

			// Skip the left subtree
			while stack <= 0 {
				match ops.next()? {
					Op::Predicate(_) => stack += 1,
					Op::And | Op::Or => stack -= 1,
					Op::Not => {}
				};
			}

			// Collect the right subtree
			let right: Vec<Op<P>> = ops.collect();
			if right.is_empty() {
				None
			} else {
				Some(Self(right))
			}
		} else {
			None
		}
	}
}

/// This is the builder API for constructing and composing expression trees from
/// predicates and other expression trees.
///
/// This overload of the bitwise AND operator creates a new expression tree
/// with the AND operator as the root and the two operands as children.
impl<P> core::ops::BitAnd for Expression<P> {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self {
		let mut ops = Vec::with_capacity(self.0.len() + rhs.0.len() + 1);
		ops.push(Op::And);
		ops.extend(self.0);
		ops.extend(rhs.0);
		Self(ops)
	}
}

impl<P> core::ops::BitAnd<P> for Expression<P> {
	type Output = Self;

	fn bitand(self, rhs: P) -> Self {
		let p_expr: Expression<_> = rhs.into();
		self & p_expr
	}
}

/// This overload of the bitwise OR operator creates a new expression tree
/// with the OR operator as the root and the two operands as children.
impl<P> core::ops::BitOr for Expression<P> {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self {
		let mut ops = Vec::with_capacity(self.0.len() + rhs.0.len() + 1);
		ops.push(Op::Or);
		ops.extend(self.0);
		ops.extend(rhs.0);
		Self(ops)
	}
}

impl<P> core::ops::BitOr<P> for Expression<P> {
	type Output = Self;

	fn bitor(self, rhs: P) -> Self {
		let p_expr: Expression<_> = rhs.into();
		self | p_expr
	}
}

/// This overload of the unary NOT operator creates a new expression tree
/// with the NOT operator as the root and the operand as the left child.
/// Not operators are unary, so they only have one child (on the left).
impl<P> Not for Expression<P> {
	type Output = Self;

	fn not(self) -> Self {
		let mut ops = Vec::with_capacity(self.0.len() + 1);
		ops.push(Op::Not);
		ops.extend(self.0);
		Self(ops)
	}
}

/// This wraps a predicate in an identity operator and creates a new expression
/// tree.
impl<P> From<P> for Expression<P> {
	fn from(p: P) -> Self {
		Self(alloc::vec![Op::Predicate(p)])
	}
}

impl<P: Clone> Clone for Expression<P> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

/// Debug print in prefix notation
impl<P: Debug> core::fmt::Debug for Expression<P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

/// Pretty print in natural infix notation
impl<P: Display> core::fmt::Display for Expression<P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		fn write_subtree<P: Display>(
			tokens: &mut core::slice::Iter<Op<P>>,
		) -> String {
			let Some(next) = tokens.next() else {
				return String::new();
			};

			if !next.is_operator() {
				return alloc::format!("{next}");
			}

			let left_operand = write_subtree(tokens);
			let righ_operand = if next.is_binary() {
				write_subtree(tokens)
			} else {
				String::new()
			};

			if next.is_unary() {
				alloc::format!("{}({})", next, left_operand)
			} else {
				alloc::format!("({} {} {})", left_operand, next, righ_operand)
			}
		}

		write!(f, "{}", write_subtree(&mut self.0.iter()))
	}
}

#[cfg(feature = "graph")]
impl<P> Expression<P> {
	/// Converts the expression tree to a tree data structure backed by petgraph.
	///
	/// This is used for more complex operations on the tree such as pattern
	/// matching, compression, and other graph algorithms.
	pub fn to_tree(&self) -> Result<ExpressionTree<P>, Error> {
		let mut graph = StableDiGraph::new();
		let mut ops = self.0.iter().rev().collect::<Vec<_>>();

		fn build_tree<'a, P>(
			graph: &mut StableDiGraph<&'a Op<P>, ()>,
			ops: &mut Vec<&'a Op<P>>,
		) -> Result<NodeIndex, Error> {
			let Some(op) = ops.pop() else {
				return Err(Error::MalformedExpression);
			};

			if op.is_binary() {
				let node = graph.add_node(op);
				let left = build_tree(graph, ops)?;
				let right = build_tree(graph, ops)?;
				graph.add_edge(node, left, ());
				graph.add_edge(node, right, ());
				Ok(node)
			} else if op.is_unary() {
				let node = graph.add_node(op);
				let child = build_tree(graph, ops)?;
				graph.add_edge(node, child, ());
				Ok(node)
			} else if op.is_leaf() {
				Ok(graph.add_node(op))
			} else {
				Err(Error::MalformedExpression)
			}
		}

		let root = build_tree(&mut graph, &mut ops)?;

		Ok(ExpressionTree { graph, root })
	}
}

/// A representation of an expression as a tree data structure that is
/// compatible with petgraph's graph library. This is used for more complex
/// operations that are not part of the fast-path use cases.
///
/// An `ExpressionTree` type is always constructed from a valid `Expression`
/// object and it should be used as a read-only interpretation of the
/// `Expression` structure.
#[cfg(feature = "graph")]
pub struct ExpressionTree<'a, P> {
	graph: StableDiGraph<&'a Op<P>, ()>,
	root: NodeIndex,
}

#[cfg(feature = "graph")]
impl<'a, P> core::ops::Deref for ExpressionTree<'a, P> {
	type Target = StableDiGraph<&'a Op<P>, ()>;

	fn deref(&self) -> &Self::Target {
		&self.graph
	}
}

#[cfg(feature = "graph")]
impl<'a, P> ExpressionTree<'a, P> {
	/// Returns a cursor to the root of the expression tree that can be used to
	/// navigate the tree.
	pub fn cursor(&'a self) -> ExpressionTreeCursor<'a, P> {
		ExpressionTreeCursor {
			graph: &self.graph,
			root: self.root,
		}
	}

	/// Returns a reference to the underlying petgraph data structure.
	pub const fn graph(&'a self) -> &'a StableDiGraph<&'a Op<P>, ()> {
		&self.graph
	}

	/// Consumes the expression tree and returns the underlying petgraph data
	/// structure.
	pub fn into_graph(self) -> StableDiGraph<&'a Op<P>, ()> {
		self.graph
	}
}

#[cfg(feature = "graph")]
impl<P: Debug> Debug for ExpressionTree<'_, P> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{:?}",
			petgraph::dot::Dot::with_config(&self.graph, &[
				petgraph::dot::Config::EdgeNoLabel
			])
		)
	}
}

/// Constructs an expression from an expression tree that is compatible with
/// petgraph.
#[cfg(feature = "graph")]
impl<P: Clone> TryFrom<ExpressionTree<'_, P>> for Expression<P> {
	type Error = Error;

	fn try_from(value: ExpressionTree<P>) -> Result<Self, Self::Error> {
		let mut value = value;
		let mut ops = Vec::with_capacity(value.graph.node_count());

		fn preorder_visit<P>(
			graph: &StableDiGraph<&Op<P>, ()>,
			node: NodeIndex,
			ops: &mut Vec<NodeIndex>,
		) -> Result<(), Error> {
			ops.push(node);

			let children: Vec<_> = graph.neighbors(node).collect();
			for child in children.into_iter().rev() {
				preorder_visit(graph, child, ops)?;
			}

			Ok(())
		}

		preorder_visit(&value.graph, value.root, &mut ops)?;

		let ops: Option<Vec<_>> = ops
			.into_iter()
			.map(|ix| value.graph.remove_node(ix).cloned())
			.collect();
		let ops = ops.ok_or(Error::MalformedExpression)?;

		Ok(Expression(ops))
	}
}

#[cfg(feature = "graph")]
impl<'a, P> From<ExpressionTree<'a, P>>
	for (StableDiGraph<&'a Op<P>, ()>, NodeIndex)
{
	fn from(value: ExpressionTree<'a, P>) -> Self {
		(value.graph, value.root)
	}
}

/// Attempts to convert a graph and a root node into an expression tree.
#[cfg(feature = "graph")]
impl<'a, P> TryFrom<(StableDiGraph<&'a Op<P>, ()>, NodeIndex)>
	for ExpressionTree<'a, P>
{
	type Error = Error;

	fn try_from(
		(graph, root): (StableDiGraph<&'a Op<P>, ()>, NodeIndex),
	) -> Result<Self, Self::Error> {
		// ensure that this is a valid binary expression tree rooted at "root"
		let mut stack = alloc::vec![root];
		let mut visited = alloc::vec![false; graph.node_count()];
		let mut count = 0;

		fn visit<P>(
			graph: &StableDiGraph<&Op<P>, ()>,
			node: NodeIndex,
			stack: &mut Vec<NodeIndex>,
			visited: &mut Vec<bool>,
			count: &mut usize,
		) -> Result<(), Error> {
			if visited[node.index()] {
				return Err(Error::CycleDetected);
			}

			visited[node.index()] = true;
			*count += 1;

			// ensure that the node has the correct number of children
			match graph.node_weight(node) {
				Some(Op::Predicate(_)) => {
					if graph.neighbors(node).count() != 0 {
						return Err(Error::InvalidGraph);
					}
				}
				Some(Op::And) | Some(Op::Or) => {
					if graph.neighbors(node).count() != 2 {
						return Err(Error::InvalidGraph);
					}
				}
				Some(Op::Not) => {
					if graph.neighbors(node).count() != 1 {
						return Err(Error::InvalidGraph);
					}
				}
				None => return Err(Error::InvalidGraph),
			}

			for child in graph.neighbors(node) {
				stack.push(child);
				visit(graph, child, stack, visited, count)?;
			}

			Ok(())
		}

		// traverse the graph and ensure that it is a valid
		// binary expression tree
		visit(&graph, root, &mut stack, &mut visited, &mut count)?;

		if count != graph.node_count() {
			return Err(Error::InvalidGraph);
		}

		Ok(Self { graph, root })
	}
}

/// This is used to navigate the expression tree.
///
/// It can be used with any graph algorithms that are compatible
/// with the `petgraph` library. This structure is cheap to clone
/// and copy.
#[cfg(feature = "graph")]
pub struct ExpressionTreeCursor<'a, P> {
	graph: &'a StableDiGraph<&'a Op<P>, ()>,
	root: NodeIndex,
}

#[cfg(feature = "graph")]
impl<P> Clone for ExpressionTreeCursor<'_, P> {
	fn clone(&self) -> Self {
		*self
	}
}

#[cfg(feature = "graph")]
impl<P> Copy for ExpressionTreeCursor<'_, P> {}

/// Implements the navigation API for the expression tree.
///
/// This type is implemented by the ExpressionTree and ExpressionTreeCursor,
/// in order to provide a common API for navigating the tree by copy and by
/// reference. Most likely you will want to use the ExpressionTreeCursor impl.
#[cfg(feature = "graph")]
pub trait ExpressionTreeNav<'d, P>
where
	Self: Sized,
{
	/// Returns the first child of the current node.
	///
	/// This returns the first child for binary operators and the
	/// only child for unary operators. For leaf nodes, this returns None.
	fn first(self) -> Option<Self>;

	/// Returns the second child of the current node.
	///
	/// This returns the second child for binary operators and None for
	/// unary operators and leaf nodes.
	fn second(self) -> Option<Self>;

	/// Returns the parent of the current node.
	///
	/// None for the root node.
	fn parent(self) -> Option<Self>;

	/// Returns the operator at the current node.
	fn op(&self) -> &'d Op<P>;
}

#[cfg(feature = "graph")]
impl<'a, P> ExpressionTreeNav<'a, P> for ExpressionTree<'a, P> {
	fn first(self) -> Option<Self> {
		let mut children = self.graph.neighbors(self.root);
		let first_child = children.next()?;
		Some(ExpressionTree {
			graph: self.graph,
			root: first_child,
		})
	}

	fn second(self) -> Option<Self> {
		let mut children = self.graph.neighbors(self.root);
		let _first_child = children.next()?;
		let second_child = children.next()?;
		Some(ExpressionTree {
			graph: self.graph,
			root: second_child,
		})
	}

	fn parent(self) -> Option<Self> {
		let parent = self
			.graph
			.neighbors_directed(self.root, petgraph::Direction::Incoming)
			.next()?;
		Some(ExpressionTree {
			graph: self.graph,
			root: parent,
		})
	}

	fn op(&self) -> &'a Op<P> {
		self.graph.node_weight(self.root).unwrap()
	}
}

#[cfg(feature = "graph")]
impl<'a, P> ExpressionTreeNav<'a, P> for ExpressionTreeCursor<'a, P> {
	fn first(self) -> Option<Self> {
		let mut children = self.graph.neighbors(self.root);
		let first_child = children.next()?;
		Some(ExpressionTreeCursor {
			graph: self.graph,
			root: first_child,
		})
	}

	fn second(self) -> Option<Self> {
		let mut children = self.graph.neighbors(self.root);
		let _first_child = children.next()?;
		let second_child = children.next()?;
		Some(ExpressionTreeCursor {
			graph: self.graph,
			root: second_child,
		})
	}

	fn parent(self) -> Option<Self> {
		let parent = self
			.graph
			.neighbors_directed(self.root, petgraph::Direction::Incoming)
			.next()?;
		Some(ExpressionTreeCursor {
			graph: self.graph,
			root: parent,
		})
	}

	fn op(&self) -> &'a Op<P> {
		self.graph.node_weight(self.root).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use {
		super::{Expression, Op},
		alloc::{
			format,
			string::{String, ToString},
			vec,
		},
		core::convert::TryInto,
	};

	#[cfg(feature = "graph")]
	use super::{ExpressionTree, ExpressionTreeNav};

	fn test_expression() -> Expression<i32> {
		// 6 leaf nodes
		let leaf1 = super::Expression::from(1);
		let leaf2 = super::Expression::from(2);
		let leaf3 = super::Expression::from(3);
		let leaf4 = super::Expression::from(4);
		let leaf5 = super::Expression::from(5);
		let leaf6 = super::Expression::from(6);

		// complex expression tree using all leaf nodes
		leaf1 & (leaf2 | !leaf3) | (leaf4 | leaf5) & !leaf6
	}

	#[test]
	fn expression_tree_print() {
		let expression = test_expression();
		let infix: String = expression.to_string();
		let prefix = alloc::format!("{:?}", expression);

		assert_eq!(infix, "((1 AND (2 OR NOT(3))) OR ((4 OR 5) AND NOT(6)))");
		assert_eq!(prefix, "[OR, AND, 1, OR, 2, NOT, 3, AND, OR, 4, 5, NOT, 6]");
	}

	#[test]
	fn expression_inflix_print() {
		use super::*;

		let leaf1 = Expression::from(1);
		let leaf2 = Expression::from(2);
		let leaf3 = Expression::from(3);

		let expression = leaf1 | (leaf2 & !leaf3);

		assert_eq!(format!("{}", expression), "(1 OR (2 AND NOT(3)))");
	}

	#[test]
	fn expression_debug_print() {
		use super::*;

		let leaf1 = Expression::from(1);
		let leaf2 = Expression::from(2);
		let leaf3 = Expression::from(3);
		let leaf4 = Expression::from(4);

		let expression = leaf1 | ((leaf2 & !leaf3) & !leaf4);

		assert_eq!(
			format!("{:?}", expression),
			"[OR, 1, AND, AND, 2, NOT, 3, NOT, 4]"
		);
	}

	#[cfg(feature = "graph")]
	#[test]
	fn expression_to_graph() {
		let expression = test_expression();
		let graph = expression.to_tree().unwrap();
		let cursor = graph.cursor();

		assert_eq!(cursor.op(), &Op::Or);

		let left = cursor.second().unwrap();
		assert_eq!(left.op(), &Op::And);

		let left_left = left.second().unwrap();
		assert_eq!(left_left.op(), &Op::Predicate(1));
		assert!(left_left.op().is_leaf());

		let left_left_left = left_left.second();
		assert!(left_left_left.is_none());

		let left_left_right = left_left.first();
		assert!(left_left_right.is_none());

		let left_right = left.first().unwrap();
		assert_eq!(left_right.op(), &Op::Or);

		let left_right_left = left_right.second().unwrap();
		assert_eq!(left_right_left.op(), &Op::Predicate(2));

		let left_right_right = left_right.first().unwrap();
		assert_eq!(left_right_right.op(), &Op::Not);

		let left_right_right_left = left_right_right.first().unwrap();
		assert_eq!(left_right_right_left.op(), &Op::Predicate(3));

		let left_right_right_right = left_right_right.second();
		assert!(left_right_right_right.is_none());

		let right = cursor.first().unwrap();
		assert_eq!(right.op(), &Op::And);

		let right_left = right.second().unwrap();
		assert_eq!(right_left.op(), &Op::Or);

		let right_left_left = right_left.second().unwrap();
		assert_eq!(right_left_left.op(), &Op::Predicate(4));

		let right_left_right = right_left.first().unwrap();
		assert_eq!(right_left_right.op(), &Op::Predicate(5));

		let right_right = right.first().unwrap();
		assert_eq!(right_right.op(), &Op::Not);

		let right_right_left = right_right.first().unwrap();
		assert_eq!(right_right_left.op(), &Op::Predicate(6));

		let right_right_right = right_right.second();
		assert!(right_right_right.is_none());
	}

	#[cfg(feature = "graph")]
	#[test]
	fn expression_to_graph_dot_print() {
		let expr = test_expression();
		let graph = expr.to_tree().unwrap();
		let generated = format!("{graph:?}").replace(" ", "").replace("\n", "");
		let expected = r#"digraph{0[label="OR"]1[label="AND"]2[label="1"]3[label="OR"]4[label="2"]5[label="NOT"]6[label="3"]7[label="AND"]8[label="OR"]9[label="4"]10[label="5"]11[label="NOT"]12[label="6"]5->6[]3->4[]3->5[]1->2[]1->3[]8->9[]8->10[]11->12[]7->8[]7->11[]0->1[]0->7[]}"#;
		assert_eq!(generated, expected);
	}

	#[cfg(feature = "graph")]
	#[test]
	fn expression_tree_to_expression() {
		let expr1 = test_expression();
		let expr_tree = expr1.to_tree().unwrap();
		let (graph, root) = expr_tree.into();
		let expr2: ExpressionTree<i32> = (graph, root).try_into().unwrap();
		let expr2: Expression<_> = expr2.try_into().unwrap();
		assert_eq!(expr1, expr2);
	}

	#[test]
	fn invalid_preorder_expression() {
		let invalid = vec![Op::And, Op::Predicate(1i32), Op::Or];
		let expr: Result<Expression<i32>, _> = invalid.try_into();
		assert!(matches!(expr, Err(super::Error::MalformedExpression)));
	}

	#[test]
	fn get_left_right_subree() {
		let expr = test_expression();
		let left = expr.left().unwrap();
		let expected =
			Expression::from(1) & (Expression::from(2) | !Expression::from(3));

		assert_eq!(left, expected);
		assert_eq!(left.root(), &Op::And);

		let left_left = left.clone().left().unwrap();
		let expected = Expression::from(1);
		assert_eq!(left_left, expected);
		assert_eq!(left_left.root(), &Op::Predicate(1));

		let left_right = left.right().unwrap();
		let expected = Expression::from(2) | !Expression::from(3);
		assert_eq!(left_right, expected);

		let left_right_left = left_right.clone().left().unwrap();
		let expected = Expression::from(2);
		assert_eq!(left_right_left, expected);

		let left_right_left_left = left_right_left.clone().left();
		assert!(left_right_left_left.is_none());

		let left_right_left_right = left_right_left.clone().right();
		assert!(left_right_left_right.is_none());

		let left_right_right = left_right.clone().right().unwrap();
		let expected = !Expression::from(3);
		assert_eq!(left_right_right, expected);

		// opearnd of NOT is always on the left side
		let left_right_right_left = left_right_right.clone().left();
		assert_eq!(left_right_right_left, Some(Expression::from(3)));

		assert_eq!(left_right_right_left.unwrap().left(), None);

		let left_right_right_right = left_right_right.clone().right();
		assert_eq!(left_right_right_right, None);
	}

	#[test]
	fn get_left_right_subree_larger() {
		let expr = (test_expression() & test_expression()) & test_expression();

		let left = expr.clone().left().unwrap();
		let expected = test_expression() & test_expression();
		assert_eq!(left, expected);
		assert_eq!(left.root(), &Op::And);

		let right = expr.right().unwrap();
		let expected = test_expression();
		assert_eq!(right, expected);
		assert_eq!(right.root(), &Op::Or);

		let left_left = left.clone().left().unwrap();
		let expected = test_expression();
		assert_eq!(left_left, expected);
		assert_eq!(left_left.root(), &Op::Or);

		let left_right = left.right().unwrap();
		let expected = test_expression();
		assert_eq!(left_right, expected);
		assert_eq!(left_right.root(), &Op::Or);
	}
}
