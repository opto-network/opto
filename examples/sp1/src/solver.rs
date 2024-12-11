#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let x = opto::stdpred::ids::CONSTANT;
	println!("NFT-Solver, c: {x}");
	Ok(())
}
