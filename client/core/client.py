import asyncio
import logging
from pathlib import Path

from disnake.ext import commands

from core.comm import WebSocketClient
from core.config import Config

logger = logging.getLogger(__name__)


class Client(commands.Bot):
    """Custom Kohaku client class"""

    def __init__(self, config: Config, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.config = config
        self.websocket = None

    async def load_features(self):
        """Loads features like the commands and the websocket"""
        # Cogs ( = Commands)
        logger.info("Loading cogs...")

        cogs_dir = Path(__file__).parent.parent / "cogs"
        cog_files = [f.stem for f in cogs_dir.glob("*.py") if f.stem != "__init__"]

        for cog_name in cog_files:
            try:
                self.load_extension(f"cogs.{cog_name}")
            except Exception as e:
                logger.error(f"Failed to load cogs.{cog_name}: {e}", exc_info=True)

        logger.info(f"Loaded {len(self.extensions)} cogs")

        # Websocket ( = Communication to backend)
        uri = f"ws://{self.config.server_addr}:{self.config.server_port}/ws"
        self.websocket = WebSocketClient(uri, secret=self.config.secret)

    async def on_ready(self):
        await self.load_features()
        await asyncio.create_task(self.websocket.run())  # Starts websocket

        logger.info(f"Kohaku is ready! Logged in as {self.user}")
        logger.info(f"Connected to {len(self.guilds)} guilds")

    async def close(self):
        logger.info("Shutting down bot...")
        await self.websocket.stop()
        await super().close()
