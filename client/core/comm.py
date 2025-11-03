import asyncio
import hashlib
import hmac
import json
import logging
import time
import uuid
from enum import Enum
from typing import Any

import websockets
from websockets.client import WebSocketClientProtocol

logger = logging.getLogger(__name__)


class MessageType(str, Enum):
    AUTHENTIFICATION = "auth"
    PING = "ping"
    PONG = "pong"
    NOTIFICATION = "notification"


class WsMessage:
    def __init__(self, timestamp: int, message_id: str, message: dict[str, Any]):
        self.timestamp = timestamp
        self.message_id = message_id
        self.message = message

    def to_dict(self) -> dict[str, Any]:
        return {"timestamp": self.timestamp, "message_id": self.message_id, "message": self.message}


class WebSocketClient:
    def __init__(self, uri: str, secret: str):
        self.uri = uri
        self.secret = secret
        self.ws: WebSocketClientProtocol | None = None
        self.running = False
        self.connected = False
        self.reconnect_delay = 5
        self.max_reconnect_delay = 30
        self.pending_responses: dict[str, asyncio.Future] = {}

    def sign_message(self, message: WsMessage) -> str:
        """Sign a message with HMAC-SHA256"""
        payload = json.dumps(message.to_dict())

        signature = hmac.new(self.secret.encode(), payload.encode(), hashlib.sha256).hexdigest()

        return f"{payload}.{signature}"

    def verify_message(self, data: str) -> WsMessage | None:
        """Verify and parse an incoming message"""
        parts = data.split(".")
        if len(parts) != 2:
            logger.warning("Invalid message format")
            return None

        payload, signature = parts

        # Verify signature
        expected_sig = hmac.new(self.secret.encode(), payload.encode(), hashlib.sha256).hexdigest()

        if expected_sig != signature:
            logger.warning("Invalid signature")
            return None

        # Parse payload
        try:
            frame_dict = json.loads(payload)
            frame = WsMessage(
                message=frame_dict["message"],
                timestamp=frame_dict["timestamp"],
                message_id=frame_dict["message_id"],
            )
        except (json.JSONDecodeError, KeyError) as e:
            logger.warning(f"Failed to parse message: {e}")
            return None

        # Check timestamp (reject messages older than 30 seconds)
        now = int(time.time())
        if abs(now - frame.timestamp) > 30:
            logger.warning("Message expired")
            return None

        return frame

    async def send_message(self, message: dict[str, Any]) -> None:
        """Send a signed message to the server"""
        if not self.ws:
            raise RuntimeError("WebSocket not connected")

        frame = WsMessage(message=message, timestamp=int(time.time()), message_id=str(uuid.uuid4()))

        signed = self.sign_message(frame)
        await self.ws.send(signed)

    async def handle_message(self, frame: WsMessage):
        """Handle incoming messages based on type"""
        msg = frame.message
        msg_type = msg.get("type")

        if msg_type == MessageType.PING:
            # Respond to ping
            pong = {"type": MessageType.PONG, "id": msg.get("id")}
            await self.send_message(pong)
            logger.info(f"Responded to ping: {msg.get('id')}")

        elif msg_type == MessageType.NOTIFICATION:
            # Handle notification from server
            data = msg.get("data")
            logger.info(f"Received notification: {data}")

        elif msg_type == MessageType.PONG:
            data = msg.get("id")
            logger.info(f"Received pong: {data}")

        else:
            logger.info(f"Unknown message type: {msg_type}")

    async def listen(self):
        """Listen for incoming messages"""
        try:
            async for message in self.ws:
                frame = self.verify_message(message)
                if frame:
                    await self.handle_message(frame)
                else:
                    logger.info("Received invalid message")
        except websockets.exceptions.ConnectionClosed:
            logger.info("WebSocket connection closed")
        except Exception as e:
            logger.info(f"Error in listen loop: {e}")

    async def run(self):
        """Main run loop with auto-reconnect"""
        self.running = True

        while self.running:
            try:
                await self.connect()
                await self.listen()
            except Exception as e:
                logger.info(f"Connection error: {e}")

            if self.running:
                self.connected = False
                logger.info(f"Reconnecting in {self.reconnect_delay} seconds...")
                await asyncio.sleep(self.reconnect_delay)

                # Exponential backoff
                if self.reconnect_delay < self.max_reconnect_delay:
                    self.reconnect_delay = min(self.reconnect_delay * 2, self.max_reconnect_delay)
                else:
                    # If connection could not be established by now, discard the websocket connection completely
                    logger.error("Couldn't establish connection! Discarding attempt completely!")
                    await self.stop()

    async def connect(self):
        """Connect to WebSocket with authentication"""
        try:
            self.ws = await websockets.connect(self.uri, ping_interval=60, ping_timeout=10)
            logger.info("WebSocket connected")
            self.connected = True
            self.reconnect_delay = 5  # Reset reconnect delay on success

            # Send auth message
            await self.send_message({"type": "auth"})
        except Exception as e:
            logger.info(f"Connection failed: {e}")
            raise

    async def stop(self):
        """Stop the client and close connection"""
        self.running = False
        self.connected = False
        if self.ws:
            await self.ws.close()
