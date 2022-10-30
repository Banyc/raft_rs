use std::collections::HashSet;

use crate::{Facts, Node, Term};

use super::{Follower, Leader};

pub struct Candidate {
    facts: Facts,
    term: Term,
    votes_from: HashSet<Node>,
}

impl Candidate {
    #[must_use]
    pub fn new(facts: Facts, term: Term) -> Self {
        Self {
            facts,
            term,
            votes_from: HashSet::from_iter(vec![facts.id]),
        }
    }

    /// - Upgrades the term if the given term is greater than the current term.
    /// - `self` becomes Follower if the given term is greater than the current term.
    #[must_use]
    pub fn try_upgrade_term(self, term: Term) -> TryUpgradeTermRes {
        if self.term < term {
            // follow the new term
            let follower = Follower::new(self.facts, term);

            TryUpgradeTermRes::Upgraded(follower)
        } else {
            TryUpgradeTermRes::NotUpgraded(self)
        }
    }

    #[must_use]
    pub fn emit(&self) -> RequestVote {
        RequestVote {
            term: self.term,
            from: self.facts.id,
        }
    }

    #[must_use]
    pub fn start_new_election(self) -> Candidate {
        Candidate::new(self.facts, self.term + 1)
    }

    pub fn receive_vote(
        mut self,
        from: Node,
        term: Term,
        vote_granted: bool,
    ) -> Result<ReceiveVoteRes, ReceiveVoteError> {
        if self.term < term {
            return Err(ReceiveVoteError::UpgradeTerm);
        }

        if term < self.term {
            return Ok(ReceiveVoteRes::NotUpgraded(self));
        }

        if vote_granted {
            // add up the vote
            self.votes_from.insert(from);

            // check if we have enough votes
            if self.votes_from.len() * 2 > self.facts.nodes {
                // become leader
                let leader = Leader::new(self.facts, self.term);

                Ok(ReceiveVoteRes::Upgraded(leader))
            } else {
                // keep waiting for more votes
                Ok(ReceiveVoteRes::NotUpgraded(self))
            }
        } else {
            // keep waiting for more votes
            Ok(ReceiveVoteRes::NotUpgraded(self))
        }
    }

    pub fn receive_ping(self, term: Term) -> Result<ReceivePingRes, ReceivePingError> {
        if self.term < term {
            return Err(ReceivePingError::UpgradeTerm);
        }

        if term < self.term {
            return Ok(ReceivePingRes::NotUpgraded(self));
        }

        // become follower
        let follower = Follower::new(self.facts, self.term);

        Ok(ReceivePingRes::Upgraded(follower))
    }

    pub fn term(&self) -> Term {
        self.term
    }

    pub fn facts(&self) -> &Facts {
        &self.facts
    }
}

pub enum TryUpgradeTermRes {
    Upgraded(Follower),
    NotUpgraded(Candidate),
}

pub struct RequestVote {
    pub term: Term,
    pub from: Node,
}

#[derive(Debug)]
pub enum ReceiveVoteError {
    UpgradeTerm,
}

pub enum ReceiveVoteRes {
    Upgraded(Leader),
    NotUpgraded(Candidate),
}

#[derive(Debug)]
pub enum ReceivePingError {
    UpgradeTerm,
}

pub enum ReceivePingRes {
    Upgraded(Follower),
    NotUpgraded(Candidate),
}