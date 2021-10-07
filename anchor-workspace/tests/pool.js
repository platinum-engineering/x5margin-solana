const anchor = require('@project-serum/anchor');
const assert = require("assert");
const spl = require("@solana/spl-token");
const TokenInstructions = require("@project-serum/serum").TokenInstructions;

const TOKEN_PROGRAM_ID = new anchor.web3.PublicKey(
  TokenInstructions.TOKEN_PROGRAM_ID.toString()
);

async function createMint(provider, authority) {
  if (authority === undefined) {
    authority = provider.wallet.publicKey;
  }
  const mint = await spl.Token.createMint(
    provider.connection,
    provider.wallet.payer,
    authority,
    null,
    6,
    TOKEN_PROGRAM_ID
  );
  return mint;
}

async function createTokenAccount(provider, mint, owner) {
  const token = new spl.Token(
    provider.connection,
    mint,
    TOKEN_PROGRAM_ID,
    provider.wallet.payer
  );
  let vault = await token.createAccount(owner);
  return vault;
}

describe('pool', () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Pool;

  it('Initializes the pool', async () => {
    const administrator = anchor.web3.Keypair.generate();
    const pool = anchor.web3.Keypair.generate();

    const stake_mint_token = await createMint(provider);
    const stakeMint = stake_mint_token.publicKey;

    const [poolAuthority, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [
        pool.publicKey.toBuffer(),
        administrator.publicKey.toBuffer(),
      ],
      program.programId
    );

    const stakeVault = await createTokenAccount(
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

    assert(poolAccount.topupDuration.toNumber() === topupDuration.toNumber());
    assert(poolAccount.lockupDuration.toNumber() === lockupDuration.toNumber());
    assert(poolAccount.stakeTargetAmount.toNumber() === targetAmount.toNumber());
    assert(poolAccount.rewardAmount.toNumber() === rewardAmount.toNumber());
  });

  // it("Updates the previously created account", async () => {
  //   const baseAccount = _baseAccount;

  //   await program.rpc.update("Some new data", {
  //     accounts: {
  //       baseAccount: baseAccount.publicKey,
  //     }
  //   });

  //   const account = await program.account.baseAccount.fetch(baseAccount.publicKey);
  //   console.log("Updated data: ", account.data);
  //   assert.ok(account.data === "Some new data");
  //   console.log("all account data: ", account);
  //   console.log("all data: ", account.dataList);
  //   assert.ok(account.dataList.length === 2);
  // });
});
