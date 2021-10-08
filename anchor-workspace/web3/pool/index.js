const anchor = require('@project-serum/anchor');
const utils = require('./utils');

const program = anchor.workspace.Pool;

async function addStake(amount, ticket, accounts) {
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

// todo: remove stake
// todo: claim reward
// todo: get pools

module.exports = {
  addStake,
};
