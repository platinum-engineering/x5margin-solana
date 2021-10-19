use serde_json::{json, Value};
use solana_api_types::{Keypair, Pubkey, Signer};
use solar_macros::{parse_base58, parse_pubkey};

// Token mints

pub const USDC_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "5BJskWFMuVQe3aPPh3ZSUHrURtihurjJvgdLgxCy4vpGMMCjNH44Ux46nBWuJMauZPeGWoEsmBtHxHkAScvTJP7g"
);
pub const USDT_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "4VX9Mqw6K3JeY42CFmqXXmHxNaDjw6XB8JzwjhDNBGbXJZDDBox73uGTQDTA6kX9ADVwRKfemU8pPY4vDmNBvFu2"
);
pub const BTC_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "5bsiLujuYqmAKgkyfp6xTuA2MkSN9KKGmHCLbZ9QdxxScR55vTSPA2R423mHdYDBKnPTL5vvpbDC42fyvjqRiKhW"
);
pub const RAY_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "36CkFdcyTjEfqMBUZSwEytWJiU64EbTgEJBWUdDzN9E2w8WSmXwRLWRz6GV9TDNUPHCjBwr1B6wLYiJqquVwXm2P"
);

pub const USDC_MINT_PUBKEY: Pubkey = parse_pubkey!("DEkvtFw8yRbTyUFF3WPizTYHLTjnbjsAYaFdgyZ8j5rS");
pub const USDT_MINT_PUBKEY: Pubkey = parse_pubkey!("Gktyk3qNiCYTkTukjR2BTVGLeV4zhoFjWFa6TSLYjiQp");
pub const BTC_MINT_PUBKEY: Pubkey = parse_pubkey!("rUDqhaCACjP59hJ8CFVePHEcT2Jqr8gREiHKAkLU4je");
pub const RAY_MINT_PUBKEY: Pubkey = parse_pubkey!("8BV3aMG4E45RTzZ6sn2pLMXcUzeBSi2xBFZLauFA72vj");

pub fn usdc_mint_keypair() -> Keypair {
    Keypair::from_bytes(&USDC_MINT_PRIVATE).expect("infallible")
}

pub fn usdt_mint_keypair() -> Keypair {
    Keypair::from_bytes(&USDT_MINT_PRIVATE).expect("infallible")
}

pub fn btc_mint_keypair() -> Keypair {
    Keypair::from_bytes(&BTC_MINT_PRIVATE).expect("infallible")
}

// LP Mints

pub const USDC_BTC_LP_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "55QJTmK5rNoaqdcPvzCpndCuvmUosXrdnxcrHPtdAFLZM1AcMnahPB8FFiXMksCifLq3iPbh7YML4poYNoseaAeQ"
);
pub const USDC_BTC_LP_MINT_PUBKEY: Pubkey =
    parse_pubkey!("knQt4coDL5QDjh4ri8BGxobFevPMCthsQJRg76ED5K2");

pub const USDC_RAY_LP_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "5vJT9Wu7jkR1C6538XqmpCfX27N1hinQXswu4Fimy8rP6M1JL43aGpSEcooXAQkKb5Yj5a8LQgkUkGWijqjdodwd"
);
pub const USDC_RAY_LP_MINT_PUBKEY: Pubkey =
    parse_pubkey!("3NCmHqS13gsTriJaobqTV2oULPKnAviUWg7KDh28CVLT");

pub const BTC_RAY_LP_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "3E9cffNR5wQVth5PxK5a3ib4PqgtRsPyEqxmnkHxiRRXQg6Yc4Qjn9wJX6iUX7gFSUZQXANP3pefiriUPH74vunK"
);
pub const BTC_RAY_LP_MINT_PUBKEY: Pubkey =
    parse_pubkey!("J39zHbwUvLhnwwi5oXzwdMWAusahtLBiYwBXnT2gU9iT");

pub const USDT_BTC_LP_MINT_PRIVATE: [u8; 64] = parse_base58!(
    "54vdjejTf7E5Zeu4j9JNDrKUF3e9of28JGP6WFMtShHVHfLSRgpWU6W5KEcbwrewDduficWxui9rV5WWhASJNrUf"
);
pub const USDT_BTC_LP_MINT_PUBKEY: Pubkey =
    parse_pubkey!("fcsYhBnpV1Qn1hHaZKzzW4m5RyAg7BAr4aTRPJyJn4P");

pub fn usdc_btc_lp_mint_keypair() -> Keypair {
    Keypair::from_bytes(&USDC_BTC_LP_MINT_PRIVATE).expect("infallible")
}

pub fn usdc_ray_lp_mint_keypair() -> Keypair {
    Keypair::from_bytes(&USDC_RAY_LP_MINT_PRIVATE).expect("infallible")
}

pub fn btc_ray_lp_mint_keypair() -> Keypair {
    Keypair::from_bytes(&BTC_RAY_LP_MINT_PRIVATE).expect("infallible")
}

pub fn usdt_btc_lp_mint_keypair() -> Keypair {
    Keypair::from_bytes(&USDT_BTC_LP_MINT_PRIVATE).expect("infallible")
}

// Authorities

pub const DEFAULT_AUTHORITY_PRIVATE: [u8; 64] = parse_base58!(
    "2CkhJSvLMvBQWraKPY7edsZqy4uYCRqm8bXxBJmARe1KyvEEWDvC99Er9SQhh2sqjyYAVFYJyzm4i1xWU2BaTFWt"
);
pub const DEFAULT_AUTHORITY_PUBKEY: Pubkey =
    parse_pubkey!("BK3zyLydBiJdeJG6wyAU6qAQrxqaAyFWQ3be2nC5GgEx");

pub const DEFAULT_PAYER_PRIVATE: [u8; 64] = parse_base58!(
    "37qDJwHG2yeeszEiczZ77H4fhV8FqThKkv3ZsCBcz8G5UMRsvvnFzSaR1FPd2kmoJCq6u6EmPEWr4rpLHVcZBETL"
);
pub const DEFAULT_PAYER_PUBKEY: Pubkey =
    parse_pubkey!("6CKToxutniug3yoj2hj7kM1bUEmSCp8WXckdvoBkjxcJ");

pub fn ray_mint_keypair() -> Keypair {
    Keypair::from_bytes(&RAY_MINT_PRIVATE).expect("infallible")
}

pub fn default_authority_keypair() -> Keypair {
    Keypair::from_bytes(&DEFAULT_AUTHORITY_PRIVATE).expect("infallible")
}

pub fn default_payer_keypair() -> Keypair {
    Keypair::from_bytes(&DEFAULT_PAYER_PRIVATE).expect("infallible")
}

pub fn generate_raydium_lp_json() -> Value {
    let pairs = [
        (
            "USDC-BTC",
            usdc_btc_lp_mint_keypair(),
            usdc_mint_keypair(),
            btc_mint_keypair(),
            parse_pubkey!("HyMT93Bw3TQyeDETh1HWTsRPx4AAs37jnHEaL2VUpQW8"),
            0.00003,
        ),
        (
            "USDC-RAY",
            usdc_ray_lp_mint_keypair(),
            usdc_mint_keypair(),
            ray_mint_keypair(),
            parse_pubkey!("6BPWyE7zgR77pocp2YB3QinMScCsCZu8fCDB5kSQddHS"),
            0.05,
        ),
        (
            "BTC-RAY",
            btc_ray_lp_mint_keypair(),
            btc_mint_keypair(),
            ray_mint_keypair(),
            parse_pubkey!("5EgFtMMzMv1BbmGBm3TRvjhfNbzoj2Tmp7AwXm5n44Qb"),
            1500.0,
        ),
        (
            "USDT-BTC",
            usdt_btc_lp_mint_keypair(),
            usdt_mint_keypair(),
            btc_mint_keypair(),
            parse_pubkey!("7djx1ZqmBP6X7uYqHKrJmftLY2StuUxeR1keA2zZFhxE"),
            0.00003,
        ),
    ];

    let mut pair_jsons: Vec<Value> = vec![];
    for (name, lp, a, b, market, price) in pairs {
        let pair = json!({
            "name": name,
            "pair_id": format!("{}-{}", a.pubkey(), b.pubkey()),
            "lp_mint": lp.pubkey().to_string(),
            "official": true,
            "liquidity": 1_000_000_000.424242,
            "market": market.to_string(),
            "volume_24h": 500_000_000.424242,
            "volume_24h_quote": 500_000_000.424242,
            "fee_24h": 25_000_000.424242,
            "fee_24h_quote": 25_000_000.424242,
            "volume_7d": 750_000_000.424242,
            "volume_7d_quote": 750_000_000.424242,
            "fee_7d": 50_000_000.424242,
            "fee_7d_quote": 50_000_000.424242,
            "price": price,
            "amm_id": market.to_string(),
        });

        pair_jsons.push(pair);
    }

    Value::Array(pair_jsons)
}
