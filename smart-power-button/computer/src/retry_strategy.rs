use std::time::Duration;

use try_again::{Delay, Retry};

pub const RETRY_STRATEGY: Retry = Retry {
    max_tries: 8,
    delay: Some(Delay::ExponentialBackoff {
        initial_delay: Duration::from_millis(100),
        max_delay: None,
    }),
};
