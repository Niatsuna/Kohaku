import asyncio
import logging
from pathlib import Path

import websockets
from websockets.asyncio.client import ClientConnection, connect

from core.config import get_config

logger = logging.getLogger(__name__)


class WsClient:
    def __init__(self, url: str):
        self.url: str = url
        self.api_key: str | None = self.load_api_key()
        self.websocket: ClientConnection | None = None
        self.running: bool = False
        self.heartbeat_timeout: int = 90

    def load_api_key(self) -> str | None:
        """Load API key from .secret file"""
        secret_path = Path(".secret")
        if not secret_path.exists():
            logger.error(f"Secret file '{secret_path}' not found")
            return None

        try:
            api_key = secret_path.read_text().strip()
            if not api_key:
                logger.error(f"Secret file '{secret_path}' is empty")
                return None
            logger.info("API key loaded successfully")
            return api_key
        except Exception as e:
            logger.error(f"Failed to read secret file: {e}")
            return None

    async def connect(self) -> bool:
        """Establish WebSocket connection with API key in header"""
        if self.api_key is not None:
            headers = {"X-API-Key": self.api_key}
            try:
                self.websocket = await connect(self.url, additional_headers=headers)
                self.running = True
                logger.info(f"Connected to {self.url}")
                return True
            except Exception as e:
                logger.error(f"Failed to connect: {e}")
                return False
        return False

    async def receive_task(self):
        """Handle incoming messages from server"""
        try:
            while self.running and self.websocket:
                message = await self.websocket.recv()
                if isinstance(message, str):
                    logger.info("Received event message from server")
                    await self.handle_server_message(message)
        except websockets.exceptions.ConnectionClosed:
            logger.info("Connection closed by server")
            self.running = False
        except Exception as e:
            logger.error(f"Error in receive task: {e}")

    async def heartbeat_task(self):
        """Monitor server activity and close if no response"""
        last_activity = asyncio.get_event_loop().time()

        while self.running and self.websocket:
            await asyncio.sleep(30)
            current_time = asyncio.get_event_loop().time()

            if current_time - last_activity > self.heartbeat_timeout:
                logger.warning("No server activity detected, closing connection")
                self.running = False
                if self.websocket:
                    await self.websocket.close()
                break

            if self.websocket and not self.websocket.closed:
                last_activity = current_time

    async def run(self):
        """Run all tasks concurrently"""
        if not await self.connect():
            return

        try:
            # Run receive and heartbeat tasks concurrently
            # Ping/Pong get automatically handled by the websockets library
            await asyncio.gather(self.receive_task(), self.heartbeat_task(), return_exceptions=True)
        finally:
            if self.websocket:
                await self.websocket.close()
            logger.info("WebSocket client shut down")

    async def handle_server_message(self, message: str):
        """Process incoming server events"""
        # TODO: Implement
        logger.info(f"Process message: {message}")


wsclient: WsClient | None = None


def get_wsclient():
    """Get the global WsClient"""
    global wsclient
    if wsclient is None:
        config = get_config()
        wsclient = WsClient(config.server_ws_url)
    return wsclient
