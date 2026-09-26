//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! Skeleton only: spec 001 M1 replaces this with the real CLI.

fn main() {
    println!("vernier {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_semver_shaped() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
