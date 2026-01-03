use core::ops::Not;

use defmt::*;

#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub clk: bool,
    pub dt: bool,
}

#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
enum RotaryPin {
    Clock,
    Dt,
}

#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

impl Not for Direction {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Clockwise => Self::CounterClockwise,
            Self::CounterClockwise => Self::Clockwise,
        }
    }
}

impl RotaryPin {
    /// Returns +1 if clockwise and -1 if counter-clockwise
    pub fn leading_direction(&self) -> Direction {
        match self {
            Self::Clock => Direction::Clockwise,
            Self::Dt => Direction::CounterClockwise,
        }
    }
}

pub struct RotaryEncoder {
    state: State,
    leading_pin: Option<RotaryPin>,
}

impl RotaryEncoder {
    pub fn new(state: State) -> Self {
        Self {
            state,
            leading_pin: None,
        }
    }

    pub fn process_data(&mut self, new_state: State) -> Option<Direction> {
        let direction = if new_state != self.state {
            trace!("new state: {}", new_state);
            let clk_changed = new_state.clk != self.state.clk;
            let dt_changed = new_state.dt != self.state.dt;
            let changed_pin = match (clk_changed, dt_changed) {
                (true, false) => Some(RotaryPin::Clock),
                (false, true) => Some(RotaryPin::Dt),
                _ => None,
            };
            if let Some(changed_pin) = changed_pin {
                Some(if let Some(leading_pin) = self.leading_pin {
                    let change = if changed_pin != leading_pin {
                        trace!("{} caught up", changed_pin);
                        leading_pin.leading_direction()
                    } else {
                        trace!("{} moved back. direction changed", changed_pin);
                        !leading_pin.leading_direction()
                    };
                    self.leading_pin = None;
                    change
                } else {
                    self.leading_pin = Some(changed_pin);
                    trace!("{} moved", changed_pin);
                    changed_pin.leading_direction()
                })
            } else {
                warn!("BOTH CHANGED!");
                None
            }
        } else {
            None
        };
        self.state = new_state;
        direction
    }
}
