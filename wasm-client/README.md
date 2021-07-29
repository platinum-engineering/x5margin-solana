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
