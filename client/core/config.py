import logging
import os
from dataclasses import dataclass

from dotenv import load_dotenv

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)

load_dotenv()


@dataclass(frozen=True)
class Config:
    """Kohaku Configuration"""

    token: str
    prefix: str
    server_addr: str
    server_port: str
    secret: str
    repo: str

    logging_level: str = "INFO"
    color_default: int = 0x1B6C8E
    color_error: int = 0x8E1B1B

    def __post_init__(self):
        required = {
            "CLIENT_TOKEN": self.token,
            "CLIENT_PREFIX": self.prefix,
            "SERVER_ADDR": self.server_addr,
            "SERVER_PORT": self.server_port,
            "SECRET": self.secret,
        }

        for name, val in required.items():
            if not val:
                raise ValueError(f"{name} must be set!")

        valid_levels = ["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]
        if self.logging_level.upper() not in valid_levels:
            raise ValueError(
                f"Invalid CLIENT_LOGGING_LEVEL! "
                f"CLIENT_LOGGING_LEVEL must be one of these values: {valid_levels}"
            )

    @classmethod
    def load(cls) -> "Config":
        """Load configuration from environment variables"""
        try:
            config = cls(
                token=os.getenv("CLIENT_TOKEN", ""),
                prefix=os.getenv("CLIENT_PREFIX", ""),
                server_addr=os.getenv("SERVER_ADDR", ""),
                server_port=os.getenv("SERVER_PORT", ""),
                secret=os.getenv("SECRET", ""),
                logging_level=os.getenv("CLIENT_LOGGING_LEVEL", "INFO"),
                repo=os.getenv("CLIENT_REPO_URL"),
            )
            logger.info("Configuration loaded successfully")
            return config
        except ValueError as e:
            logger.error(f"Configuration error: {e}")
            raise


config: Config | None = None


def get_config() -> Config:
    """Get the global config instance"""
    global config
    if config is None:
        config = Config.load()
    return config
