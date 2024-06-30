use esp_idf_svc::hal::gpio::{AnyIOPin, Pins};

pub trait GpioPinsVecExt {
    fn into_indexable(self) -> Vec<AnyIOPin>;
}

impl GpioPinsVecExt for Pins {
    fn into_indexable(self) -> Vec<AnyIOPin> {
        vec![
            self.gpio0.into(),
            self.gpio1.into(),
            self.gpio2.into(),
            self.gpio3.into(),
            self.gpio4.into(),
            self.gpio5.into(),
            self.gpio6.into(),
            self.gpio7.into(),
            self.gpio8.into(),
            self.gpio9.into(),
            self.gpio10.into(),
            self.gpio11.into(),
            self.gpio12.into(),
            self.gpio13.into(),
            self.gpio14.into(),
            self.gpio15.into(),
            self.gpio16.into(),
            self.gpio17.into(),
            self.gpio18.into(),
            self.gpio19.into(),
            self.gpio20.into(),
            self.gpio21.into(),
        ]
    }
}
