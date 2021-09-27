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

let stake_instruction = pool.stake(
    1000,
    staker_key,
    staker_ticket_key,
    aux_wallet_key
);

let payer = Pk.new("payer-public-key");

let instructions = Instructions.new();
instructions.push(stake_instruction);

let tx = transaction_signed_with_payer(
    instructions,
    payer,
    signers,
    recent_blockhash
);
client
    .send_transaction(tx)
    .and_then(tx_signature =>
        console.log(tx_signature);
    );
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
