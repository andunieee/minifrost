//! Amounts, addresses and network names as the user reads them.

/// 1234567 -> "0.01234567 BTC"
pub fn format_btc(sats: i64) -> String {
    let abs = sats.unsigned_abs();
    let sign = if sats < 0 { "-" } else { "" };
    format!("{sign}{}.{:08} BTC", abs / 100_000_000, abs % 100_000_000)
}

/// 1234567 -> "1 234 567 sats"
pub fn format_sats(sats: i64) -> String {
    let digits = sats.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    let sign = if sats < 0 { "-" } else { "" };
    format!("{sign}{out} sats")
}

/// Break a long address into groups of four so it can be compared by eye.
pub fn group_address(address: &str) -> String {
    let chars: Vec<char> = address.chars().collect();
    chars
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The user-facing name of a network id as stored by the engines (`signet` is Mutinynet here).
pub fn network_label(network: &str) -> &str {
    match network {
        "signet" => "mutinynet",
        "bitcoin" => "mainnet",
        other => other,
    }
}

/// The networks a wallet can be on: (id, name in a list).
pub const NETWORKS: [(&str, &str); 3] = [
    ("signet", "Mutinynet"),
    ("testnet", "Testnet"),
    ("bitcoin", "Bitcoin mainnet (real money)"),
];

/// The first `n` characters of an id, then an ellipsis.
pub fn short_id(id: &str, n: usize) -> String {
    format!("{}…", &id[..id.len().min(n)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts() {
        assert_eq!(format_btc(1234567), "0.01234567 BTC");
        assert_eq!(format_btc(-150_000_000), "-1.50000000 BTC");
        assert_eq!(format_sats(1234567), "1 234 567 sats");
        assert_eq!(format_sats(999), "999 sats");
        assert_eq!(format_sats(-1000), "-1 000 sats");
    }

    #[test]
    fn addresses() {
        assert_eq!(group_address("bc1qabcdefg"), "bc1q abcd efg");
    }
}
