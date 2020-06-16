// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.
//
// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! `NetworkBehaviour` implementation which handles incoming block requests.
//!
//! Every request is coming in on a separate connection substream which gets
//! closed after we have sent the response back. Incoming requests are encoded
//! as protocol buffers (cf. `api.v1.proto`).

#![allow(unused)]

use bytes::Bytes;
use codec::{Encode, Decode};
use crate::{
	chain::Client,
	config::ProtocolId,
	protocol::{message::{self, BlockAttributes}},
	schema,
};
use futures::{future::BoxFuture, prelude::*, stream::FuturesUnordered};
use futures_timer::Delay;
use libp2p::{
	core::{
		ConnectedPoint,
		Multiaddr,
		PeerId,
		connection::ConnectionId,
		upgrade::{InboundUpgrade, OutboundUpgrade, ReadOneError, UpgradeInfo, Negotiated},
		upgrade::{DeniedUpgrade, read_one, write_one}
	},
	swarm::{
		NegotiatedSubstream,
		NetworkBehaviour,
		NetworkBehaviourAction,
		NotifyHandler,
		OneShotHandler,
		OneShotHandlerConfig,
		PollParameters,
		SubstreamProtocol
	}
};
use prost::Message;
use sp_runtime::{generic::BlockId, traits::{Block, Header, One, Zero}};
use std::{
	cmp::min,
	collections::{HashMap, VecDeque},
	io,
	iter,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	time::Duration,
	task::{Context, Poll}
};
use void::{Void, unreachable};
use wasm_timer::Instant;

// Type alias for convenience.
pub type Error = Box<dyn std::error::Error + 'static>;

/// Event generated by the block requests behaviour.
#[derive(Debug)]
pub enum Event<B: Block> {
	/// A request came and we have successfully answered it.
	AnsweredRequest {
		/// Peer which has emitted the request.
		peer: PeerId,
		/// Time elapsed between when we received the request and when we sent back the response.
		total_handling_time: Duration,
	},

	/// A response to a block request has arrived.
	Response {
		peer: PeerId,
		/// The original request passed to `send_request`.
		original_request: message::BlockRequest<B>,
		response: message::BlockResponse<B>,
		/// Time elapsed between the start of the request and the response.
		request_duration: Duration,
	},

	/// A request has been cancelled because the peer has disconnected.
	/// Disconnects can also happen as a result of violating the network protocol.
	///
	/// > **Note**: This event is NOT emitted if a request is overridden by calling `send_request`.
	/// > For that, you must check the value returned by `send_request`.
	RequestCancelled {
		peer: PeerId,
		/// The original request passed to `send_request`.
		original_request: message::BlockRequest<B>,
		/// Time elapsed between the start of the request and the cancellation.
		request_duration: Duration,
	},

	/// A request has timed out.
	RequestTimeout {
		peer: PeerId,
		/// The original request passed to `send_request`.
		original_request: message::BlockRequest<B>,
		/// Time elapsed between the start of the request and the timeout.
		request_duration: Duration,
	}
}

/// Configuration options for `BlockRequests`.
#[derive(Debug, Clone)]
pub struct Config {
	max_block_data_response: u32,
	max_request_len: usize,
	max_response_len: usize,
	inactivity_timeout: Duration,
	request_timeout: Duration,
	protocol: Bytes,
}

impl Config {
	/// Create a fresh configuration with the following options:
	///
	/// - max. block data in response = 128
	/// - max. request size = 1 MiB
	/// - max. response size = 16 MiB
	/// - inactivity timeout = 15s
	/// - request timeout = 40s
	pub fn new(id: &ProtocolId) -> Self {
		let mut c = Config {
			max_block_data_response: 128,
			max_request_len: 1024 * 1024,
			max_response_len: 16 * 1024 * 1024,
			inactivity_timeout: Duration::from_secs(15),
			request_timeout: Duration::from_secs(40),
			protocol: Bytes::new(),
		};
		c.set_protocol(id);
		c
	}

	/// Limit the max. number of block data in a response.
	pub fn set_max_block_data_response(&mut self, v: u32) -> &mut Self {
		self.max_block_data_response = v;
		self
	}

	/// Limit the max. length of incoming block request bytes.
	pub fn set_max_request_len(&mut self, v: usize) -> &mut Self {
		self.max_request_len = v;
		self
	}

	/// Limit the max. size of responses to our block requests.
	pub fn set_max_response_len(&mut self, v: usize) -> &mut Self {
		self.max_response_len = v;
		self
	}

	/// Limit the max. duration the substream may remain inactive before closing it.
	pub fn set_inactivity_timeout(&mut self, v: Duration) -> &mut Self {
		self.inactivity_timeout = v;
		self
	}

	/// Set protocol to use for upgrade negotiation.
	pub fn set_protocol(&mut self, id: &ProtocolId) -> &mut Self {
		let mut v = Vec::new();
		v.extend_from_slice(b"/");
		v.extend_from_slice(id.as_bytes());
		v.extend_from_slice(b"/sync/2");
		self.protocol = v.into();
		self
	}
}

/// The block request handling behaviour.
pub struct BlockRequests<B: Block> {
	/// This behaviour's configuration.
	config: Config,
	/// Blockchain client.
	chain: Arc<dyn Client<B>>,
	/// List of all active connections and the requests we've sent.
	peers: HashMap<PeerId, Vec<Connection<B>>>,
	/// Futures sending back the block request response. Returns the `PeerId` we sent back to, and
	/// the total time the handling of this request took.
	outgoing: FuturesUnordered<BoxFuture<'static, (PeerId, Duration)>>,
	/// Events to return as soon as possible from `poll`.
	pending_events: VecDeque<NetworkBehaviourAction<OutboundProtocol<B>, Event<B>>>,
}

/// Local tracking of a libp2p connection.
#[derive(Debug)]
struct Connection<B: Block> {
	id: ConnectionId,
	ongoing_request: Option<OngoingRequest<B>>,
}

#[derive(Debug)]
struct OngoingRequest<B: Block> {
	/// `Instant` when the request has been emitted. Used for diagnostic purposes.
	emitted: Instant,
	request: message::BlockRequest<B>,
	timeout: Delay,
}

/// Outcome of calling `send_request`.
#[derive(Debug)]
#[must_use]
pub enum SendRequestOutcome<B: Block> {
	/// Request has been emitted.
	Ok,
	/// The request has been emitted and has replaced an existing request.
	Replaced {
		/// The previously-emitted request.
		previous: message::BlockRequest<B>,
		/// Time that had elapsed since `previous` has been emitted.
		request_duration: Duration,
	},
	/// Didn't start a request because we have no connection to this node.
	/// If `send_request` returns that, it is as if the function had never been called.
	NotConnected,
	/// Error while serializing the request.
	EncodeError(prost::EncodeError),
}

impl<B> BlockRequests<B>
where
	B: Block,
{
	pub fn new(cfg: Config, chain: Arc<dyn Client<B>>) -> Self {
		BlockRequests {
			config: cfg,
			chain,
			peers: HashMap::new(),
			outgoing: FuturesUnordered::new(),
			pending_events: VecDeque::new(),
		}
	}

	/// Returns the libp2p protocol name used on the wire (e.g. `/foo/sync/2`).
	pub fn protocol_name(&self) -> &[u8] {
		&self.config.protocol
	}

	/// Issue a new block request.
	///
	/// Cancels any existing request targeting the same `PeerId`.
	///
	/// If the response doesn't arrive in time, or if the remote answers improperly, the target
	/// will be disconnected.
	pub fn send_request(&mut self, target: &PeerId, req: message::BlockRequest<B>) -> SendRequestOutcome<B> {
		// Determine which connection to send the request to.
		let connection = if let Some(peer) = self.peers.get_mut(target) {
			// We don't want to have multiple requests for any given node, so in priority try to
			// find a connection with an existing request, to override it.
			if let Some(entry) = peer.iter_mut().find(|c| c.ongoing_request.is_some()) {
				entry
			} else if let Some(entry) = peer.get_mut(0) {
				entry
			} else {
				log::error!(
					target: "sync",
					"State inconsistency: empty list of peer connections"
				);
				return SendRequestOutcome::NotConnected;
			}
		} else {
			return SendRequestOutcome::NotConnected;
		};

		let protobuf_rq = build_protobuf_block_request(
			req.fields,
			req.from.clone(),
			req.to.clone(),
			req.direction,
			req.max,
		);

		let mut buf = Vec::with_capacity(protobuf_rq.encoded_len());
		if let Err(err) = protobuf_rq.encode(&mut buf) {
			log::warn!(
				target: "sync",
				"Failed to encode block request {:?}: {:?}",
				protobuf_rq,
				err
			);
			return SendRequestOutcome::EncodeError(err);
		}

		let previous_request = connection.ongoing_request.take();
		connection.ongoing_request = Some(OngoingRequest {
			emitted: Instant::now(),
			request: req.clone(),
			timeout: Delay::new(self.config.request_timeout),
		});

		log::trace!(target: "sync", "Enqueueing block request to {:?}: {:?}", target, protobuf_rq);
		self.pending_events.push_back(NetworkBehaviourAction::NotifyHandler {
			peer_id: target.clone(),
			handler: NotifyHandler::One(connection.id),
			event: OutboundProtocol {
				request: buf,
				original_request: req,
				max_response_size: self.config.max_response_len,
				protocol: self.config.protocol.clone(),
			},
		});

		if let Some(previous_request) = previous_request {
			log::debug!(
				target: "sync",
				"Replacing existing block request on connection {:?}",
				connection.id
			);
			SendRequestOutcome::Replaced {
				previous: previous_request.request,
				request_duration: previous_request.emitted.elapsed(),
			}
		} else {
			SendRequestOutcome::Ok
		}
	}

	/// Callback, invoked when a new block request has been received from remote.
	fn on_block_request
		( &mut self
		, peer: &PeerId
		, request: &schema::v1::BlockRequest
		) -> Result<schema::v1::BlockResponse, Error>
	{
		log::trace!(
			target: "sync",
			"Block request from peer {}: from block {:?} to block {:?}, max blocks {:?}",
			peer,
			request.from_block,
			request.to_block,
			request.max_blocks);

		let from_block_id =
			match request.from_block {
				Some(schema::v1::block_request::FromBlock::Hash(ref h)) => {
					let h = Decode::decode(&mut h.as_ref())?;
					BlockId::<B>::Hash(h)
				}
				Some(schema::v1::block_request::FromBlock::Number(ref n)) => {
					let n = Decode::decode(&mut n.as_ref())?;
					BlockId::<B>::Number(n)
				}
				None => {
					let msg = "missing `BlockRequest::from_block` field";
					return Err(io::Error::new(io::ErrorKind::Other, msg).into())
				}
			};

		let max_blocks =
			if request.max_blocks == 0 {
				self.config.max_block_data_response
			} else {
				min(request.max_blocks, self.config.max_block_data_response)
			};

		let direction =
			if request.direction == schema::v1::Direction::Ascending as i32 {
				schema::v1::Direction::Ascending
			} else if request.direction == schema::v1::Direction::Descending as i32 {
				schema::v1::Direction::Descending
			} else {
				let msg = format!("invalid `BlockRequest::direction` value: {}", request.direction);
				return Err(io::Error::new(io::ErrorKind::Other, msg).into())
			};

		let attributes = BlockAttributes::from_be_u32(request.fields)?;
		let get_header = attributes.contains(BlockAttributes::HEADER);
		let get_body = attributes.contains(BlockAttributes::BODY);
		let get_justification = attributes.contains(BlockAttributes::JUSTIFICATION);

		let mut blocks = Vec::new();
		let mut block_id = from_block_id;
		while let Some(header) = self.chain.header(block_id).unwrap_or(None) {
			if blocks.len() >= max_blocks as usize {
				break
			}

			let number = *header.number();
			let hash = header.hash();
			let parent_hash = *header.parent_hash();
			let justification = if get_justification {
				self.chain.justification(&BlockId::Hash(hash))?
			} else {
				None
			};
			let is_empty_justification = justification.as_ref().map(|j| j.is_empty()).unwrap_or(false);

			let block_data = schema::v1::BlockData {
				hash: hash.encode(),
				header: if get_header {
					header.encode()
				} else {
					Vec::new()
				},
				body: if get_body {
					self.chain.block_body(&BlockId::Hash(hash))?
						.unwrap_or(Vec::new())
						.iter_mut()
						.map(|extrinsic| extrinsic.encode())
						.collect()
				} else {
					Vec::new()
				},
				receipt: Vec::new(),
				message_queue: Vec::new(),
				justification: justification.unwrap_or(Vec::new()),
				is_empty_justification,
			};

			blocks.push(block_data);

			match direction {
				schema::v1::Direction::Ascending => {
					block_id = BlockId::Number(number + One::one())
				}
				schema::v1::Direction::Descending => {
					if number.is_zero() {
						break
					}
					block_id = BlockId::Hash(parent_hash)
				}
			}
		}

		Ok(schema::v1::BlockResponse { blocks })
	}
}

impl<B> NetworkBehaviour for BlockRequests<B>
where
	B: Block
{
	type ProtocolsHandler = OneShotHandler<InboundProtocol<B>, OutboundProtocol<B>, NodeEvent<B, NegotiatedSubstream>>;
	type OutEvent = Event<B>;

	fn new_handler(&mut self) -> Self::ProtocolsHandler {
		let p = InboundProtocol {
			max_request_len: self.config.max_request_len,
			protocol: self.config.protocol.clone(),
			marker: PhantomData,
		};
		let mut cfg = OneShotHandlerConfig::default();
		cfg.inactive_timeout = self.config.inactivity_timeout;
		cfg.substream_timeout = self.config.request_timeout;
		OneShotHandler::new(SubstreamProtocol::new(p), cfg)
	}

	fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
		Vec::new()
	}

	fn inject_connected(&mut self, _peer: &PeerId) {
	}

	fn inject_disconnected(&mut self, _peer: &PeerId) {
	}

	fn inject_connection_established(&mut self, peer_id: &PeerId, id: &ConnectionId, _: &ConnectedPoint) {
		self.peers.entry(peer_id.clone())
			.or_default()
			.push(Connection {
				id: *id,
				ongoing_request: None,
			});
	}

	fn inject_connection_closed(&mut self, peer_id: &PeerId, id: &ConnectionId, _: &ConnectedPoint) {
		let mut needs_remove = false;
		if let Some(entry) = self.peers.get_mut(peer_id) {
			if let Some(pos) = entry.iter().position(|i| i.id == *id) {
				let ongoing_request = entry.remove(pos).ongoing_request;
				if let Some(ongoing_request) = ongoing_request {
					log::debug!(
						target: "sync",
						"Connection {:?} with {} closed with ongoing sync request: {:?}",
						id,
						peer_id,
						ongoing_request
					);
					let ev = Event::RequestCancelled {
						peer: peer_id.clone(),
						original_request: ongoing_request.request.clone(),
						request_duration: ongoing_request.emitted.elapsed(),
					};
					self.pending_events.push_back(NetworkBehaviourAction::GenerateEvent(ev));
				}
				if entry.is_empty() {
					needs_remove = true;
				}
			} else {
				log::error!(
					target: "sync",
					"State inconsistency: connection id not found in list"
				);
			}
		} else {
			log::error!(
				target: "sync",
				"State inconsistency: peer_id not found in list of connections"
			);
		}
		if needs_remove {
			self.peers.remove(peer_id);
		}
	}

	fn inject_event(
		&mut self,
		peer: PeerId,
		connection_id: ConnectionId,
		node_event: NodeEvent<B, NegotiatedSubstream>
	) {
		match node_event {
			NodeEvent::Request(request, mut stream, handling_start) => {
				match self.on_block_request(&peer, &request) {
					Ok(res) => {
						log::trace!(
							target: "sync",
							"Enqueueing block response for peer {} with {} blocks",
							peer, res.blocks.len()
						);
						let mut data = Vec::with_capacity(res.encoded_len());
						if let Err(e) = res.encode(&mut data) {
							log::debug!(
								target: "sync",
								"Error encoding block response for peer {}: {}",
								peer, e
							)
						} else {
							self.outgoing.push(async move {
								if let Err(e) = write_one(&mut stream, data).await {
									log::debug!(
										target: "sync",
										"Error writing block response: {}",
										e
									);
								}
								(peer, handling_start.elapsed())
							}.boxed());
						}
					}
					Err(e) => log::debug!(
						target: "sync",
						"Error handling block request from peer {}: {}", peer, e
					)
				}
			}
			NodeEvent::Response(original_request, response) => {
				log::trace!(
					target: "sync",
					"Received block response from peer {} with {} blocks",
					peer, response.blocks.len()
				);
				let request_duration = if let Some(connections) = self.peers.get_mut(&peer) {
					if let Some(connection) = connections.iter_mut().find(|c| c.id == connection_id) {
						if let Some(ongoing_request) = &mut connection.ongoing_request {
							if ongoing_request.request == original_request {
								let request_duration = ongoing_request.emitted.elapsed();
								connection.ongoing_request = None;
								request_duration
							} else {
								// We're no longer interested in that request.
								log::debug!(
									target: "sync",
									"Received response from {} to obsolete block request {:?}",
									peer,
									original_request
								);
								return;
							}
						} else {
							// We remove from `self.peers` requests we're no longer interested in,
							// so this can legitimately happen.
							log::trace!(
								target: "sync",
								"Response discarded because it concerns an obsolete request"
							);
							return;
						}
					} else {
						log::error!(
							target: "sync",
							"State inconsistency: response on non-existing connection {:?}",
							connection_id
						);
						return;
					}
				} else {
					log::error!(
						target: "sync",
						"State inconsistency: response on non-connected peer {}",
						peer
					);
					return;
				};

				let blocks = response.blocks.into_iter().map(|block_data| {
					Ok(message::BlockData::<B> {
						hash: Decode::decode(&mut block_data.hash.as_ref())?,
						header: if !block_data.header.is_empty() {
							Some(Decode::decode(&mut block_data.header.as_ref())?)
						} else {
							None
						},
						body: if original_request.fields.contains(message::BlockAttributes::BODY) {
							Some(block_data.body.iter().map(|body| {
								Decode::decode(&mut body.as_ref())
							}).collect::<Result<Vec<_>, _>>()?)
						} else {
							None
						},
						receipt: if !block_data.message_queue.is_empty() {
							Some(block_data.receipt)
						} else {
							None
						},
						message_queue: if !block_data.message_queue.is_empty() {
							Some(block_data.message_queue)
						} else {
							None
						},
						justification: if !block_data.justification.is_empty() {
							Some(block_data.justification)
						} else if block_data.is_empty_justification {
							Some(Vec::new())
						} else {
							None
						},
					})
				}).collect::<Result<Vec<_>, codec::Error>>();

				match blocks {
					Ok(blocks) => {
						let id = original_request.id;
						let ev = Event::Response {
							peer,
							original_request,
							response: message::BlockResponse::<B> { id, blocks },
							request_duration,
						};
						self.pending_events.push_back(NetworkBehaviourAction::GenerateEvent(ev));
					}
					Err(err) => {
						log::debug!(
							target: "sync",
							"Failed to decode block response from peer {}: {}", peer, err
						);
					}
				}
			}
		}
	}

	fn poll(&mut self, cx: &mut Context, _: &mut impl PollParameters)
		-> Poll<NetworkBehaviourAction<OutboundProtocol<B>, Event<B>>>
	{
		if let Some(ev) = self.pending_events.pop_front() {
			return Poll::Ready(ev);
		}

		// Check the request timeouts.
		for (peer, connections) in &mut self.peers {
			for connection in connections {
				let ongoing_request = match &mut connection.ongoing_request {
					Some(rq) => rq,
					None => continue,
				};

				if let Poll::Ready(_) = Pin::new(&mut ongoing_request.timeout).poll(cx) {
					let original_request = ongoing_request.request.clone();
					let request_duration = ongoing_request.emitted.elapsed();
					connection.ongoing_request = None;
					log::debug!(
						target: "sync",
						"Request timeout for {}: {:?}",
						peer, original_request
					);
					let ev = Event::RequestTimeout {
						peer: peer.clone(),
						original_request,
						request_duration,
					};
					return Poll::Ready(NetworkBehaviourAction::GenerateEvent(ev));
				}
			}
		}

		if let Poll::Ready(Some((peer, total_handling_time))) = self.outgoing.poll_next_unpin(cx) {
			let ev = Event::AnsweredRequest {
				peer,
				total_handling_time,
			};
			return Poll::Ready(NetworkBehaviourAction::GenerateEvent(ev));
		}

		Poll::Pending
	}
}

/// Output type of inbound and outbound substream upgrades.
#[derive(Debug)]
pub enum NodeEvent<B: Block, T> {
	/// Incoming request from remote, substream to use for the response, and when we started
	/// handling this request.
	Request(schema::v1::BlockRequest, T, Instant),
	/// Incoming response from remote.
	Response(message::BlockRequest<B>, schema::v1::BlockResponse),
}

/// Substream upgrade protocol.
///
/// We attempt to parse an incoming protobuf encoded request (cf. `Request`)
/// which will be handled by the `BlockRequests` behaviour, i.e. the request
/// will become visible via `inject_node_event` which then dispatches to the
/// relevant callback to process the message and prepare a response.
#[derive(Debug, Clone)]
pub struct InboundProtocol<B> {
	/// The max. request length in bytes.
	max_request_len: usize,
	/// The protocol to use during upgrade negotiation.
	protocol: Bytes,
	/// Type of the block.
	marker: PhantomData<B>,
}

impl<B: Block> UpgradeInfo for InboundProtocol<B> {
	type Info = Bytes;
	type InfoIter = iter::Once<Self::Info>;

	fn protocol_info(&self) -> Self::InfoIter {
		iter::once(self.protocol.clone())
	}
}

impl<B, T> InboundUpgrade<T> for InboundProtocol<B>
where
	B: Block,
	T: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
	type Output = NodeEvent<B, T>;
	type Error = ReadOneError;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_inbound(self, mut s: T, _: Self::Info) -> Self::Future {
		// This `Instant` will be passed around until the processing of this request is done.
		let handling_start = Instant::now();

		let future = async move {
			let len = self.max_request_len;
			let vec = read_one(&mut s, len).await?;
			match schema::v1::BlockRequest::decode(&vec[..]) {
				Ok(r) => Ok(NodeEvent::Request(r, s, handling_start)),
				Err(e) => Err(ReadOneError::Io(io::Error::new(io::ErrorKind::Other, e)))
			}
		};
		future.boxed()
	}
}

/// Substream upgrade protocol.
///
/// Sends a request to remote and awaits the response.
#[derive(Debug, Clone)]
pub struct OutboundProtocol<B: Block> {
	/// The serialized protobuf request.
	request: Vec<u8>,
	/// The original request. Passed back through the API when the response comes back.
	original_request: message::BlockRequest<B>,
	/// The max. response length in bytes.
	max_response_size: usize,
	/// The protocol to use for upgrade negotiation.
	protocol: Bytes,
}

impl<B: Block> UpgradeInfo for OutboundProtocol<B> {
	type Info = Bytes;
	type InfoIter = iter::Once<Self::Info>;

	fn protocol_info(&self) -> Self::InfoIter {
		iter::once(self.protocol.clone())
	}
}

impl<B, T> OutboundUpgrade<T> for OutboundProtocol<B>
where
	B: Block,
	T: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
	type Output = NodeEvent<B, T>;
	type Error = ReadOneError;
	type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

	fn upgrade_outbound(self, mut s: T, _: Self::Info) -> Self::Future {
		async move {
			write_one(&mut s, &self.request).await?;
			let vec = read_one(&mut s, self.max_response_size).await?;

			schema::v1::BlockResponse::decode(&vec[..])
				.map(|r| NodeEvent::Response(self.original_request, r))
				.map_err(|e| {
					ReadOneError::Io(io::Error::new(io::ErrorKind::Other, e))
				})
		}.boxed()
	}
}

/// Build protobuf block request message.
pub(crate) fn build_protobuf_block_request<Hash: Encode, Number: Encode>(
	attributes: BlockAttributes,
	from_block: message::FromBlock<Hash, Number>,
	to_block: Option<Hash>,
	direction: message::Direction,
	max_blocks: Option<u32>,
) -> schema::v1::BlockRequest {
	schema::v1::BlockRequest {
		fields: attributes.to_be_u32(),
		from_block: match from_block {
			message::FromBlock::Hash(h) =>
				Some(schema::v1::block_request::FromBlock::Hash(h.encode())),
			message::FromBlock::Number(n) =>
				Some(schema::v1::block_request::FromBlock::Number(n.encode())),
		},
		to_block: to_block.map(|h| h.encode()).unwrap_or_default(),
		direction: match direction {
			message::Direction::Ascending => schema::v1::Direction::Ascending as i32,
			message::Direction::Descending => schema::v1::Direction::Descending as i32,
		},
		max_blocks: max_blocks.unwrap_or(0),
	}
}
