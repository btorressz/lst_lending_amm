// No imports needed: web3, anchor, pg, and more are globally available
//TODO: update test file

describe("LST Lending AMM", () => {
  // 🛡️ Test accounts
  let userKp = new web3.Keypair();
  let liquidatorKp = new web3.Keypair();
  let poolAuthorityKp = new web3.Keypair();

  // 📊 Account Public Keys
  let collateralAccount;
  let debtAccount;
  let collateralVault;
  let lendingPool;
  let ammPool;
  let protocolStats;
  let globalState;
  let priceFeed;
  let switchboardFeed;

  before(async () => {
    console.log("🔄 Setting up test accounts...");

    // Initialize accounts
    collateralAccount = web3.Keypair.generate();
    debtAccount = web3.Keypair.generate();
    collateralVault = web3.Keypair.generate();
    lendingPool = web3.Keypair.generate();
    ammPool = web3.Keypair.generate();
    protocolStats = web3.Keypair.generate();
    globalState = web3.Keypair.generate();
    priceFeed = web3.Keypair.generate();
    switchboardFeed = web3.Keypair.generate();

    console.log("✅ Test accounts setup complete");
  });

  it("Fetches Oracle Price (Simulated in Test)", async () => {
    console.log("🔹 Simulating Oracle Price Fetch...");

    // Simulate a price-fetching call
    const simulatedPrice = 100; // Simulated oracle price
    console.log(`✅ Simulated Oracle Price: ${simulatedPrice}`);

    // Check if the price is valid
    assert(simulatedPrice > 0, "Oracle price fetch failed");
  });

  it("Deposits LST as Collateral", async () => {
    console.log("🔹 Depositing collateral...");

    const depositAmount = new BN(100);

    const txHash = await pg.program.methods
      .depositCollateral(depositAmount)
      .accounts({
        user: userKp.publicKey,
        userLstAccount: userKp.publicKey,
        collateralVault: collateralVault.publicKey,
        userCollateralAccount: collateralAccount.publicKey,
        protocolStats: protocolStats.publicKey,
        globalState: globalState.publicKey,
        tokenProgram: web3.SystemProgram.programId,
      })
      .signers([userKp])
      .rpc();

    console.log(`Use 'solana confirm -v ${txHash}' to see the logs`);

    await pg.connection.confirmTransaction(txHash);

    const userCollateralData = await pg.program.account.collateralAccount.fetch(
      collateralAccount.publicKey
    );

    console.log("✅ Collateral deposited:", userCollateralData.collateralAmount.toString());

    assert.equal(
      userCollateralData.collateralAmount.toString(),
      depositAmount.toString(),
      "Collateral deposit failed"
    );
  });

  it("Borrows Assets Against Collateral", async () => {
    console.log("🔹 Borrowing against collateral...");

    const borrowAmount = new BN(50);

    const txHash = await pg.program.methods
      .borrow(borrowAmount)
      .accounts({
        user: userKp.publicKey,
        userBorrowAccount: userKp.publicKey,
        lendingPool: lendingPool.publicKey,
        userDebtAccount: debtAccount.publicKey,
        userCollateralAccount: collateralAccount.publicKey,
        poolAuthority: poolAuthorityKp.publicKey,
        priceFeed: priceFeed.publicKey,
        switchboardFeed: switchboardFeed.publicKey,
        protocolStats: protocolStats.publicKey,
        globalState: globalState.publicKey,
        tokenProgram: web3.SystemProgram.programId,
      })
      .signers([userKp])
      .rpc();

    console.log(`Use 'solana confirm -v ${txHash}' to see the logs`);

    await pg.connection.confirmTransaction(txHash);

    const userDebtData = await pg.program.account.debtAccount.fetch(
      debtAccount.publicKey
    );

    console.log("✅ Borrowed amount:", userDebtData.debtAmount.toString());

    assert.equal(
      userDebtData.debtAmount.toString(),
      borrowAmount.toString(),
      "Borrow failed"
    );
  });

  it("Liquidates Under-Collateralized Position", async () => {
    console.log("🔹 Performing liquidation...");

    const repayAmount = new BN(30);

    const txHash = await pg.program.methods
      .liquidate(repayAmount)
      .accounts({
        liquidator: liquidatorKp.publicKey,
        borrower: userKp.publicKey,
        borrowerCollateralAccount: collateralAccount.publicKey,
        borrowerDebtAccount: debtAccount.publicKey,
        collateralVault: collateralVault.publicKey,
        ammPool: ammPool.publicKey,
        poolAuthority: poolAuthorityKp.publicKey,
        priceFeed: priceFeed.publicKey,
        switchboardFeed: switchboardFeed.publicKey,
        globalState: globalState.publicKey,
        tokenProgram: web3.SystemProgram.programId,
      })
      .signers([liquidatorKp])
      .rpc();

    console.log(`Use 'solana confirm -v ${txHash}' to see the logs`);

    await pg.connection.confirmTransaction(txHash);

    const borrowerDebtData = await pg.program.account.debtAccount.fetch(
      debtAccount.publicKey
    );

    console.log("✅ Debt after liquidation:", borrowerDebtData.debtAmount.toString());

    assert.notEqual(
      borrowerDebtData.debtAmount.toString(),
      "50",
      "Liquidation did not reduce debt"
    );
  });
});
