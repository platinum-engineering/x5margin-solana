## Пример использования

```js
// Создаем клиент для девелоперского кластера соланы.
let client = ApiClient.devnet();
client.get_slot();

// Инициализируем публичный ключ контракта. Это константа
// в рамках отдельного кластера соланы.
let program_id = Pk.new("test-program-id");
let pool_client = PoolClient.new(client, program);
// Запрашиваем все пулы контракта.
let pools = pool_client.get_pools();

let pool = pools[0]; // возьмем первый пул для примера

pool.max_pool_size(); // максимальный размер пула
pool.total_pool_deposits(); // размер пула на данный момент
pool.total_rewards(); // общий размер наград в пуле за все время
pool.rewards_remaining(); // текущий размер наград в пуле
pool.start_date(); // дата создания пула (unix timestamp в секундах)
pool.end_date(); // дата окончания работы пула (unix timestamp в секундах)

let instructions = Instructions.new();

instructions.add(
    // Добавляем монеты в пул.
    pool.stake(
          10000 // количество монет
        , staker_key
        , staker_ticket_key
        , aux_wallet_key
    )
);

instructions.add(
    // Забираем монеты из пула.
    pool.unstake(
          1000 // количество монет
    )
);

instructions.add(
    // Забираем награду из пула.
    pool.claim_reward()
);

// Нужно интегрироваться с кошельком -- подключить провайдера
// пользовательских данных, который позволит подписывать транзакции
// от имени пользователя.
// https://github.com/solana-labs/wallet-adapter/ -- список кошельков
// и адаптеров к ним.
let tx = instructions.to_transaction(provider.publicKey);
tx.feePayer = provider.publicKey;

const anyTx = tx;
anyTx.recentBlockhash = (
    await client.get_recent_blockhash()
).blockhash;

let signed = await provider.signTransaction(anyTx);
let signature = await connection.sendRawTransaction(signed.serialize());
await connection.confirmTransaction(signature);
```

## Локальная сборка и запуск тестового сервера

1. Нужны nodejs, npm/yarn.

2. wasm-pack
https://github.com/wasm-tool/wasm-pack-plugin
https://rustwasm.github.io/wasm-pack/installer/

3. Таргет в WASM для rustc.

```
rustup target add wasm32-unknown-unknown
```

4. Установка зависимостей и запуск тестового сервера.

```
yarn install
yarn run serve
```
