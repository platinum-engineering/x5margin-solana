## Создание ключей и токена

```
solana-keygen new -o admin.json
solana-keygen new -o pool.json
spl-token create-token
spl-token create-account <pubkey from create-token>
```

Создали две пары ключей: одна для админа, вторая для самого пула.

## Создание пула

```
cargo run -p pool-cli --
--cluster localnet
--pool-program-id BHfLU4UsBdxBZk56GjpGAXkzu8B7JdMitGa9A1VTMmva
initialize
--administrator ./admin.json
--pool ./pool.json
# mint и token account выше
--stake-mint 63YnGfWna9HAyEXEn7QmTeuTYSczPSSxvsGbXModWQKT
--stake-vault HX5jjA2a2L9237YHzwUNj9VQpTQrj2eFuWqFCCSTCWHG
--lockup-duration 1000
--topup-duration 200
--reward-amount 1000
--target-amount 10000
```
