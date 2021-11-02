const spl = require("@solana/spl-token");
const anchor = require('@project-serum/anchor');
const TokenInstructions = require("@project-serum/serum").TokenInstructions;
const serumCmn = require('@project-serum/common');

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
  if (owner === undefined) {
    owner = provider.wallet.publicKey;
  }
  const token = new spl.Token(
    provider.connection,
    mint,
    TOKEN_PROGRAM_ID,
    provider.wallet.payer
  );
  let vault = await token.createAccount(owner);
  return vault;
}

async function getTokenAccount(provider, addr) {
  return await serumCmn.getTokenAccount(provider, addr);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

module.exports = {
    createMint,
    createTokenAccount,
    getTokenAccount,
    sleep,
    TOKEN_PROGRAM_ID,
};
