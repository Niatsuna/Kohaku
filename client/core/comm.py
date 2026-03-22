import asyncio
import json
import logging
from collections.abc import Callable

import requests
import websockets
from websockets.asyncio.client import ClientConnection, connect

from core.config import get_config
from utils.files import load_file, read_secret, store_file

logger = logging.getLogger(__name__)

API_AUTH_LOGIN = "/auth/login"
API_AUTH_REFRESH = "/auth/manage/refresh"
FILE_APIKEY = ".secret"
FILE_TOKENS = ".session"


class CommunicationHandler:
    def __init__(self, api_url: str, ws_url: str):
        self.url: str = api_url
        self.ws_url: str = ws_url

        self.api_key: str | None = read_secret(FILE_APIKEY)
        self.access_token: str | None = None
        self.refresh_token: str | None = None

        self.running: bool = False
        self.heartbeat_timeout: int = 90
        self.websocket: ClientConnection | None = None

        self.topic_registry: dict = {}
        self.load_tokens()

    def load_tokens(self) -> bool:
        """
        Loads access and refresh token from file.

        Returns boolean indicating if loading and storing resulted in useable tokens.
        """
        session = load_file(FILE_TOKENS)

        if session is not None and "access_token" in session and "refresh_token" in session:
            self.access_token = session["access_token"]
            self.refresh_token = session["refresh_token"]
            if self.access_token is not None and self.refresh_token is not None:
                return True
            logger.error("Failed to load jwt tokens: Invalid stored values (None)")
        elif session is None:
            logger.error(f"Failed to load '{FILE_TOKENS}': File does not exist")
        else:
            logger.error("Failed to load jwt tokens: Invalid file format found!")
        self.access_token = None
        self.refresh_token = None
        return False

    def login(self) -> bool:
        """
        Authenticates the client using the API Key and stores the returned tokens.

        Returns boolean indicating if login was successful and useable tokens were returned.
        """
        url = self.url + API_AUTH_LOGIN
        headers = {"X-API-Key": self.api_key}

        try:
            response = requests.post(url, headers=headers, timeout=3)
            data = response.json()

            if response.status_code == 200:
                store_file(FILE_TOKENS, data)
                return self.load_tokens()
        except Exception:
            data = {"status": 500, "kind": "unknown", "message": "unknown"}
        logger.error(f"Login failed ({data["status"]}) - {data["kind"]} : {data["message"]}")
        return False

    def refresh(self) -> bool:
        """
        Refreshes the jwt tokens and stores them.

        Returns boolean indicating if refresh was successful and useable tokens were returned.
        """
        url = self.url + API_AUTH_REFRESH
        headers = {"Authorization": f"Bearer {self.refresh_token}"}

        try:
            response = requests.post(url, headers=headers, timeout=3)
            data = response.json()

            if response.status_code == 200:
                store_file(FILE_TOKENS, data)
                return self.load_tokens()
        except Exception:
            data = {
                "status": 504,
                "kind": "Bad Gateway",
                "message": "The server is currently unavailable or returns a invalid json response.",
            }
        logger.error(f"Refresh failed ({data["status"]}) - {data["kind"]} : {data["message"]}")
        return False

    def __request(self, url: str, token: str) -> dict | int:
        """
        Request a given endpoint with a given token.
        Is being used in `request(...)`.

        Will return json body of response or the status code if an error occured.
        """
        headers = {"Authorization": f"Bearer {token}"}

        try:
            status = 504
            response = requests.get(url, headers=headers, timeout=5)
            data = response.json()
            status = data["status"]

            if response.status_code == 200:
                return response.json()
        except Exception:
            (kind, message) = (
                (
                    "Bad Gateway",
                    "The server is currently unavailable or returns a invalid json response.",
                )
                if status == 504
                else (data["kind"], data["message"])
            )
            data = {"status": status, "kind": kind, "message": message}
        logger.error(f"Request failed ({data["status"]}) : {data["kind"]} : {data["message"]}")
        return None

    def request(self, endpoint: str, secure_mode: bool = False, attempt: int = 0) -> dict | None:
        """
        Requests a resource at a given endpoint.

        If secure mode is activated, use the stored access token.
        Try to refresh or login if failing in secure mode.

        Returns responses json body or None if an error occured
        """
        url = self.url + endpoint

        # Normal Mode =============================
        if not secure_mode:
            response = requests.get(url)
            if response.status_code == 200:
                return response.json()
            return None

        # Secure Mode =============================
        if self.access_token is None or attempt > 1:
            if not self.login():
                logger.error("[Request] Unable to login. Aborting request attempt!")
                return None
        elif attempt == 1 and not self.refresh():
            logger.error("[Request] Unable to refresh jwt tokens. Trying again after login ...")
            return self.request(endpoint, secure_mode, attempt=2)

        # Attempt request
        if self.access_token is not None:
            response = self.__request(url, self.access_token)
            if isinstance(response, int):
                if attempt > 1:
                    logger.error("[Request] Failed after re-log. Aborting request attempt")
                    return None
                return self.request(endpoint, secure_mode, attempt=attempt + 1)
            return response
        logger.error("[Request] No available access token. Trying again after login ...")
        return self.request(endpoint, secure_mode, attempt=2)

    # Websocket Connection :
    async def connect(self, attempt: int = 0) -> bool:
        """Establish Websocket connection with JWT Token in header"""
        if self.access_token is None or attempt > 1:
            # No tokens / Refreshing failed -> Login
            if self.api_key is None:
                logger.error("[WS] No credentials available. Aborting connection attempt!")
                return False

            if not self.login():
                logger.error("[WS] Login failed. Aborting connection attempt!")
                return False

        elif attempt == 1:
            # Access token failed in previous attempt -> Refresh
            if not self.refresh():
                logger.error("[WS] Refreshing failed. Aborting connection attempt!")
                return False

        # Attempt connection
        if self.access_token is not None:
            headers = {"Authorization": f"Bearer {self.access_token}"}
            try:
                self.websocket = await connect(self.ws_url, additional_headers=headers)
                self.running = True
                logger.info(f"[WS] Connected to {self.ws_url}")
                return True
            except Exception as e:
                logger.error(f"[WS] Failed to connect {e}")
                if attempt == 0:
                    logger.info("[WS] Try again after refreshing the tokens ...")
                    return await self.connect(attempt=1)
                if attempt == 1:
                    logger.info("[WS] Try again after relogging into backend service ...")
                    return await self.connect(attempt=2)
                logger.info("[WS] Aborting connection attempt!")
        else:
            logger.error("[WS] Invalid token format - Tokens must meet JWT standard!")
        return False

    async def disconnect(self) -> bool:
        """Disconnect current Websocket connection"""
        if self.running or self.websocket:
            try:
                self.running = False
                if self.websocket:
                    await self.websocket.close(1000, "Requested Disconnect")
                    self.websocket = None
                logger.info(f"[WS] Disconnected from {self.ws_url}")
                return True
            except Exception as e:
                logger.error(f"[WS] Failed to disconnect: {e}")
        return False

    async def receive_task(self):
        """Handle incoming messages from server"""
        try:
            while self.running and self.websocket:
                message = await self.websocket.recv()

                data: dict = json.loads(message)
                topic = data["topic"]
                if topic in self.topic_registry:
                    await self.topic_registry[topic](data)
                else:
                    logger.info(
                        f"[Event] Received data for topic '{topic}', but no event handler can be found for this topic."
                    )
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
                await self.disconnect()
                break

            if self.websocket is not None:
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
            await self.disconnect()
            logger.info("WebSocket client shut down")

    # Event Handler Registry :
    def register(self, topic: str, event_handler: Callable[[dict], None]):
        """Registers a function as an event handler for a topic"""
        self.topic_registry[topic] = event_handler

    def unregister(self, topic: str):
        """Unregistes a function as an event handler for a topic"""
        del self.topic_registry[topic]


handler: CommunicationHandler | None = None


def get_comm_handler() -> CommunicationHandler:
    """Get the global authentication handler for requesting anything from the backend"""
    global handler
    if handler is None:
        config = get_config()
        handler = CommunicationHandler(config.server_api_url, config.server_ws_url)
    return handler
