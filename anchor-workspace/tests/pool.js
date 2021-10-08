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

    const topupDuration = new anchor.BN(200);
    const lockupDuration = new anchor.BN(1000);
    const targetAmount = new anchor.BN(10000);
    const rewardAmount = new anchor.BN(1000);

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

    const poolAccount = await program.account.pool.fetch(pool.publicKey);
    console.log('Data: ', poolAccount);

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

    await poolClient.addStake(amount, ticket, {
      pool: globals.pool.publicKey,
      stakeVault: globals.stakeVault,
      sourceAuthority: provider.wallet.publicKey,
      sourceWallet,
    });

    const poolAccount = await program.account.pool.fetch(globals.pool.publicKey);
    console.log('Data: ', poolAccount);

    assert(poolAccount.stakeAcquiredAmount.eq(amount));

    const ticketAccount = await program.account.ticket.fetch(ticket.publicKey);
    console.log('Data: ', ticketAccount);

    assert.ok(ticketAccount.stakedAmount.eq(amount));
    assert.ok(ticketAccount.authority.equals(provider.wallet.publicKey));

    globals.ticket = ticket;
    globals.userWallet = sourceWallet;
  });

  // todo: remove stake
  // todo: claim reward
  // todo: add reward
  // todo: get pools
});
