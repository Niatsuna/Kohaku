import json
import logging
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


def read_secret(path: str) -> str | None:
    """Reads single line secret file"""
    file_path = Path(path)
    if not file_path.exists:
        logger.error(f"File '{file_path} not found")
        return None

    try:
        data = file_path.read_text().strip()
        if not data:
            return None
        return data
    except Exception as e:
        logger.error(f"Failed to read secret file: {e}")
        return None


def load_file(path: str) -> Any | None:
    """Loads file from given path"""
    file_path = Path(path)
    if not file_path.exists:
        logger.error(f"File '{file_path} not found")
        return None

    try:
        with open(file_path) as f:
            return json.load(f)
    except Exception as e:
        logger.error(f"Failed to read file: {e}")
        return None


def store_file(path: str, data: list | dict):
    file_path = Path(path)
    try:
        with open(file_path, "w") as f:
            json.dump(data, f, indent=2)
        logger.info(f"Stored data in '{path}'")
    except Exception as e:
        logger.error(f"Failed to store data in '{path}' : {e}")
