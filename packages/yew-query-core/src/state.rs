use crate::Error;

/// Represents the state of a query.
#[derive(Clone, Debug)]
pub enum QueryState {
    /// The query is stopped or not had started.
    Idle,

    /// The query is loading the data for the first time.
    Loading,

    /// The query has finished loading the data.
    Ready,

    /// The query failed to load the data.
    Failed(Error),
}

impl QueryState {
    /// Returns `true` if the query is stopped or had not started.
    pub fn is_idle(&self) -> bool {
        matches!(self, QueryState::Idle)
    }

    /// Returns `true` if the query is loading.
    pub fn is_loading(&self) -> bool {
        matches!(self, QueryState::Loading)
    }

    /// Returns `true` if the query had loaded the data.
    pub fn is_ready(&self) -> bool {
        matches!(self, QueryState::Ready)
    }

    /// Returns `true` if the query had failed to load the data.
    pub fn is_failed(&self) -> bool {
        matches!(self, QueryState::Failed(_))
    }
}
