//! Token and file budget enforcement.
//!
//! The [`Pruner`] is a stateful budget gate: candidates are offered in
//! relevance order and each is either kept or dropped with a reason once a
//! `max_files` or `max_tokens` cap is reached.

use crate::models::DroppedReason;

/// A candidate offered to the budget gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneCandidate {
    /// Repository-relative path.
    pub path: String,
    /// Approximate token count for this file's content.
    pub tokens: usize,
}

/// The gate's decision for a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PruneOutcome {
    /// The candidate fits within the budget and is kept.
    Keep,
    /// The candidate exceeds the budget.
    Drop {
        /// Why the candidate was dropped.
        reason: DroppedReason,
        /// Human-readable detail.
        detail: String,
    },
}

/// A stateful token/file budget gate.
#[derive(Debug, Clone)]
pub struct Pruner {
    max_files: Option<usize>,
    max_tokens: Option<usize>,
    files_used: usize,
    tokens_used: usize,
}

impl Pruner {
    /// Create a gate with the given caps. `None` disables a cap.
    pub fn new(max_files: Option<usize>, max_tokens: Option<usize>) -> Self {
        Pruner {
            max_files,
            max_tokens,
            files_used: 0,
            tokens_used: 0,
        }
    }

    /// Offer the next candidate (in descending relevance order).
    pub fn offer(&mut self, candidate: &PruneCandidate) -> PruneOutcome {
        if self.max_files.is_some_and(|limit| self.files_used >= limit) {
            return PruneOutcome::Drop {
                reason: DroppedReason::BudgetExceeded,
                detail: format!("max files budget reached ({})", self.max_files.unwrap()),
            };
        }
        if self
            .max_tokens
            .is_some_and(|limit| self.tokens_used + candidate.tokens > limit)
        {
            return PruneOutcome::Drop {
                reason: DroppedReason::BudgetExceeded,
                detail: format!(
                    "token budget would be exceeded ({} needed, {} of {} used)",
                    candidate.tokens,
                    self.tokens_used,
                    self.max_tokens.unwrap()
                ),
            };
        }
        self.files_used += 1;
        self.tokens_used += candidate.tokens;
        PruneOutcome::Keep
    }

    /// Number of tokens consumed so far.
    pub fn tokens_used(&self) -> usize {
        self.tokens_used
    }

    /// Number of files kept so far.
    pub fn files_used(&self) -> usize {
        self.files_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_everything_without_budget() {
        let mut pruner = Pruner::new(None, None);
        for index in 0..10 {
            assert_eq!(
                pruner.offer(&PruneCandidate {
                    path: format!("f{index}"),
                    tokens: 10,
                }),
                PruneOutcome::Keep
            );
        }
        assert_eq!(pruner.files_used(), 10);
    }

    #[test]
    fn enforces_file_budget() {
        let mut pruner = Pruner::new(Some(2), None);
        assert_eq!(
            pruner.offer(&PruneCandidate {
                path: "a".into(),
                tokens: 1
            }),
            PruneOutcome::Keep
        );
        assert_eq!(
            pruner.offer(&PruneCandidate {
                path: "b".into(),
                tokens: 1
            }),
            PruneOutcome::Keep
        );
        assert!(matches!(
            pruner.offer(&PruneCandidate {
                path: "c".into(),
                tokens: 1
            }),
            PruneOutcome::Drop {
                reason: DroppedReason::BudgetExceeded,
                ..
            }
        ));
    }

    #[test]
    fn enforces_token_budget() {
        let mut pruner = Pruner::new(None, Some(10));
        assert_eq!(
            pruner.offer(&PruneCandidate {
                path: "a".into(),
                tokens: 6
            }),
            PruneOutcome::Keep
        );
        assert_eq!(
            pruner.offer(&PruneCandidate {
                path: "b".into(),
                tokens: 4
            }),
            PruneOutcome::Keep
        );
        assert!(matches!(
            pruner.offer(&PruneCandidate {
                path: "c".into(),
                tokens: 1
            }),
            PruneOutcome::Drop { .. }
        ));
        assert_eq!(pruner.tokens_used(), 10);
    }
}
