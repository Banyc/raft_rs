use crate::{election, log::Log, log_replication, Facts, Node, Term};

use super::follower::{self, Follower};

pub struct Leader {
    peers: Vec<Node>,
    election: election::Leader,
    log_replication: log_replication::Leader,
}

impl Leader {
    pub fn new(
        peers: Vec<Node>,
        facts: Facts,
        term: Term,
        log: Log,
    ) -> Result<Self, log_replication::leader::NewError> {
        let log_replication = log_replication::Leader::new(term, log, &peers)?;
        Ok(Self {
            peers,
            election: election::Leader::new(facts, term),
            log_replication,
        })
    }

    pub fn emit(
        &self,
        to: Node,
    ) -> Result<log_replication::leader::AppendEntriesReq, log_replication::leader::EmitError> {
        self.log_replication.emit(to)
    }

    #[must_use]
    pub fn receive_vote_req(
        self,
        from: Node,
        term: Term,
        last_log_index: Option<usize>,
        last_log_term: Option<Term>,
    ) -> (ReceiveVoteReqRes, bool) {
        let election = match self.election.try_upgrade_term(term) {
            election::leader::TryUpgradeTermRes::Upgraded(election) => {
                let follower = Follower::new(
                    self.peers,
                    *election.facts(),
                    term,
                    self.log_replication.into_log(),
                );

                let (res, vote_granted) =
                    follower.receive_vote_req(from, term, last_log_index, last_log_term);

                let follower = match res {
                    // SAFETY: We know that the term is the same as the one we just upgraded to.
                    follower::ReceiveVoteReqRes::TermUpgraded(_) => unreachable!(),

                    follower::ReceiveVoteReqRes::NotUpgraded(v) => v,
                };

                return (ReceiveVoteReqRes::TermUpgraded(follower), vote_granted);
            }
            election::leader::TryUpgradeTermRes::NotUpgraded(v) => v,
        };

        let leader = Self {
            peers: self.peers,
            election,
            log_replication: self.log_replication,
        };

        (ReceiveVoteReqRes::NotUpgraded(leader), false)
    }

    pub fn receive_append_entries_resp(
        &mut self,
        from: Node,
        res: log_replication::leader::AppendEntriesRes,
    ) -> Result<(), log_replication::leader::ReceiveAppendEntriesRespError> {
        self.log_replication.receive_append_entries_resp(from, res)
    }
}

pub enum ReceiveVoteReqRes {
    // - The follower should reset its election timer.
    TermUpgraded(Follower),

    NotUpgraded(Leader),
}