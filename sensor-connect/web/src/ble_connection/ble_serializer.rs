use web_sys::js_sys::DataView;

pub trait BleSerializer<T> {
    fn serialize(data: T) -> Vec<u8>;
    fn deserialize(data: DataView) -> T;
}
