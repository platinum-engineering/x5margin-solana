## Создание ключей и токена

```
solana-keygen new -o admin.json
solana-keygen new -o pool.json
spl-token create-token
spl-token create-account <pubkey from create-token>
```

Создали две пары ключей: одна для админа, вторая для самого пула.

## Вывод program derived address для инициализации пула

```
cargo run -p pool-cli --
--cluster localnet # заменить по необходимости
# адрес программы в локалнете именно такой
# меняем соответственно, если меняем сеть
--pool-program-id BHfLU4UsBdxBZk56GjpGAXkzu8B7JdMitGa9A1VTMmva
generate-pda
--administrator `solana-keygen pubkey ./admin.json` # выводим pubkey по keypair
--pool `solana-keygen pubkey ./pool.json`

Generated PDA: ByKTbYdmbGD9d3b4NPkDheJ3E3YBYTSpbjgfuwsM42bY
Nonce: 253
```

Адрес и nonce укажем в другой команде.

## Создание пула

```
cargo run -p pool-cli --
--cluster localnet
--pool-program-id BHfLU4UsBdxBZk56GjpGAXkzu8B7JdMitGa9A1VTMmva
initialize
--administrator ./admin.json
--pool ./pool.json
# адрес выше (PDA)
--pool-authority ByKTbYdmbGD9d3b4NPkDheJ3E3YBYTSpbjgfuwsM42bY
# mint и token account выше
--stake-mint 63YnGfWna9HAyEXEn7QmTeuTYSczPSSxvsGbXModWQKT
--stake-vault HX5jjA2a2L9237YHzwUNj9VQpTQrj2eFuWqFCCSTCWHG
# нонс выше
--nonce 253
--lockup-duration 1000
--topup-duration 200
--reward-amount 1000
--target-amount 10000
```
