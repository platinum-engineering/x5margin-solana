const anchor = require('@project-serum/anchor');
const assert = require("assert");
const { globalAgent } = require('http');
const poolClient = require("../web3/pool/index");

describe('pool', () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Pool;

  // Stuff to save between tests.
  let globals = {
    administrator: undefined,
    pool: undefined,
    bump: undefined,
    stakeMintToken: undefined,
    stakeVault: undefined,
    ticket: undefined,
    userWallet: undefined,
    endLockupTs: undefined,
  };

  it('Calculates the APY', () => {
    const data = {
      stakeTargetAmount: new anchor.BN(10000),
      rewardAmount: new anchor.BN(1000),
      lockupDuration: new anchor.BN(60 * 60 * 24 * 7), // week
      stakeAcquiredAmount: new anchor.BN(10000),
      depositedRewardAmount: new anchor.BN(500),
    };
    const pool = new poolClient.Pool(data);

    assert.ok(pool.expectedAPY() === 141.04293198443193);
    assert.ok(pool.APY() === 11.642808263793455);
  });

  it('Initializes the pool', async () => {
    const administrator = anchor.web3.Keypair.generate();
    const pool = anchor.web3.Keypair.generate();

    const stakeMintToken = await poolClient.utils.createMint(provider);
    const stakeMint = stakeMintToken.publicKey;

    const [poolAuthority, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        pool.publicKey.toBuffer(),
        administrator.publicKey.toBuffer(),
      ],
      program.programId
    );

    const stakeVault = await poolClient.utils.createTokenAccount(
      provider,
      stakeMint,
      poolAuthority
    );

    const topupDuration = new anchor.BN(3);
    const lockupDuration = new anchor.BN(6);
    const targetAmount = new anchor.BN(10000);
    const rewardAmount = new anchor.BN(100);

    await program.rpc.initializePool(
      bump,
      topupDuration,
      lockupDuration,
      targetAmount,
      rewardAmount,
      {
        accounts: {
          administratorAuthority: administrator.publicKey,
          poolAuthority,
          pool: pool.publicKey,
          stakeMint,
          stakeVault,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        },
        signers: [administrator, pool],
        instructions: [
          await program.account.pool.createInstruction(pool),
        ],
      });

    const pools = await poolClient.getPools(provider);
    console.log('Known pools: ', pools);
    assert.ok(pools.length == 1);

    const createdPool = pools[0];

    console.log('Start date:', createdPool.startDate());
    console.log('End date:', createdPool.endDate());
    console.log('Topup end date:', createdPool.topupEndDate());
    console.log('Time to deposit:', createdPool.timeToDeposit());
    console.log('Time until withdrawal:', createdPool.timeUntilWithdrawal());

    assert.ok(createdPool.publicKey.equals(pool.publicKey));

    assert.ok(createdPool.topupDuration.eq(topupDuration));
    assert.ok(createdPool.lockupDuration.eq(lockupDuration));
    assert.ok(createdPool.stakeTargetAmount.eq(targetAmount));
    assert.ok(createdPool.rewardAmount.eq(rewardAmount));

    globals.administrator = administrator;
    globals.pool = createdPool;
    globals.bump = bump;
    globals.stakeMintToken = stakeMintToken;
    globals.stakeVault = stakeVault;

    const nowTs = new anchor.BN(Date.now() / 1000);
    globals.endLockupTs = nowTs.add(lockupDuration);
  });

  it('Adds stake to the pool', async () => {
    const amount = new anchor.BN(100);

    const sourceWallet = await poolClient.utils.createTokenAccount(
      provider,
      globals.stakeMintToken.publicKey,
      provider.wallet.publicKey,
    );

    await globals.stakeMintToken.mintTo(
      sourceWallet,
      provider.wallet.publicKey,
      [],
      amount.toString(),
    );

    const ticket = await globals.pool.prepareTicket(provider.wallet.publicKey);

    await globals.pool.addStake(provider, amount, ticket, {
      stakeVault: globals.stakeVault,
      sourceAuthority: provider.wallet.publicKey,
      sourceWallet,
      staker: provider.wallet.publicKey,
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.stakeAcquiredAmount.eq(amount));

    const ticketAccount = await program.account.ticket.fetch(ticket.publicKey);
    console.log('Data: ', ticketAccount);

    assert.ok(ticketAccount.stakedAmount.eq(amount));
    assert.ok(ticketAccount.authority.equals(provider.wallet.publicKey));

    const stakeVault = await poolClient.utils.getTokenAccount(provider, globals.stakeVault);
    assert.ok(stakeVault.amount.eq(new anchor.BN(100)));

    globals.ticket = ticket;
    globals.userWallet = sourceWallet;
  });

  it('Removes the stake from the pool', async () => {
    const amount = new anchor.BN(50);

    await globals.pool.removeStake(provider, amount, {
      staker: provider.wallet.publicKey,
      ticket: globals.ticket.publicKey,
      stakeVault: globals.stakeVault,
      targetWallet: globals.userWallet
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.stakeAcquiredAmount.eq(amount));

    const targetWallet = await poolClient.utils.getTokenAccount(provider, globals.userWallet);
    assert.ok(targetWallet.amount.eq(amount));

    const stakeVault = await poolClient.utils.getTokenAccount(provider, globals.stakeVault);
    assert.ok(stakeVault.amount.eq(amount));
  });

  it('Adds the reward to the pool', async () => {
    const amount = new anchor.BN(100);
    // has to mint to the new account since minting
    // to the old one hangs solana node :(
    const sourceWallet = await poolClient.utils.createTokenAccount(
      provider,
      globals.stakeMintToken.publicKey,
      provider.wallet.publicKey,
    );

    await globals.stakeMintToken.mintTo(
      sourceWallet,
      provider.wallet.publicKey,
      [],
      amount.toString(),
    );

    await globals.pool.addReward(provider, amount, {
      stakeVault: globals.stakeVault,
      sourceAuthority: provider.wallet.publicKey,
      sourceWallet,
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.depositedRewardAmount.eq(amount));

    const stakeVault = await poolClient.utils.getTokenAccount(provider, globals.stakeVault);
    assert.ok(stakeVault.amount.eq(new anchor.BN(150)));
  });

  it('Claims the reward from the pool', async () => {
    const amountBefore = new anchor.BN(50);

    let poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    assert.ok(poolAccount.stakeAcquiredAmount.eq(amountBefore));

    let ticketAccount = await program.account.ticket.fetch(globals.ticket.publicKey);
    assert.ok(ticketAccount.stakedAmount.eq(amountBefore));

    poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    assert.ok(poolAccount.stakeAcquiredAmount.eq(new anchor.BN(50)));

    let stakeVault = await poolClient.utils.getTokenAccount(provider, globals.stakeVault);
    console.log('Stake vault holds ', stakeVault.amount.toNumber());
    assert.ok(stakeVault.amount.eq(new anchor.BN(150)));

    // waiting till pool expiration
    if (Date.now() < globals.endLockupTs.toNumber() * 1000) {
      await poolClient.utils.sleep(globals.endLockupTs.toNumber() * 1000 - Date.now() + 5000);
    }

    await globals.pool.claimReward(provider, {
      staker: provider.wallet.publicKey,
      ticket: globals.ticket.publicKey,
      stakeVault: globals.stakeVault,
      targetWallet: globals.userWallet,
    });

    stakeVault = await poolClient.utils.getTokenAccount(provider, globals.stakeVault);
    console.log('Stake vault holds ', stakeVault.amount.toNumber());
    assert.ok(stakeVault.amount.eq(new anchor.BN(0)));

    const targetWallet = await poolClient.utils.getTokenAccount(provider, globals.userWallet);
    console.log('Target wallet holds ', targetWallet.amount.toNumber());
    // 100 - 100 + 50 + 150
    assert.ok(targetWallet.amount.eq(new anchor.BN(200)));

    try {
      const ticketAccount = await program.account.ticket.fetch(globals.ticket.publicKey);
      assert.ok(false, "ticket should be deleted");
    } catch {
      assert.ok(true);
    }
  });
});
