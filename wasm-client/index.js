const rust = import('./pkg');

rust
    .then(m => {
        m.init_rust_logs();

        let client = m.ApiClient.devnet();

        client.get_account_info("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").then((r) => {
            console.log(r);
        });

        client.get_program_accounts("4Nd1mBQtrMJVYVfKf2PJy9NZUZdTAsp7D4xWLs4gDB4T").then((r) => {
            console.log(r);
        });

        client.get_slot().then((r) => {
            console.log(r);
        });

        client.get_multiple_accounts([
            "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg",
            "4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA"
        ], {
            encoding: "base58",
        }).then((r) => {
            console.log(r);
        });

        client.get_signature_statuses([
            "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW",
            "5j7s6NiJS3JAkvgkoc18WVAsiSaci2pxB2A6ueCJP4tprA2TFg9wSyTLeYouxPBJEMzJinENTkpA52YStRW5Dia7"
        ], {
            searchTransactionHistory: true,
        }).then((r) => {
            console.log(r);
        })

        client.get_signatures_for_address(
            "Vote111111111111111111111111111111111111111",
            {
                limit: 1,
            }
        ).then((r) => {
            console.log(r);
        });

        client.get_transaction(
            "2nBhEBYYvfaAe16UMNqRHre4YNSskvuYgx3M6E4JP1oDYvZEJHvoPzyUidNgNX5r9sTyN1J9UxtbCXy2rqYcuyuv",
            {
                encoding: "json",
            }
        ).then((r) => {
            console.log(r);
        });

        client.request_airdrop(
            "83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri",
            BigInt(1000000000)
        ).then((r) => {
            console.log(r);
        })
    })
    .catch(console.error);
