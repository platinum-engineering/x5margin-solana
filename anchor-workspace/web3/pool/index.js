const solana_web3 = require('@solana/web3.js');
const anchor = require('@project-serum/anchor');
const utils = require('./utils');
const _ = require('lodash');
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
  const pools = (await program.account.pool.all())
    .map((pool) => new Pool(pool));
  return pools;
}

class Pool {
  constructor(data) {
    // pool objects come in two flavors:
    // { publicKey: "...", account: { props } }
    // { props }
    if (data.hasOwnProperty('account')) {
      _.extend(this, data.account);
      this.publicKey = data.publicKey;
    } else {
      _.extend(this, data);
    }
  }
  expectedAPY() {
    const rewardAmount = this.rewardAmount.toNumber().toFixed(20);
    const stakeTargetAmount = this.stakeTargetAmount.toNumber().toFixed(20);
    const rate = rewardAmount / stakeTargetAmount;

    const periodsInYear = this.calcPeriodsInYear();
    const annualRate = rate * periodsInYear;

    return calcAPY(annualRate, periodsInYear);
  }
  APY() {
    const depositedRewardAmount = this.depositedRewardAmount.toNumber().toFixed(20);
    const stakeAcquiredAmount = this.stakeAcquiredAmount.toNumber().toFixed(20);
    const rate = depositedRewardAmount / stakeAcquiredAmount;

    const periodsInYear = this.calcPeriodsInYear();
    const annualRate = rate * periodsInYear;

    return calcAPY(annualRate, periodsInYear);
  }
  calcPeriodsInYear() {
    const lockupDuration = this.lockupDuration.toNumber().toFixed(20);
    return Math.round(secondsInYear() / lockupDuration);
  }
  totalPoolDeposits() {
    return this.stakeAcquiredAmount;
  }
  maxPoolSize() {
    return this.stakeTargetAmount;
  }
  totalRewards() {
    return this.rewardAmount;
  }
  rewardsRemaining() {
    return this.depositedRewardAmount;
  }
  startDate() {
    return new Date(this.genesis.toNumber() * 1000);
  }
  endDate() {
    let date = this.startDate();
    date.setSeconds(date.getSeconds() + this.lockupDuration.toNumber());
    return date;
  }
  topupEndDate() {
    let date = this.startDate();
    date.setSeconds(date.getSeconds() + this.topupDuration.toNumber());
    return date;
  }
  timeToDeposit() {
    let now = new Date();
    let topupEnd = this.topupEndDate();
    return Math.ceil((topupEnd - now) / 1000);
  }
  timeUntilWithdrawal() {
    let now = new Date();
    let lockupEnd = this.endDate();
    return Math.ceil((lockupEnd - now) / 1000);
  }
}

function leapYear(year) {
  return ((year % 4 == 0) && (year % 100 != 0)) || (year % 400 == 0);
}

function secondsInYear() {
  const year = new Date().getFullYear();
  const days = leapYear(year) ? 366 : 365;

  return days * 24 * 60 * 60;
}

function calcAPY(annualRate, periodsInYear) {
  return (1 + annualRate / periodsInYear) ** periodsInYear - 1;
}

module.exports = {
  addStake,
  removeStake,
  addReward,
  claimReward,
  getPools,
  Pool,
};
