//! Near-duplicate detection with a MinHash LSH pipeline.
//!
//! Content is tokenized into hashed tokens, then k-shingles (sliding windows)
//! form a set per file. A MinHash signature estimates Jaccard similarity
//! cheaply, and Locality-Sensitive Hashing banding buckets candidate pairs so
//! only promising pairs are compared exactly. Files whose estimated Jaccard
//! similarity reaches the configured threshold are unioned into a duplicate
//! group. The first member in the supplied priority order becomes the
//! representative; the remaining members are reported with their similarity to
//! it.
//!
//! Everything here is deterministic: hashing uses FNV-1a with fixed seeds, so
//! identical input produces identical output across runs and platforms.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Tunable knobs for near-duplicate detection.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Sliding-window size in tokens used to build shingles.
    pub shingle_size: usize,
    /// Number of MinHash functions per signature.
    pub signature_count: usize,
    /// Number of signature rows hashed into each LSH band.
    pub band_size: usize,
    /// Deterministic cap on shingles retained per file.
    pub max_shingles_per_file: usize,
    /// Minimum estimated Jaccard similarity to treat files as duplicates.
    pub similarity_threshold: f64,
}

impl Default for DedupConfig {
    fn default() -> Self {
        DedupConfig {
            shingle_size: 5,
            signature_count: 64,
            band_size: 4,
            max_shingles_per_file: 512,
            similarity_threshold: 0.8,
        }
    }
}

/// A collapsed near-duplicate group.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateGroup {
    /// Path kept as the group's representative.
    pub representative: String,
    /// Collapsed members with their similarity to the representative.
    pub members: Vec<DuplicateMember>,
}

/// A collapsed member and its similarity to the representative.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMember {
    /// Repository-relative path of the collapsed file.
    pub path: String,
    /// Estimated Jaccard similarity to the representative.
    pub similarity: f64,
}

/// Detect near-duplicate groups among `entries`, which map priority-ordered
/// `(path, content)` pairs. The earliest entry in a group (highest priority)
/// is chosen as its representative.
pub fn detect_near_duplicates(
    entries: &[(String, String)],
    config: &DedupConfig,
) -> Vec<DuplicateGroup> {
    if entries.len() < 2 {
        return Vec::new();
    }

    let salts = signature_salts(config.signature_count);
    let mut shingle_sets: Vec<HashSet<u64>> = Vec::with_capacity(entries.len());
    let mut signatures: Vec<Vec<u64>> = Vec::with_capacity(entries.len());
    for (_, content) in entries {
        let shingles = shingle_set(content, config);
        let signature = signature(&shingles, &salts, config.signature_count);
        shingle_sets.push(shingles);
        signatures.push(signature);
    }

    let mut uf = UnionFind::new(entries.len());
    let bands = config.signature_count / config.band_size;
    for band in 0..bands {
        let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
        for (index, signature) in signatures.iter().enumerate() {
            buckets
                .entry(band_key(signature, band, config.band_size))
                .or_default()
                .push(index);
        }
        for bucket in buckets.values() {
            for a in 0..bucket.len() {
                for b in (a + 1)..bucket.len() {
                    let (i, j) = (bucket[a], bucket[b]);
                    if jaccard(&shingle_sets[i], &shingle_sets[j]) >= config.similarity_threshold {
                        uf.union(i, j);
                    }
                }
            }
        }
    }

    let mut members_by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..entries.len() {
        members_by_root
            .entry(uf.find(index))
            .or_default()
            .push(index);
    }

    let mut groups = Vec::new();
    for (_, members) in members_by_root {
        if members.len() < 2 {
            continue;
        }
        let representative = members[0];
        let representative_set = &shingle_sets[representative];
        let mut collapsed = Vec::new();
        for &member in &members[1..] {
            collapsed.push(DuplicateMember {
                path: entries[member].0.clone(),
                similarity: jaccard(representative_set, &shingle_sets[member]),
            });
        }
        collapsed.sort_by(|left, right| left.path.cmp(&right.path));
        groups.push(DuplicateGroup {
            representative: entries[representative].0.clone(),
            members: collapsed,
        });
    }
    groups
}

/// FNV-1a 64-bit hash. Deterministic, seedable and allocation-free.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Hash the token stream of `content`: identifiers/numbers hashed whole, and
/// each significant punctuation character hashed alone.
fn token_hashes(content: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    for ch in content.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            buffer.push(ch.to_ascii_lowercase());
        } else {
            if !buffer.is_empty() {
                out.push(fnv1a_64(buffer.as_bytes()));
                buffer.clear();
            }
            if !ch.is_whitespace() {
                let mut encoded = [0u8; 4];
                out.push(fnv1a_64(ch.encode_utf8(&mut encoded).as_bytes()));
            }
        }
    }
    if !buffer.is_empty() {
        out.push(fnv1a_64(buffer.as_bytes()));
    }
    out
}

/// Build the (bounded, deduplicated) shingle set for `content`.
fn shingle_set(content: &str, config: &DedupConfig) -> HashSet<u64> {
    let tokens = token_hashes(content);
    if tokens.len() < config.shingle_size {
        return HashSet::new();
    }
    let mut set: BTreeSet<u64> = tokens
        .windows(config.shingle_size)
        .map(combine_shingles)
        .collect();
    if set.len() > config.max_shingles_per_file {
        let stride = set.len().div_ceil(config.max_shingles_per_file);
        set = set.iter().step_by(stride).copied().collect();
    }
    set.into_iter().collect()
}

/// Combine a shingle window into a single u64 with a deterministic mix.
fn combine_shingles(window: &[u64]) -> u64 {
    let mut hash = window[0];
    for &value in &window[1..] {
        hash = hash.rotate_left(9) ^ value;
    }
    hash
}

/// Deterministic per-hash-function salts.
fn signature_salts(count: usize) -> Vec<u64> {
    (0..count)
        .map(|index| fnv1a_64(format!("ink-signature-salt-{index}").as_bytes()))
        .collect()
}

/// Compute a MinHash signature: for each hash function, the minimum value of
/// that function over all shingles.
fn signature(shingle_set: &HashSet<u64>, salts: &[u64], signature_count: usize) -> Vec<u64> {
    (0..signature_count)
        .map(|index| {
            let salt = salts[index];
            shingle_set
                .iter()
                .map(|&shingle| fnv1a_64(&(shingle ^ salt).to_le_bytes()))
                .min()
                .unwrap_or(0)
        })
        .collect()
}

/// Combine one LSH band of a signature into a bucket key.
fn band_key(signature: &[u64], band: usize, band_size: usize) -> u64 {
    let start = band * band_size;
    let mut hash = 0x811c_9dc5u64;
    for &value in &signature[start..start + band_size] {
        hash = hash.rotate_left(11) ^ value;
    }
    hash
}

/// Exact Jaccard similarity between two shingle sets.
fn jaccard(left: &HashSet<u64>, right: &HashSet<u64>) -> f64 {
    let (small, large) = if left.len() < right.len() {
        (left, right)
    } else {
        (right, left)
    };
    if large.is_empty() {
        return 0.0;
    }
    let intersection = small.iter().filter(|s| large.contains(s)).count();
    let union = left.len() + right.len() - intersection;
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// A minimal disjoint-set structure for grouping duplicate clusters.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        UnionFind {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, mut node: usize) -> usize {
        while self.parent[node] != node {
            self.parent[node] = self.parent[self.parent[node]];
            node = self.parent[node];
        }
        node
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        match self.rank[left_root].cmp(&self.rank[right_root]) {
            std::cmp::Ordering::Less => self.parent[left_root] = right_root,
            std::cmp::Ordering::Greater => self.parent[right_root] = left_root,
            std::cmp::Ordering::Equal => {
                self.parent[right_root] = left_root;
                self.rank[left_root] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, content: &str) -> (String, String) {
        (path.to_owned(), content.to_owned())
    }

    fn group_by_representative(groups: &[DuplicateGroup]) -> BTreeMap<&str, Vec<&str>> {
        groups
            .iter()
            .map(|group| {
                (
                    group.representative.as_str(),
                    group
                        .members
                        .iter()
                        .map(|member| member.path.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    #[test]
    fn identical_files_are_collapsed() {
        let source = "fn main() {\n    let value = 42;\n    println!(\"{value}\");\n}\n";
        let entries = vec![
            entry("src/main.rs", source),
            entry("src/copy.rs", source),
            entry("src/other.rs", "pub struct Other;"),
        ];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        let by_rep = group_by_representative(&groups);
        assert!(by_rep
            .get("src/main.rs")
            .is_some_and(|members| members.contains(&"src/copy.rs")));
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn representative_follows_priority_order() {
        let source = "const value = 1;\nexport { value };\n";
        let entries = vec![
            entry("src/a.rs", source),
            entry("src/b.rs", source),
            entry("src/c.rs", source),
        ];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].representative, "src/a.rs");
        assert_eq!(groups[0].members.len(), 2);
    }

    #[test]
    fn distinct_files_are_not_grouped() {
        let entries = vec![
            entry("src/a.rs", "fn alpha() { let x = 1; }"),
            entry("src/b.rs", "fn beta() { let y = 2; }"),
            entry("src/c.rs", "struct Gamma { field: u8 }"),
        ];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        assert!(groups.is_empty());
    }

    #[test]
    fn short_or_empty_content_yields_no_group() {
        let entries = vec![entry("a.rs", "x"), entry("b.rs", "x")];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        assert!(groups.is_empty());
    }

    #[test]
    fn whitespace_only_differences_are_collapsed() {
        let base = "pub struct Config {\n  pub host: String,\n  pub port: u16,\n}\n";
        let reindented = "pub struct Config {\n\tpub host: String,\n\tpub port: u16,\n}\n\n";
        let entries = vec![
            entry("src/config.rs", base),
            entry("src/config_copy.rs", reindented),
        ];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].representative, "src/config.rs");
        assert!(groups[0].members[0].similarity > 0.95);
    }

    #[test]
    fn substantive_additions_below_threshold_are_not_grouped() {
        let base = "pub struct Config {\n  pub host: String,\n  pub port: u16,\n}\n";
        let extended =
            "pub struct Config {\n  pub host: String,\n  pub port: u16,\n  pub tls: bool,\n}\n";
        let entries = vec![
            entry("src/config.rs", base),
            entry("src/config2.rs", extended),
        ];
        let groups = detect_near_duplicates(&entries, &DedupConfig::default());
        // ~0.53 similarity is below the default 0.8 threshold, so the files
        // are kept as distinct candidates.
        assert!(groups.is_empty());
    }

    #[test]
    fn hashing_is_deterministic() {
        let entries = vec![
            entry("a.rs", "fn f() { let a = 1; }"),
            entry("b.rs", "fn f() { let a = 1; }"),
        ];
        let first = detect_near_duplicates(&entries, &DedupConfig::default());
        let second = detect_near_duplicates(&entries, &DedupConfig::default());
        assert_eq!(first, second);
    }
}
