import asyncio
import logging
from pathlib import Path

from disnake import Embed
from disnake.ext import commands

from core.comm import get_comm_handler
from core.config import Config

logger = logging.getLogger(__name__)


class Client(commands.Bot):
    """Custom Kohaku client class"""

    def __init__(self, config: Config, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.config = config

    async def load_features(self):
        """Loads features like the commands and the websocket"""
        # Communication Handler (Init)
        comm = get_comm_handler()
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
        asyncio.create_task(comm.run())

    async def on_ready(self):
        await self.load_features()

        logger.info(f"Kohaku is ready! Logged in as {self.user}")
        logger.info(f"Connected to {len(self.guilds)} guilds")

    async def on_command_error(self, ctx: commands.Context, error: commands.CommandError):
        if isinstance(error, commands.CommandNotFound):
            return

        embed = Embed(color=self.config.color_error)
        if isinstance(error, commands.MissingRequiredArgument):
            embed.description = f"❌ Missing argument! Please refer to `{self.config.prefix}help {ctx.command}` for more information"
        elif isinstance(error, commands.CommandOnCooldown):
            embed.description = "❌ Command is currently on cooldown. Please try again later!"
        elif isinstance(error, commands.CheckFailure):
            embed.description = "❌ Permission denied!"
        else:
            logger.error(f"Unhandled error in '{ctx.command}'", exc_info=error)
            embed.description = f"❌ Unhandled error in '{ctx.command}'. Please inform an admin!"
        await ctx.send(embed=embed)

    async def close(self):
        logger.info("Shutting down bot...")

        comm = get_comm_handler()
        await comm.disconnect()

        await super().close()
