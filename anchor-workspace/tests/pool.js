const anchor = require('@project-serum/anchor');
const assert = require("assert");
const utils = require("../web3/pool/utils");
const poolClient = require("../web3/pool/index");

describe('pool', () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Pool;

  // Stuff to save between tests.
  let globals = {
    administrator: undefined,
    pool: undefined,
    poolAuthority: undefined,
    nonce: undefined,
    stakeMintToken: undefined,
    stakeVault: undefined,
    ticket: undefined,
    userWallet: undefined,
    endLockupTs: undefined,
  };

  it('Initializes the pool', async () => {
    const administrator = anchor.web3.Keypair.generate();
    const pool = anchor.web3.Keypair.generate();

    const stakeMintToken = await utils.createMint(provider);
    const stakeMint = stakeMintToken.publicKey;

    const [poolAuthority, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [
        pool.publicKey.toBuffer(),
        administrator.publicKey.toBuffer(),
      ],
      program.programId
    );

    const stakeVault = await utils.createTokenAccount(
      provider,
      stakeMint,
      poolAuthority
    );

    const topupDuration = new anchor.BN(3);
    const lockupDuration = new anchor.BN(6);
    const targetAmount = new anchor.BN(10000);
    const rewardAmount = new anchor.BN(100);

    await program.rpc.initializePool(
      nonce,
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
    assert.ok(pools[0].publicKey.equals(pool.publicKey));

    const poolAccount = pools[0].account;

    assert.ok(poolAccount.topupDuration.eq(topupDuration));
    assert.ok(poolAccount.lockupDuration.eq(lockupDuration));
    assert.ok(poolAccount.stakeTargetAmount.eq(targetAmount));
    assert.ok(poolAccount.rewardAmount.eq(rewardAmount));

    globals.administrator = administrator;
    globals.pool = pool;
    globals.poolAuthority = poolAuthority;
    globals.nonce = nonce;
    globals.stakeMintToken = stakeMintToken;
    globals.stakeVault = stakeVault;

    const nowTs = new anchor.BN(Date.now() / 1000);
    globals.endLockupTs = nowTs.add(lockupDuration);
  });

  it('Adds stake to the pool', async () => {
    const amount = new anchor.BN(100);
    const ticket = anchor.web3.Keypair.generate();

    const sourceWallet = await utils.createTokenAccount(
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

    await poolClient.addStake(provider, amount, ticket, {
      pool: globals.pool.publicKey,
      stakeVault: globals.stakeVault,
      sourceAuthority: provider.wallet.publicKey,
      sourceWallet,
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.stakeAcquiredAmount.eq(amount));

    const ticketAccount = await program.account.ticket.fetch(ticket.publicKey);
    console.log('Data: ', ticketAccount);

    assert.ok(ticketAccount.stakedAmount.eq(amount));
    assert.ok(ticketAccount.authority.equals(provider.wallet.publicKey));

    const stakeVault = await utils.getTokenAccount(provider, globals.stakeVault);
    assert.ok(stakeVault.amount.eq(new anchor.BN(100)));

    globals.ticket = ticket;
    globals.userWallet = sourceWallet;
  });

  it('Removes the stake from the pool', async () => {
    const amount = new anchor.BN(50);

    await poolClient.removeStake(provider, amount, {
      pool: globals.pool.publicKey,
      staker: provider.wallet.publicKey,
      ticket: globals.ticket.publicKey,
      poolAuthority: globals.poolAuthority,
      stakeVault: globals.stakeVault,
      targetWallet: globals.userWallet
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.stakeAcquiredAmount.eq(amount));

    const targetWallet = await utils.getTokenAccount(provider, globals.userWallet);
    assert.ok(targetWallet.amount.eq(amount));

    const stakeVault = await utils.getTokenAccount(provider, globals.stakeVault);
    assert.ok(stakeVault.amount.eq(amount));
  });

  it('Adds the reward to the pool', async () => {
    const amount = new anchor.BN(100);
    // has to mint to the new account since minting
    // to the old one hangs solana node :(
    const sourceWallet = await utils.createTokenAccount(
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

    await poolClient.addReward(provider, amount, {
      pool: globals.pool.publicKey,
      stakeVault: globals.stakeVault,
      sourceAuthority: provider.wallet.publicKey,
      sourceWallet,
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert.ok(poolAccount.depositedRewardAmount.eq(amount));

    const stakeVault = await utils.getTokenAccount(provider, globals.stakeVault);
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

    let stakeVault = await utils.getTokenAccount(provider, globals.stakeVault);
    console.log('Stake vault holds ', stakeVault.amount.toNumber());
    assert.ok(stakeVault.amount.eq(new anchor.BN(150)));

    // waiting till pool expiration
    if (Date.now() < globals.endLockupTs.toNumber() * 1000) {
      await utils.sleep(globals.endLockupTs.toNumber() * 1000 - Date.now() + 5000);
    }

    await poolClient.claimReward(provider, {
      pool: globals.pool.publicKey,
      staker: provider.wallet.publicKey,
      ticket: globals.ticket.publicKey,
      poolAuthority: globals.poolAuthority,
      stakeVault: globals.stakeVault,
      targetWallet: globals.userWallet,
    });

    stakeVault = await utils.getTokenAccount(provider, globals.stakeVault);
    console.log('Stake vault holds ', stakeVault.amount.toNumber());
    assert.ok(stakeVault.amount.eq(new anchor.BN(0)));

    const targetWallet = await utils.getTokenAccount(provider, globals.userWallet);
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
