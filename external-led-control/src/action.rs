use std::str::FromStr;

#[derive(Debug)]
pub enum Action {
    On,
    Off,
    Toggle,
}

impl FromStr for Action {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            "toggle" => Ok(Self::Toggle),
            _ => Err(()),
        }
    }
}
