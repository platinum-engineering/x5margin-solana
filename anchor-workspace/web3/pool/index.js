const solana_web3 = require('@solana/web3.js');
const anchor = require('@project-serum/anchor');
const utils = require('./utils');
const idl = require('./idl.json');

const programId = new solana_web3.PublicKey(idl.metadata.address);

async function addStake(provider, amount, ticket, accounts) {
  const program = new anchor.Program(idl, programId, provider);

  return await program.rpc.addStake(
    amount,
    {
      accounts: {
        tokenProgram: utils.TOKEN_PROGRAM_ID,
        pool: accounts.pool,
        ticket: ticket.publicKey,
        stakeVault: accounts.stakeVault,
        sourceAuthority: accounts.sourceAuthority,
        sourceWallet: accounts.sourceWallet,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
      signers: [ticket],
      instructions: [
        await program.account.ticket.createInstruction(ticket),
      ],
    }
  );
}

async function removeStake(provider, amount, accounts) {
  const program = new anchor.Program(idl, programId, provider);

  return await program.rpc.removeStake(
    amount,
    {
      accounts: {
        tokenProgram: utils.TOKEN_PROGRAM_ID,
        pool: accounts.pool,
        staker: accounts.staker,
        ticket: accounts.ticket,
        poolAuthority: accounts.poolAuthority,
        stakeVault: accounts.stakeVault,
        targetWallet: accounts.targetWallet,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    }
  );
}

async function addReward(provider, amount, accounts) {
  const program = new anchor.Program(idl, programId, provider);

  return await program.rpc.addReward(
    amount,
    {
      accounts: {
        tokenProgram: utils.TOKEN_PROGRAM_ID,
        pool: accounts.pool,
        stakeVault: accounts.stakeVault,
        sourceAuthority: accounts.sourceAuthority,
        sourceWallet: accounts.sourceWallet,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    }
  );
}

async function claimReward(provider, accounts) {
  const program = new anchor.Program(idl, programId, provider);

  return await program.rpc.claimReward(
    {
      accounts: {
        tokenProgram: utils.TOKEN_PROGRAM_ID,
        pool: accounts.pool,
        staker: accounts.staker,
        ticket: accounts.ticket,
        poolAuthority: accounts.poolAuthority,
        stakeVault: accounts.stakeVault,
        targetWallet: accounts.targetWallet,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    }
  )
}

async function getPools(provider) {
  const program = new anchor.Program(idl, programId, provider);
  return await program.account.pool.all();
}

module.exports = {
  addStake,
  removeStake,
  addReward,
  claimReward,
  getPools,
};
