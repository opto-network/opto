use {
	super::targets::PredicateWasmBinary,
	cid::{multihash::Multihash, Cid},
	integer_encoding::VarInt,
	libipld::multihash::Code,
	opto_core::Digest,
	serde::{Deserialize, Serialize},
	std::{collections::HashMap, io::Write, path::Path},
};

#[derive(Debug, Serialize, Deserialize)]
struct CarHeader {
	version: u64,
	roots: Vec<Cid>,
}

#[derive(Default)]
pub struct Bundle {
	predicates: HashMap<Cid, Vec<u8>>,
}

impl Bundle {
	pub fn insert(&mut self, binary: PredicateWasmBinary) {
		let binary_digest = Digest::compute(&binary.wasm);
		let cid = Cid::new_v1(
			0x55, // ipld raw in multicodec
			Multihash::wrap(Code::Blake2b256.into(), binary_digest.as_ref()).unwrap(),
		);
		self.predicates.insert(cid, binary.wasm);
	}

	pub fn write(self, dest: impl AsRef<Path>) -> Result<(), std::io::Error> {
		let mut car_file = std::fs::File::create(dest.as_ref())?;

		let cids = self.predicates.keys().cloned().collect::<Vec<_>>();
		let root_index_content = serde_ipld_dagcbor::to_vec(&cids)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
		let root_index_digest = Digest::compute(&root_index_content);
		let root_index_cid = Cid::new_v1(
			0x71, // ipld dag-cbor (MerkleDAG cbor)
			Multihash::wrap(Code::Blake2b256.into(), root_index_digest.as_ref())
				.unwrap(),
		);

		let header = CarHeader {
			version: 1,
			roots: vec![root_index_cid],
		};

		let header_content = serde_ipld_dagcbor::to_vec(&header)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

		car_file.write_all(header_content.len().encode_var_vec().as_slice())?;
		car_file.write_all(&header_content)?;

		let root_index_len =
			root_index_content.len() + root_index_cid.encoded_len();
		car_file.write_all(root_index_len.encode_var_vec().as_slice())?;
		car_file.write_all(&root_index_cid.to_bytes())?;
		car_file.write_all(&root_index_content)?;

		for (cid, wasm) in self.predicates {
			let wasm_len = wasm.len() + cid.encoded_len();
			car_file.write_all(wasm_len.encode_var_vec().as_slice())?;
			car_file.write_all(&cid.to_bytes())?;
			car_file.write_all(&wasm)?;
		}

		car_file.flush()?;
		Ok(())
	}
}
