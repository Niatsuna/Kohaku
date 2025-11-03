import logging
import sys

from disnake import Intents

from core.client import Client
from core.config import get_config
from utils.help import KohakuHelpCommand


def setup_logging(log_level: str):
    """Configure project-wide logging"""
    root_logger = logging.getLogger()
    root_logger.setLevel(log_level)

    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setLevel(log_level)

    formatter = logging.Formatter(
        "%(asctime)s - %(name)s - %(levelname)s - %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )
    console_handler.setFormatter(formatter)

    root_logger.handlers.clear()
    root_logger.addHandler(console_handler)


def main():
    config = get_config()

    setup_logging(config.logging_level)

    logger = logging.getLogger(__name__)
    logger.info("Starting Kohaku Client ...")

    intents = Intents.default()
    intents.message_content = True

    client = Client(
        config, command_prefix=config.prefix, intents=intents, help_command=KohakuHelpCommand()
    )

    client.run(config.token)


if __name__ == "__main__":
    main()
