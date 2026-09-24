use crate::Player;
use crate::avm1::Object as Avm1Object;
use crate::avm1::globals::netconnection::NetConnection as Avm1NetConnectionObject;
use crate::avm2::object::{
    NetConnectionObject as Avm2NetConnectionObject, ResponderObject as Avm2ResponderObject,
};
use crate::avm2::{Activation as Avm2Activation, Avm2, EventObject as Avm2EventObject};
use crate::backend::navigator::{
    ErrorResponse, FetchReason, NavigatorBackend, OwnedFuture, Request,
};
use crate::context::UpdateContext;
use crate::loader::Error;
use flash_lso::amf3::read::AMF3Decoder;
use flash_lso::extra::flex::read::register_decoders;
use flash_lso::packet::{Header, Message, Packet};
use flash_lso::types::{AMFVersion, Element, Value as AmfValue};
use gc_arena::{Collect, DynamicRoot, Gc, Rootable};
use slotmap::{SlotMap, new_key_type};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

new_key_type! {
    pub struct NetConnectionHandle;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ResponderCallback {
    Result,
    Status,
}

#[derive(Clone)]
pub enum ResponderHandle {
    Avm2(DynamicRoot<Rootable![Avm2ResponderObject<'_>]>),
    Avm1(DynamicRoot<Rootable![Avm1Object<'_>]>),
}

impl Debug for ResponderHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponderHandle::Avm2(_) => write!(f, "ResponderHandle::Avm2"),
            ResponderHandle::Avm1(_) => write!(f, "ResponderHandle::Avm1"),
        }
    }
}

impl ResponderHandle {
    pub fn call(
        &self,
        context: &mut UpdateContext<'_>,
        callback: ResponderCallback,
        message: Rc<AmfValue>,
    ) {
        match self {
            ResponderHandle::Avm2(handle) => {
                let object = context.dynamic_root.fetch(handle);
                let mut activation = Avm2Activation::from_nothing(context);

                if let Err(err) = object.send_callback(&mut activation, callback, &message) {
                    Avm2::uncaught_error(
                        &mut activation,
                        None, // TODO we need to set this, but how?
                        err,
                        "Error running AVM2 NetConnection callback",
                    );
                }
            }
            ResponderHandle::Avm1(handle) => {
                let object = context.dynamic_root.fetch(handle);
                if let Err(e) =
                    Avm1NetConnectionObject::send_callback(context, *object, callback, &message)
                {
                    tracing::error!("Unhandled error sending {callback:?} callback: {e}");
                }
            }
        }
    }
}

#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub enum NetConnectionObject<'gc> {
    Avm2(Avm2NetConnectionObject<'gc>),
    Avm1(Avm1Object<'gc>),
}

impl NetConnectionObject<'_> {
    pub fn set_handle(&self, handle: Option<NetConnectionHandle>) -> Option<NetConnectionHandle> {
        match self {
            NetConnectionObject::Avm2(object) => object.set_handle(handle),
            NetConnectionObject::Avm1(object) => {
                if let Some(net_connection) = Avm1NetConnectionObject::cast((*object).into()) {
                    net_connection.set_handle(handle)
                } else {
                    None
                }
            }
        }
    }
}

impl<'gc> From<Avm2NetConnectionObject<'gc>> for NetConnectionObject<'gc> {
    fn from(value: Avm2NetConnectionObject<'gc>) -> Self {
        NetConnectionObject::Avm2(value)
    }
}

impl<'gc> From<Avm1Object<'gc>> for NetConnectionObject<'gc> {
    fn from(value: Avm1Object<'gc>) -> Self {
        NetConnectionObject::Avm1(value)
    }
}

/// Manages the collection of NetConnections.
#[derive(Collect)]
#[collect(no_drop)]
pub struct NetConnections<'gc> {
    connections: SlotMap<NetConnectionHandle, NetConnection<'gc>>,
}

impl Default for NetConnections<'_> {
    fn default() -> Self {
        Self {
            connections: SlotMap::with_key(),
        }
    }
}

impl<'gc> NetConnections<'gc> {
    pub fn connect_to_local<O: Into<NetConnectionObject<'gc>>>(
        context: &mut UpdateContext<'gc>,
        target: O,
    ) {
        let target = target.into();
        let connection = NetConnection {
            object: target,
            protocol: NetConnectionProtocol::Local,
        };
        let handle = context.net_connections.connections.insert(connection);

        if let Some(existing_handle) = target.set_handle(Some(handle)) {
            NetConnections::close(context, existing_handle, false);
        }

        match target {
            NetConnectionObject::Avm2(object) => {
                let mut activation = Avm2Activation::from_nothing(context);
                let event = Avm2EventObject::net_status_event(
                    &mut activation,
                    [
                        ("code", "NetConnection.Connect.Success"),
                        ("level", "status"),
                    ],
                );
                Avm2::dispatch_event(activation.context, event, object.into());
            }
            NetConnectionObject::Avm1(object) => {
                if let Err(e) = Avm1NetConnectionObject::on_status_event(
                    context,
                    object,
                    "NetConnection.Connect.Success",
                ) {
                    tracing::error!("Unhandled error sending connection callback: {e}");
                }
            }
        }
    }

    pub fn connect_to_flash_remoting<O: Into<NetConnectionObject<'gc>>>(
        context: &mut UpdateContext<'gc>,
        target: O,
        url: String,
    ) {
        let target = target.into();
        let connection = NetConnection {
            object: target,
            protocol: NetConnectionProtocol::FlashRemoting(FlashRemoting {
                url,
                headers: vec![],
                outgoing_queue: vec![],
                next_response_uri: 1,
            }),
        };
        let handle = context.net_connections.connections.insert(connection);

        if let Some(existing_handle) = target.set_handle(Some(handle)) {
            NetConnections::close(context, existing_handle, false);
        }

        // No open event here
    }

    pub fn close(context: &mut UpdateContext<'gc>, handle: NetConnectionHandle, is_explicit: bool) {
        let Some(connection) = context.net_connections.connections.remove(handle) else {
            return;
        };

        match connection.object {
            NetConnectionObject::Avm2(object) => {
                let mut activation = Avm2Activation::from_nothing(context);
                let event = Avm2EventObject::net_status_event(
                    &mut activation,
                    [
                        ("code", "NetConnection.Connect.Closed"),
                        ("level", "status"),
                    ],
                );
                Avm2::dispatch_event(activation.context, event, object.into());

                if is_explicit
                    && matches!(connection.protocol, NetConnectionProtocol::FlashRemoting(_))
                {
                    // [NA] I have no idea why, but a NetConnection receives a second and nonsensical event on close
                    let event = Avm2EventObject::net_status_event(
                        &mut activation,
                        [
                            ("code", ""),
                            ("description", ""),
                            ("details", ""),
                            ("level", "status"),
                        ],
                    );
                    Avm2::dispatch_event(activation.context, event, object.into());
                }
            }
            NetConnectionObject::Avm1(object) => {
                if let Err(e) = Avm1NetConnectionObject::on_status_event(
                    context,
                    object,
                    "NetConnection.Connect.Closed",
                ) {
                    tracing::error!("Unhandled error sending connection callback: {e}");
                }
                if is_explicit
                    && matches!(connection.protocol, NetConnectionProtocol::FlashRemoting(_))
                    && let Err(e) = Avm1NetConnectionObject::on_empty_status_event(context, object)
                {
                    tracing::error!("Unhandled error sending connection callback: {e}");
                }
            }
        }
    }

    pub fn update_connections(context: &mut UpdateContext<'gc>) {
        let player = context.player_handle();
        for (handle, connection) in context.net_connections.connections.iter_mut() {
            connection.update(handle, context.navigator, &player);
        }
    }

    pub fn send_without_response(
        context: &mut UpdateContext<'gc>,
        handle: NetConnectionHandle,
        command: String,
        message: AmfValue,
    ) {
        if let Some(connection) = context.net_connections.connections.get_mut(handle) {
            connection.send(command, None, message);
        }
    }

    pub fn send_avm2(
        context: &mut UpdateContext<'gc>,
        handle: NetConnectionHandle,
        command: String,
        message: AmfValue,
        responder: Avm2ResponderObject<'gc>,
    ) {
        let mc = context.gc();
        if let Some(connection) = context.net_connections.connections.get_mut(handle) {
            // TODO(moulins): it'd be nice to avoid the double indirection here...
            let responder_handle =
                ResponderHandle::Avm2(context.dynamic_root.stash(mc, Gc::new(mc, responder)));
            connection.send(command, Some(responder_handle), message);
        }
    }

    pub fn send_avm1(
        context: &mut UpdateContext<'gc>,
        handle: NetConnectionHandle,
        command: String,
        message: AmfValue,
        responder: Avm1Object<'gc>,
    ) {
        let mc = context.gc();
        if let Some(connection) = context.net_connections.connections.get_mut(handle) {
            // TODO(moulins): it'd be nice to avoid the double indirection here...
            let responder_handle =
                ResponderHandle::Avm1(context.dynamic_root.stash(mc, Gc::new(mc, responder)));
            connection.send(command, Some(responder_handle), message);
        }
    }

    pub fn set_header(&mut self, handle: NetConnectionHandle, header: Header) {
        if let Some(connection) = self.connections.get_mut(handle) {
            connection.set_header(header);
        }
    }

    pub fn is_connected(&self, handle: NetConnectionHandle) -> bool {
        self.connections
            .get(handle)
            .map(|c| c.is_connected())
            .unwrap_or_default()
    }

    pub fn get_connected_proxy_type(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections
            .get(handle)
            .and_then(|c| c.connected_proxy_type())
    }

    pub fn get_far_id(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections.get(handle).and_then(|c| c.far_id())
    }

    pub fn get_far_nonce(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections.get(handle).and_then(|c| c.far_nonce())
    }

    pub fn get_near_id(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections.get(handle).and_then(|c| c.near_id())
    }

    pub fn get_near_nonce(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections.get(handle).and_then(|c| c.near_nonce())
    }

    pub fn get_protocol(&self, handle: NetConnectionHandle) -> Option<&'static str> {
        self.connections.get(handle).and_then(|c| c.protocol())
    }

    pub fn get_uri(&self, handle: NetConnectionHandle) -> Option<String> {
        self.connections.get(handle).and_then(|c| c.uri())
    }

    pub fn is_using_tls(&self, handle: NetConnectionHandle) -> Option<bool> {
        self.connections.get(handle).and_then(|c| c.using_tls())
    }
}

#[derive(Collect)]
#[collect(no_drop)]
pub struct NetConnection<'gc> {
    object: NetConnectionObject<'gc>,

    #[collect(require_static)]
    protocol: NetConnectionProtocol,
}

impl NetConnection<'_> {
    pub fn is_connected(&self) -> bool {
        match self.protocol {
            NetConnectionProtocol::Local => true,
            NetConnectionProtocol::FlashRemoting(_) => false,
        }
    }

    pub fn connected_proxy_type(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => Some("none"),
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn far_id(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => Some(""),
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn far_nonce(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => {
                Some("0000000000000000000000000000000000000000000000000000000000000000")
            }
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn near_id(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => Some(""),
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn near_nonce(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => {
                Some("0000000000000000000000000000000000000000000000000000000000000000")
            }
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn protocol(&self) -> Option<&'static str> {
        match self.protocol {
            NetConnectionProtocol::Local => Some("rtmp"),
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn uri(&self) -> Option<String> {
        match &self.protocol {
            NetConnectionProtocol::Local => Some("null".to_string()), // Yes, it's a string "null", not a real null.
            NetConnectionProtocol::FlashRemoting(remoting) => Some(remoting.url.to_string()),
        }
    }

    pub fn using_tls(&self) -> Option<bool> {
        match &self.protocol {
            NetConnectionProtocol::Local => Some(false),
            NetConnectionProtocol::FlashRemoting(_) => None,
        }
    }

    pub fn send(
        &mut self,
        command: String,
        responder_handle: Option<ResponderHandle>,
        message: AmfValue,
    ) {
        match &mut self.protocol {
            NetConnectionProtocol::Local => {}
            NetConnectionProtocol::FlashRemoting(remoting) => {
                remoting.send(command, responder_handle, message)
            }
        }
    }

    pub fn update(
        &mut self,
        self_handle: NetConnectionHandle,
        navigator: &mut dyn NavigatorBackend,
        player: &Arc<Mutex<Player>>,
    ) {
        match &mut self.protocol {
            NetConnectionProtocol::Local => {}
            NetConnectionProtocol::FlashRemoting(remoting) => {
                if remoting.has_pending_packet() {
                    navigator.spawn_future(remoting.flush_queue(self_handle, player.clone()));
                }
            }
        }
    }

    pub fn set_header(&mut self, header: Header) {
        match &mut self.protocol {
            NetConnectionProtocol::Local => {}
            NetConnectionProtocol::FlashRemoting(remoting) => {
                remoting.set_header(header);
            }
        }
    }
}

#[derive(Debug)]
pub enum NetConnectionProtocol {
    /// A "local" connection, caused by connecting to null
    Local,

    /// Flash Remoting protocol, caused by connecting to a `http://` address.
    FlashRemoting(FlashRemoting),
}

#[derive(Debug)]
pub struct FlashRemoting {
    url: String,
    headers: Vec<Header>,
    outgoing_queue: Vec<(Message, Option<ResponderHandle>)>,
    next_response_uri: u32,
}

impl FlashRemoting {
    pub fn send(
        &mut self,
        command: String,
        responder_handle: Option<ResponderHandle>,
        message: AmfValue,
    ) {
        let response_uri = format!("/{}", self.next_response_uri);
        self.next_response_uri = self.next_response_uri.wrapping_add(1);
        self.outgoing_queue.push((
            Message {
                target_uri: command,
                response_uri,
                contents: Rc::new(message),
            },
            responder_handle,
        ));
    }

    pub fn has_pending_packet(&self) -> bool {
        !self.outgoing_queue.is_empty()
    }

    pub fn set_header(&mut self, header: Header) {
        // Only one header of the same name (case insensitive) should exist
        self.headers
            .retain(|h| !h.name.eq_ignore_ascii_case(&header.name));

        self.headers.push(header);
    }

    pub fn flush_queue(
        &mut self,
        self_handle: NetConnectionHandle,
        player: Arc<Mutex<Player>>,
    ) -> OwnedFuture<(), Error> {
        let queue = std::mem::take(&mut self.outgoing_queue);
        let (messages, responder_handles): (Vec<_>, Vec<_>) = queue.into_iter().unzip();
        let response_uris = messages
            .iter()
            .map(|message| message.response_uri.clone())
            .collect::<Vec<_>>();
        let version = if messages
            .iter()
            .any(|message| match message.contents.as_ref() {
                AmfValue::AMF3(_) => true,
                AmfValue::StrictArray(_, values) => values
                    .iter()
                    .any(|value| matches!(value.as_ref(), AmfValue::AMF3(_))),
                _ => false,
            }) {
            AMFVersion::AMF3
        } else {
            AMFVersion::AMF0
        };
        let packet = Packet {
            version,
            headers: self.headers.clone(),
            messages,
        };
        let url = self.url.clone();

        Box::pin(async move {
            let bytes = flash_lso::packet::write::write_to_bytes(&packet, true)
                .expect("Must be able to serialize a packet");
            let request = Request::post(url, Some((bytes, "application/x-amf".to_string())));
            let fetch = player.lock().unwrap().fetch(request, FetchReason::Other);
            let response: Result<_, ErrorResponse> = async {
                let response = fetch.await?;
                let url = response.url().to_string();
                let body = response
                    .body()
                    .await
                    .map_err(|error| ErrorResponse { url, error })?;

                Ok(body)
            }
            .await;
            let response = match response {
                Ok(response) => response,
                Err(response) => {
                    player.lock().unwrap().update(|uc| {
                        tracing::error!(
                            "Couldn't submit AMF Packet to {}: {:?}",
                            response.url,
                            response.error
                        );
                        if let Some(connection) = uc.net_connections.connections.get(self_handle) {
                            match connection.object {
                                NetConnectionObject::Avm2(object) => {
                                    let mut activation = Avm2Activation::from_nothing(uc);
                                    let event = Avm2EventObject::net_status_event(
                                        &mut activation,
                                        [
                                            ("code", "NetConnection.Call.Failed"),
                                            ("level", "error"),
                                            ("details", &response.url),
                                            ("description", "HTTP: Failed"),
                                        ],
                                    );
                                    Avm2::dispatch_event(activation.context, event, object.into());
                                }
                                NetConnectionObject::Avm1(object) => {
                                    if let Err(e) =
                                        Avm1NetConnectionObject::on_empty_status_event(uc, object)
                                    {
                                        tracing::error!(
                                            "Unhandled error sending connection callback: {e}"
                                        );
                                    }
                                }
                            }
                        }
                    });
                    return Ok(());
                }
            };

            // Flash completely ignores invalid responses, it seems
            if let Some(response_packet) = parse_remoting_response(&response) {
                player.lock().unwrap().update(|uc| {
                    for message in response_packet.messages {
                        let response = message
                            .target_uri
                            .strip_suffix("/onStatus")
                            .map(|uri| (uri, ResponderCallback::Status))
                            .or_else(|| {
                                message
                                    .target_uri
                                    .strip_suffix("/onResult")
                                    .map(|uri| (uri, ResponderCallback::Result))
                            });
                        if let Some((uri, callback)) = response
                            && let Some(index) = response_uris.iter().position(|item| item == uri)
                            && let Some(responder_handle) =
                                responder_handles.get(index).cloned().flatten()
                        {
                            responder_handle.call(uc, callback, message.contents);
                        }
                    }
                });
            }

            Ok(())
        })
    }
}

fn parse_remoting_response(bytes: &[u8]) -> Option<Packet> {
    if let Ok(packet) = flash_lso::packet::read::parse(bytes) {
        return Some(packet);
    }

    // Flex may return externalizable AMF3 messages, which the generic packet
    // reader cannot decode without its Flex decoders.
    fn take<'a>(data: &mut &'a [u8], len: usize) -> Option<&'a [u8]> {
        let (head, tail) = data.split_at_checked(len)?;
        *data = tail;
        Some(head)
    }
    fn word(data: &mut &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes(take(data, 2)?.try_into().ok()?))
    }
    fn dword(data: &mut &[u8]) -> Option<u32> {
        Some(u32::from_be_bytes(take(data, 4)?.try_into().ok()?))
    }
    fn string(data: &mut &[u8]) -> Option<String> {
        let len = word(data)? as usize;
        Some(std::str::from_utf8(take(data, len)?).ok()?.to_owned())
    }

    let mut data = bytes;
    let version = word(&mut data)?;
    if version != 3 || word(&mut data)? != 0 {
        return None;
    }
    let count = word(&mut data)?;
    let mut messages = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let target_uri = string(&mut data)?;
        let response_uri = string(&mut data)?;
        let length = dword(&mut data)?;
        if take(&mut data, 1)? != [0x11] {
            return None;
        }
        let mut decoder = AMF3Decoder::default();
        register_decoders(&mut decoder);
        decoder
            .external_decoders
            .insert("DSK".to_owned(), Rc::new(Box::new(parse_flex_acknowledge)));
        let before = data.len();
        let (rest, contents) = decoder.parse_single_element(data).ok()?;
        let consumed = before - rest.len() + 1;
        if length != u32::MAX && length as usize != consumed {
            return None;
        }
        data = rest;
        messages.push(Message {
            target_uri,
            response_uri,
            contents,
        });
    }
    if !data.is_empty() {
        return None;
    }
    Some(Packet {
        version: AMFVersion::AMF3,
        headers: vec![],
        messages,
    })
}

fn parse_flex_acknowledge<'a>(
    mut data: &'a [u8],
    decoder: &mut AMF3Decoder,
) -> Result<(&'a [u8], Vec<Element>), nom::Err<flash_lso::errors::Error<'a>>> {
    let mut elements = Vec::new();
    // Flex externalizes the AbstractMessage, AsyncMessage, and
    // AcknowledgeMessage fields as three consecutive flag sections.
    for section in 0..3 {
        let mut flags = Vec::new();
        loop {
            let (&flag, rest) = data
                .split_first()
                .ok_or(nom::Err::Error(flash_lso::errors::Error::OutOfBounds))?;
            data = rest;
            flags.push(flag);
            if flag & 0x80 == 0 {
                break;
            }
        }
        for (group, flag) in flags.into_iter().enumerate() {
            for bit in 0..7 {
                if flag & (1 << bit) == 0 {
                    continue;
                }
                let (rest, value) = decoder.parse_single_element(data)?;
                data = rest;
                let name = match (section, group, bit) {
                    (0, 0, 0) => "body".to_owned(),
                    (0, 0, 1) => "clientId".to_owned(),
                    (0, 0, 2) => "destination".to_owned(),
                    (0, 0, 3) => "headers".to_owned(),
                    (0, 0, 4) => "messageId".to_owned(),
                    (0, 0, 5) => "timestamp".to_owned(),
                    (0, 0, 6) => "timeToLive".to_owned(),
                    (0, 1, 0) => "clientIdBytes".to_owned(),
                    (0, 1, 1) => "messageIdBytes".to_owned(),
                    (1, 0, 0) => "correlationId".to_owned(),
                    (1, 0, 1) => "correlationIdBytes".to_owned(),
                    _ => format!("flex_{section}_{group}_{bit}"),
                };
                elements.push(Element { name, value });
            }
        }
    }
    Ok((data, elements))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_remoting_keeps_response_uri_sequence_across_packets() {
        let mut connection = FlashRemoting {
            url: String::new(),
            headers: vec![],
            outgoing_queue: vec![],
            next_response_uri: 1,
        };
        connection.send("first".to_owned(), None, AmfValue::Null);
        assert_eq!(connection.outgoing_queue[0].0.response_uri, "/1");
        connection.outgoing_queue.clear();
        connection.send("second".to_owned(), None, AmfValue::Null);
        assert_eq!(connection.outgoing_queue[0].0.response_uri, "/2");
    }

    #[test]
    fn flash_remoting_wraps_amf3_argument_in_amf0_array() {
        let packet = Packet {
            version: AMFVersion::AMF3,
            headers: vec![],
            messages: vec![Message {
                target_uri: "null".to_owned(),
                response_uri: "/1".to_owned(),
                contents: Rc::new(AmfValue::StrictArray(
                    flash_lso::types::ObjectId::INVALID,
                    vec![Rc::new(AmfValue::AMF3(Rc::new(AmfValue::String(
                        "test".to_owned(),
                    ))))],
                )),
            }],
        };
        let bytes = flash_lso::packet::write::write_to_bytes(&packet, true).unwrap();
        assert_eq!(&bytes[20..27], &[0x0a, 0, 0, 0, 1, 0x11, 0x06]);
        let parsed = flash_lso::packet::read::parse(&bytes).unwrap();
        assert_eq!(parsed.version, AMFVersion::AMF3);
        let AmfValue::StrictArray(_, values) = parsed.messages[0].contents.as_ref() else {
            panic!("AMF0 array envelope was lost");
        };
        assert!(matches!(values[0].as_ref(), AmfValue::AMF3(_)));
    }

    #[test]
    fn parses_externalizable_flex_acknowledgement() {
        let bytes = include_bytes!("../tests/fixtures/flex_acknowledge_amf3.bin");
        let packet = parse_remoting_response(bytes).expect("Flex packet should parse");
        assert_eq!(packet.version, AMFVersion::AMF3);
        assert_eq!(packet.messages.len(), 1);
        assert_eq!(packet.messages[0].target_uri, "/1/onResult");
        let AmfValue::Custom(fields, _, Some(class)) = packet.messages[0].contents.as_ref() else {
            panic!("Expected an externalizable Flex acknowledgement");
        };
        assert_eq!(class.name, "DSK");
        assert!(fields.iter().any(|field| field.name == "clientIdBytes"));
    }
}
