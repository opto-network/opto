# NFT Example

This example demonstrates how to use the NFT module to create a simple NFT contract. The contract allows users to mint NFTs, transfer them to other users, and query the NFTs owned by a user.

Additionally this example shows how intents work in Opto. The following scenario is implemented:

## Patron

-  If `NFT` and `NFT_MINT` predicates are not installed, **Alice** installs them.
- **Alice** creates a `BYAC'2025` NFT mint.
- **Alice** mints 10 `BYAC'2025` NFTs and transfers them to **Bob** and **Charlie**, 5 each.

Patron terminates after all nfts are minted and transferred.

## Agent1

Agent1 can run in either `BOB` mode or `CHARLIE` mode.
In `BOB` mode, the solver will act as **Bob** and in `CHARLIE` mode, the solver will act as **Charlie**.

- **Bob** and **Charlie** watch for interesting NFTs and recognize NFTs that are minted by **Alice** for them.
- **Bob** and **Charlie** query [CoinBase](https://api.coinbase.com/v2/prices/ETH-USD/buy) for the current price of 1 ETH in USD.
- **Bob** and **Charlie** offer their NFTs for sale at a price of 0.1 ETH each in USD by creating intents.

Agent1 will terminate when all NFTs are sold.

## Agent2

Agent2 can run in either `DAVE` mode or `FERDIE` mode.
In `DAVE` mode, the solver will act as **Dave** and in `FERDIE` mode, the solver will act as **Ferdie**.

- **Dave** and **Ferdie** watch for NFTs offered for sale by **Bob** and **Charlie**.
- Whenever any NFT is offered for sale at a price that is less than the current price of 0.1 ETH in USD,
  **Dave** and **Ferdie** will buy the NFTs by fulfilling intents.

Agent2 will terminate when all NFTs are bought.

## Nomenclature

- In Opto the term **Patron** is used to refer to the agent that oversees a scenario or orchestrates the completion of some work.
  They typically create intents with rewards for achieving a certain desired end state and watch for the fulfillment of their intents.
  In most cases patrons are responsible for installing any missing predicates that are required for the scenario to run.

- The term **Solver** and **Agent** are used interchangeably to refer to the process that are responsible for solving for intents.
  They typically watch for interesting intents that they can solve and take actions necessary to fulfill them and claim their rewards.
  In most cases solvers subscribe to a stream of new intents and watch for intents that they can solve.

- Those are only lose definitions and the roles of patrons and solvers can be mixed and matched as needed.
