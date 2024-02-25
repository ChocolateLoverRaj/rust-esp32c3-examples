use common::{CommandData, ResponseData};

pub trait UsbCharacteristicMessenger<T> {
    fn create_get_request() -> CommandData;
    fn find_get_response(response_data: ResponseData) -> Option<T>;

    // fn create_subscribe_request() -> CommandData;
    // fn create_unsubscribe_request() -> CommandData;

    fn create_set_request(value: T) -> CommandData;
}
