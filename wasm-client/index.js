const rust = import('./pkg');

rust
    .then(m => {
        return m.run("rustwasm/wasm-bindgen").then((data) => {
            console.log(data);
        })
    })
    .catch(console.error);
