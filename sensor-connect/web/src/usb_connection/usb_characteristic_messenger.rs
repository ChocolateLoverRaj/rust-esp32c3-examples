use common::{CommandData, Message, MessageToEsp, ResponseData};

pub trait UsbCharacteristicMessenger<T> {
    fn create_get_request() -> CommandData;
    fn find_get_response(response_data: ResponseData) -> Option<T>;

    fn create_subscribe_request() -> Option<MessageToEsp>;

    fn create_unsubscribe_request() -> Option<MessageToEsp>;

    fn match_event(value: Message) -> bool;

    fn create_set_request(value: T) -> CommandData;
}
